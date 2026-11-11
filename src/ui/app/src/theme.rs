//! Theme resolution and the platform style recipes (openspec: gpui-kit-ui-rewrite,
//! task 4.1; REQ-UI-05).
//!
//! The colour values live as plain data in `snackpilot_model::tokens` (04 §8);
//! this layer parses them into gpui colours, resolves the `System`/`Hell`/
//! `Dunkel` preference against the OS appearance, and exposes the §7 recipes
//! with the iOS-glass versus flat-Android split (v1 `useFlatStyle`).

use gpui::{hsla, InteractiveElement};
use gpui_kit::{point, px, BoxShadow, Div, Hsla, Pixels, Rgba, StyleRefinement, Styled};
use snackpilot_model::desktop::{self, ControlState, Rgb, FOCUS_GAP, FOCUS_WIDTH};
use snackpilot_model::tokens;
use snackpilot_model::{AccentColor, ThemePreference};

/// Convert a model-side `Rgb` (from the desktop token helpers) into a gpui
/// colour. Desktop fills are derived in the gpui-free model so the contrast
/// tests can check them; this is the only place they become paint.
pub fn from_rgb(c: Rgb) -> Hsla {
    Rgba {
        r: c.r as f32 / 255.0,
        g: c.g as f32 / 255.0,
        b: c.b as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

/// Focus recipe hook shared by the desktop compositions: a 2 px ring in the
/// resolved focus ink on the target's own shape. Applied with
/// `.focus(theme::desktop_focus(ink))`, so the rest style — and every legacy
/// mobile value — stays untouched.
pub fn desktop_focus(ink: Hsla) -> impl FnOnce(StyleRefinement) -> StyleRefinement {
    move |s| s.border(px(FOCUS_WIDTH)).border_color(ink)
}

/// Reserve the brief's 4 px focus room (`desktop.focusGap` 2 px + the 2 px
/// ring) around a desktop control. The frame is identical in every state —
/// only the ring colour changes — so focusing never shifts layout and the
/// ring is never clipped by a neighbouring edge.
pub fn focus_frame(div: Div, state: ControlState, radius: f32, theme: &AppTheme) -> Div {
    let ring = if state.ring_visible() {
        theme.desktop.focus
    } else {
        hsla(0., 0., 0., 0.)
    };
    div.p(px(FOCUS_GAP))
        .rounded(px(radius + FOCUS_GAP + FOCUS_WIDTH))
        .border(px(FOCUS_WIDTH))
        .border_color(ring)
        .focus(desktop_focus(theme.desktop.focus))
}

/// One filled desktop action and its states (brief "Action states"): rest,
/// hover (8 % ink mix), pressed (14 % ink mix) and the ink on top, chosen
/// per fill for the better contrast (REQ-UI-13).
#[derive(Debug, Clone, Copy)]
pub struct DesktopAction {
    pub fill: Hsla,
    pub hover: Hsla,
    pub pressed: Hsla,
    pub ink: Hsla,
}

/// The desktop extension palette (desktop-ui-redesign D4): derived purely
/// from the pinned §8 tokens via `snackpilot_model::desktop`, so the mobile
/// recipes above keep their exact values and the desktop shell adds no new
/// colour literals.
#[derive(Debug, Clone)]
pub struct DesktopPalette {
    pub action: DesktopAction,
    pub danger: DesktopAction,
    /// Workspace panel fill (`surface`).
    pub panel: Hsla,
    /// Navigation rail fill (`background`).
    pub rail: Hsla,
    /// Focus ring ink (`text_primary`, >=3:1 on panel and rail).
    pub focus: Hsla,
    /// Primary text ink (`text_primary`).
    pub ink: Hsla,
    /// Accent used as text, resolved against the panel fill: the accent itself
    /// only when it clears 4.5:1, otherwise `text_primary` (REQ-UI-13).
    pub accent_on_panel: Hsla,
    /// Accent used as text, resolved against the rail fill (REQ-UI-13).
    pub accent_on_rail: Hsla,
    /// Accent used as text on the selection fill (the selected desktop chip):
    /// it sits on `primary_surface`, not the panel, so it resolves there.
    pub accent_on_selection: Hsla,
    /// Accent used as a selected-card edge, resolved against the selection
    /// fill: the accent only when it clears 3:1 there, else `text_primary`.
    pub accent_boundary: Hsla,
}

/// Parse a §8 token value (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` or
/// `rgba(r,g,b,a)`) into a gpui colour. Unparsable input is a token-table
/// bug, so it panics — the token tests pin the table.
pub fn color(value: &str) -> Hsla {
    let value = value.trim();
    let (r, g, b, a) = if let Some(hex) = value.strip_prefix('#') {
        let v =
            u32::from_str_radix(hex, 16).unwrap_or_else(|_| panic!("invalid hex token: {value}"));
        match hex.len() {
            3 => {
                let (r, g, b) = ((v >> 8) & 0xF, (v >> 4) & 0xF, v & 0xF);
                (r * 17, g * 17, b * 17, 0xFF)
            }
            4 => {
                let (r, g, b, a) = ((v >> 12) & 0xF, (v >> 8) & 0xF, (v >> 4) & 0xF, v & 0xF);
                (r * 17, g * 17, b * 17, a * 17)
            }
            6 => ((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF, 0xFF),
            8 => (
                (v >> 24) & 0xFF,
                (v >> 16) & 0xFF,
                (v >> 8) & 0xFF,
                v & 0xFF,
            ),
            _ => panic!("hex token must have 3, 4, 6 or 8 digits: {value}"),
        }
    } else if let Some(inner) = value
        .strip_prefix("rgba(")
        .and_then(|v| v.strip_suffix(')'))
    {
        let n: Vec<f32> = inner
            .split(',')
            .map(|p| {
                p.trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("invalid rgba token: {value}"))
            })
            .collect();
        // Spec rgba() tokens: channels are 0-255, alpha is 0-1 (04 §8).
        (
            n[0].round() as u32,
            n[1].round() as u32,
            n[2].round() as u32,
            (n[3] * 255.0).round() as u32,
        )
    } else {
        panic!("token must be #hex or rgba(): {value}");
    };
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    }
    .into()
}

/// The §7 recipe for one surface: background, corner radius, hairline/1 px
/// border and soft shadow, already resolved for the current dialect.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub bg: Hsla,
    pub radius: Pixels,
    /// `(width, colour)`; `None` means no border.
    pub border: Option<(Pixels, Hsla)>,
    pub shadow: Option<BoxShadow>,
}

impl Recipe {
    /// iOS glass dialect: translucent fill, 0.5 px hairline, soft shadow.
    fn glass(bg: Hsla, radius: f32, edge: Hsla, shadow: (f32, f32, f32)) -> Self {
        Self {
            bg,
            radius: px(radius),
            border: Some((px(0.5), edge)),
            shadow: Some(soft_shadow(0., shadow.0, shadow.1, shadow.2)),
        }
    }
    /// Android flat dialect: opaque fill, 1 px border, elevation shadow.
    fn flat(bg: Hsla, radius: f32, border: Hsla, elevation: u32) -> Self {
        Self {
            bg,
            radius: px(radius),
            border: Some((px(1.), border)),
            shadow: (elevation > 0).then(|| elevation_shadow(elevation)),
        }
    }
    /// Override the border (04 §7 `buttonSecondary` iOS wants a 1 px
    /// `border`, not the usual 0.5 px glass hairline).
    pub fn with_border(mut self, width: Pixels, color: Hsla) -> Self {
        self.border = Some((width, color));
        self
    }

    /// A recipe with the border dropped (v1 `buttonPrimary`/`buttonDanger`).
    pub fn borderless(mut self) -> Self {
        self.border = None;
        self
    }
    /// Replace the shadow with an accent-coloured glow:
    /// `shadow(c, o, r, (x, y))` from the §7 table.
    pub fn tinted_shadow(
        mut self,
        color: Hsla,
        opacity: f32,
        radius: f32,
        offset: (f32, f32),
    ) -> Self {
        self.shadow = Some(BoxShadow {
            color: color.alpha(opacity),
            offset: point(px(offset.0), px(offset.1)),
            blur_radius: px(radius),
            spread_radius: px(0.),
            inset: false,
        });
        self
    }
}

fn soft_shadow(_: f32, opacity: f32, radius: f32, y: f32) -> BoxShadow {
    BoxShadow {
        color: color("#000000").alpha(opacity),
        offset: point(px(0.), px(y)),
        blur_radius: px(radius),
        spread_radius: px(0.),
        inset: false,
    }
}

/// Android `elevation: N` (04 §7) approximated as a black drop shadow,
/// scaled like Material's key light.
fn elevation_shadow(elevation: u32) -> BoxShadow {
    let e = elevation as f32;
    BoxShadow {
        color: color("#000000").alpha(0.06 + e * 0.02),
        offset: point(px(0.), px(e * 0.75)),
        blur_radius: px(e * 1.5),
        spread_radius: px(0.),
        inset: false,
    }
}

/// Every §8 token resolved to gpui colours for one appearance and dialect.
#[derive(Debug, Clone)]
pub struct AppTheme {
    pub dark: bool,
    /// `useFlatStyle`: true on Android and desktop, false on iOS (04 §7).
    pub flat: bool,

    pub background: Hsla,
    pub surface: Hsla,
    pub surface_variant: Hsla,
    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_tertiary: Hsla,
    /// Text on filled primary/danger buttons and the FAB (04 §7: white in
    /// both appearances).
    pub on_primary: Hsla,
    pub primary: Hsla,
    pub primary_dark: Hsla,
    pub primary_surface: Hsla,
    pub glass_primary: Hsla,
    pub border: Hsla,
    pub border_input: Hsla,
    pub success: Hsla,
    pub success_surface: Hsla,
    pub success_text: Hsla,
    pub success_border: Hsla,
    pub warning: Hsla,
    pub warning_surface: Hsla,
    pub warning_text: Hsla,
    pub warning_border: Hsla,
    pub error: Hsla,
    pub error_surface: Hsla,
    pub error_text: Hsla,
    pub overlay: Hsla,
    pub glass_surface: Hsla,
    pub glass_surface_variant: Hsla,
    pub glass_highlight: Hsla,
    pub glass_shadow_edge: Hsla,
    pub glass_success: Hsla,
    pub glass_warning: Hsla,
    pub glass_error: Hsla,
    /// Desktop extension palette, derived from the pinned tokens above; the
    /// mobile screens never read it (desktop-ui-redesign D4, REQ-UI-13).
    pub desktop: DesktopPalette,
}

/// `useFlatStyle` for the platform this binary was built for: Android and
/// desktop are flat, iOS is glass (04 §7; the desktop recipes are dropped, so
/// desktop shares the flat dialect).
pub fn use_flat_style() -> bool {
    !cfg!(target_os = "ios")
}

/// Hover/press shade for the kit button variants: gpui's `Hsla` has no
/// `darken`, so drop the lightness channel by `amount` (0..1).
pub fn darken(c: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (c.l - amount).clamp(0.0, 1.0),
        ..c
    }
}

impl AppTheme {
    /// Resolve a preference against the OS appearance (03-features/themes):
    /// `System` follows `os_dark`, `Hell`/`Dunkel` force their scheme.
    pub fn resolve(preference: ThemePreference, accent: AccentColor, os_dark: bool) -> Self {
        let dark = match preference {
            ThemePreference::System => os_dark,
            ThemePreference::Light => false,
            ThemePreference::Dark => true,
        };
        Self::build(dark, accent, use_flat_style())
    }

    /// Build one appearance directly (the gallery shows light and dark side
    /// by side regardless of the OS).
    pub fn build(dark: bool, accent: AccentColor, flat: bool) -> Self {
        let a = tokens::accent(accent);
        let accents = if dark { &a.dark } else { &a.light };
        let glass_primary = if flat {
            accents.android_glass_primary()
        } else {
            accents.glass_primary
        };
        let pick = |t: tokens::ColorToken| color(if dark { t.dark } else { t.light });
        // On Android the glass* tokens resolve to opaque fallbacks
        // (v1: colors.ts:95-104, 145-154).
        let (glass_surface, glass_surface_variant, glass_highlight, glass_shadow_edge) = if flat {
            (
                tokens::ANDROID_GLASS_SURFACE,
                tokens::ANDROID_GLASS_SURFACE_VARIANT,
                tokens::ANDROID_GLASS_HIGHLIGHT,
                tokens::ANDROID_GLASS_SHADOW_EDGE,
            )
        } else {
            (
                tokens::GLASS_SURFACE,
                tokens::GLASS_SURFACE_VARIANT,
                tokens::GLASS_HIGHLIGHT,
                tokens::GLASS_SHADOW_EDGE,
            )
        };
        Self {
            dark,
            flat,
            background: pick(tokens::BACKGROUND),
            surface: pick(tokens::SURFACE),
            surface_variant: pick(tokens::SURFACE_VARIANT),
            text_primary: pick(tokens::TEXT_PRIMARY),
            text_secondary: pick(tokens::TEXT_SECONDARY),
            text_tertiary: pick(tokens::TEXT_TERTIARY),
            on_primary: color("#FFFFFF"),
            primary: color(accents.primary),
            primary_dark: color(accents.primary_dark),
            primary_surface: color(accents.primary_surface),
            glass_primary: color(glass_primary),
            border: pick(tokens::BORDER),
            border_input: pick(tokens::BORDER_INPUT),
            success: pick(tokens::SUCCESS),
            success_surface: pick(tokens::SUCCESS_SURFACE),
            success_text: pick(tokens::SUCCESS_TEXT),
            success_border: pick(tokens::SUCCESS_BORDER),
            warning: pick(tokens::WARNING),
            warning_surface: pick(tokens::WARNING_SURFACE),
            warning_text: pick(tokens::WARNING_TEXT),
            warning_border: pick(tokens::WARNING_BORDER),
            error: pick(tokens::ERROR),
            error_surface: pick(tokens::ERROR_SURFACE),
            error_text: pick(tokens::ERROR_TEXT),
            overlay: pick(tokens::OVERLAY),
            glass_surface: pick(glass_surface),
            glass_surface_variant: pick(glass_surface_variant),
            glass_highlight: pick(glass_highlight),
            glass_shadow_edge: pick(glass_shadow_edge),
            // Badges and status surfaces use glass{Success,Warning,Error} on
            // iOS vs {success,warning,error}Surface on Android (04 §7).
            glass_success: if flat {
                pick(tokens::SUCCESS_SURFACE)
            } else {
                pick(tokens::GLASS_SUCCESS)
            },
            glass_warning: if flat {
                pick(tokens::WARNING_SURFACE)
            } else {
                pick(tokens::GLASS_WARNING)
            },
            glass_error: if flat {
                pick(tokens::ERROR_SURFACE)
            } else {
                pick(tokens::GLASS_ERROR)
            },
            desktop: build_desktop_palette(dark, accents),
        }
    }

    // §7 platform style recipes.

    pub fn card_surface(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.surface, 14., self.border, 1)
        } else {
            Recipe::glass(
                self.glass_surface,
                16.,
                self.glass_shadow_edge,
                (0.06, 8., 2.),
            )
        }
    }

    pub fn banner_surface(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.surface, 12., self.border, 1)
        } else {
            Recipe::glass(
                self.glass_surface,
                12.,
                self.glass_shadow_edge,
                (0.04, 4., 1.),
            )
        }
    }

    pub fn tinted_banner(&self, bg: Hsla) -> Recipe {
        if self.flat {
            Recipe::flat(bg, 12., self.border, 1)
        } else {
            Recipe::glass(bg, 12., self.glass_shadow_edge, (0.04, 4., 1.))
        }
    }

    pub fn button_primary(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.primary, 12., self.primary, 2).borderless()
        } else {
            Recipe::glass(self.primary, 14., self.glass_shadow_edge, (0.04, 4., 1.))
                .borderless()
                .tinted_shadow(self.primary, 0.20, 6., (0., 3.))
        }
    }

    pub fn button_secondary(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.surface, 12., self.primary, 0)
        } else {
            Recipe::glass(self.glass_surface, 14., self.border, (0.04, 4., 1.))
                .with_border(px(1.), self.border)
        }
    }

    pub fn button_danger(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.error, 12., self.error, 2).borderless()
        } else {
            // iOS: bg error, radius 14 — no border, no shadow.
            Recipe {
                bg: self.error,
                radius: px(14.),
                border: None,
                shadow: None,
            }
        }
    }

    pub fn input_field(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.surface, 12., self.border_input, 0)
        } else {
            Recipe::glass(
                self.glass_surface,
                12.,
                self.glass_shadow_edge,
                (0.03, 3., 1.),
            )
        }
    }

    pub fn circle_button(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.surface_variant, 24., self.surface_variant, 1).borderless()
        } else {
            Recipe::glass(
                self.glass_surface_variant,
                24.,
                self.glass_shadow_edge,
                (0.04, 4., 1.),
            )
        }
    }

    pub fn fab(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.primary, 16., self.primary, 4).borderless()
        } else {
            Recipe::glass(self.primary, 28., self.glass_shadow_edge, (0.04, 4., 1.))
                .borderless()
                .tinted_shadow(self.primary, 0.25, 12., (0., 4.))
        }
    }

    /// Selected chip/option fill: iOS `glassPrimary`, Android `primarySurface`
    /// with a primary border (04 §7 inline differences).
    pub fn selected_chip(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.primary_surface, 8., self.primary, 0)
        } else {
            Recipe::glass(
                self.glass_primary,
                8.,
                self.glass_shadow_edge,
                (0.04, 4., 1.),
            )
        }
    }

    /// Unselected chip/option fill.
    pub fn unselected_chip(&self) -> Recipe {
        if self.flat {
            Recipe::flat(self.surface, 8., self.border, 0)
        } else {
            Recipe::glass(
                self.glass_surface_variant,
                8.,
                self.glass_shadow_edge,
                (0.04, 4., 1.),
            )
        }
    }

    /// Divider colour: `glassHighlight` iOS / `border` Android (04 §7).
    pub fn divider(&self) -> Hsla {
        if self.flat {
            self.border
        } else {
            self.glass_highlight
        }
    }

    /// Fill of a passive desktop control in `state` (REQ-UI-13). The model
    /// owns the choice; mobile recipe defaults never read it.
    pub fn control_fill(&self, state: ControlState) -> Hsla {
        from_rgb(desktop::control_fill(state, self.dark))
    }

    /// Ink of a passive desktop control in `state` (REQ-UI-13): disabled is
    /// `text_tertiary`, everything else `text_primary`.
    pub fn control_ink(&self, state: ControlState) -> Hsla {
        from_rgb(desktop::control_ink(state, self.dark))
    }

    /// Boundary of a passive desktop control: `border_input` normally and
    /// `focusInk` while focused, so the outline itself also reads at 3:1.
    pub fn control_border(&self, state: ControlState) -> Hsla {
        if state.ring_visible() {
            self.desktop.focus
        } else {
            self.border_input
        }
    }

    /// The one desktop workspace panel: `surface` fill, 12 px radius, 1 px
    /// `border`, no shadow (brief "Surfaces"; shadows are a mobile dialect
    /// here). One panel per workspace, one divider between the panes.
    pub fn workspace_panel(&self) -> Recipe {
        Recipe {
            bg: self.desktop.panel,
            radius: px(12.),
            border: Some((px(1.), self.border)),
            shadow: None,
        }
    }
}

/// Derive the desktop palette from the pinned tokens only (REQ-UI-13): the
/// model helpers own the maths so the model tests can check the contrast
/// floors without gpui.
fn build_desktop_palette(dark: bool, accents: &tokens::AccentColors) -> DesktopPalette {
    let ink = desktop::text_ink(dark);
    let panel = desktop::panel_fill(dark);
    let rail = desktop::rail_fill(dark);
    let accent = desktop::parse_color(accents.primary);
    let selected = desktop::control_fill(desktop::ControlState::Selected, dark);
    let states = |fill: Rgb| DesktopAction {
        fill: from_rgb(fill),
        hover: from_rgb(desktop::action_hover_fill(fill, ink)),
        pressed: from_rgb(desktop::action_pressed_fill(fill, ink)),
        ink: from_rgb(desktop::on_fill(fill)),
    };
    DesktopPalette {
        action: states(desktop::action_fill(accents, dark)),
        danger: states(desktop::danger_fill(dark)),
        panel: from_rgb(panel),
        rail: from_rgb(rail),
        focus: from_rgb(desktop::focus_ink(dark)),
        ink: from_rgb(ink),
        accent_on_panel: from_rgb(desktop::readable_ink(panel, accent)),
        accent_on_rail: from_rgb(desktop::readable_ink(rail, accent)),
        accent_on_selection: from_rgb(desktop::readable_ink(selected, accent)),
        accent_boundary: from_rgb(desktop::boundary_ink(selected, accent)),
    }
}

/// Style a `Div` with a recipe: background, radius, border and shadow.
pub fn apply(div: Div, r: &Recipe) -> Div {
    let div = div.bg(r.bg).rounded(r.radius);
    let div = match r.border {
        Some((w, c)) => div.border(w).border_color(c),
        None => div,
    };
    match r.shadow.clone() {
        Some(s) => div.shadow(vec![s]),
        None => div,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse a token and return the 0-255 channels for comparison.
    fn rgb(value: &str) -> (u32, u32, u32, u32) {
        let rgba: Rgba = color(value).into();
        let q = |x: f32| (x * 255.0).round() as u32;
        (q(rgba.r), q(rgba.g), q(rgba.b), q(rgba.a))
    }

    #[test]
    fn color_parses_every_token_shape() {
        assert_eq!(rgb("#F00"), (255, 0, 0, 255));
        assert_eq!(rgb("#F008"), (255, 0, 0, 136));
        assert_eq!(rgb("#D4501A"), (212, 80, 26, 255));
        assert_eq!(rgb("#D4501A80"), (212, 80, 26, 128));
        // rgba() tokens: channels 0-255, alpha 0-1 (04 §8). Regression for
        // the parser bug fixed in 82ede35.
        assert_eq!(rgb("rgba(20, 32, 24, 0.5)"), (20, 32, 24, 128));
        assert_eq!(rgb("rgba(255,255,255,1)"), (255, 255, 255, 255));
        assert_eq!(rgb(" #F5F5F7 "), (245, 245, 247, 255));
    }

    #[test]
    #[should_panic(expected = "token must be #hex or rgba()")]
    fn color_rejects_garbage() {
        color("transparent");
    }

    #[test]
    fn resolve_follows_and_forces_appearance() {
        let flat = use_flat_style();
        let dark_text = AppTheme::build(true, AccentColor::Orange, flat).text_primary;
        let light_text = AppTheme::build(false, AccentColor::Orange, flat).text_primary;
        assert_ne!(dark_text, light_text);
        // System follows the OS; Hell/Dunkel force their scheme.
        let system_dark =
            AppTheme::resolve(ThemePreference::System, AccentColor::Orange, true).text_primary;
        let system_light =
            AppTheme::resolve(ThemePreference::System, AccentColor::Orange, false).text_primary;
        assert_eq!(system_dark, dark_text);
        assert_eq!(system_light, light_text);
        let forced_dark =
            AppTheme::resolve(ThemePreference::Dark, AccentColor::Orange, false).text_primary;
        let forced_light =
            AppTheme::resolve(ThemePreference::Light, AccentColor::Orange, true).text_primary;
        assert_eq!(forced_dark, dark_text);
        assert_eq!(forced_light, light_text);
    }

    #[test]
    fn recipes_match_spec_borders() {
        let t = AppTheme::build(false, AccentColor::Orange, false);
        // 04 §7: buttonSecondary iOS border 1 `border`; cardSurface 0.5.
        let secondary = t.button_secondary();
        assert_eq!(secondary.border.map(|(w, _)| w), Some(px(1.)));
        let card = t.card_surface();
        assert_eq!(card.border.map(|(w, _)| w), Some(px(0.5)));
        // buttonPrimary and fab carry no border on iOS.
        assert_eq!(t.button_primary().border, None);
        assert_eq!(t.fab().border, None);
        // Android: 1 px borders from the flat dialect.
        let a = AppTheme::build(false, AccentColor::Orange, true);
        assert_eq!(a.button_secondary().border.map(|(w, _)| w), Some(px(1.)));
        assert_eq!(a.card_surface().border.map(|(w, _)| w), Some(px(1.)));
    }

    /// The desktop control states paint exactly the model's token choice:
    /// rest/hover/selected fills, focus on the rest fill, disabled ink only.
    /// Mobile recipes never call these, so no legacy value moves.
    /// @req REQ-UI-13
    #[test]
    fn desktop_control_states_paint_model_colours() {
        for dark in [false, true] {
            let t = AppTheme::build(dark, AccentColor::Orange, true);
            for state in [
                ControlState::Rest,
                ControlState::Hover,
                ControlState::Focus,
                ControlState::Disabled,
                ControlState::Selected,
            ] {
                assert_eq!(
                    t.control_fill(state),
                    from_rgb(desktop::control_fill(state, dark)),
                    "fill for {state:?}"
                );
                assert_eq!(
                    t.control_ink(state),
                    from_rgb(desktop::control_ink(state, dark)),
                    "ink for {state:?}"
                );
                assert_eq!(
                    t.control_border(state),
                    if state.ring_visible() {
                        t.desktop.focus
                    } else {
                        t.border_input
                    }
                );
            }
            // Disabled changes only the ink; the fill stays the rest fill.
            assert_eq!(
                t.control_fill(ControlState::Disabled),
                t.control_fill(ControlState::Rest)
            );
            assert_ne!(
                t.control_ink(ControlState::Disabled),
                t.control_ink(ControlState::Rest)
            );
            // Selected is a real tint, not the rest fill.
            assert_ne!(
                t.control_fill(ControlState::Selected),
                t.control_fill(ControlState::Rest)
            );
        }
    }

    /// The desktop extension consumes the pinned tokens only: filled actions
    /// are `primaryDark` (light) / `primary` (dark), destructive is `error`,
    /// panel is `surface`, rail is `background`, focus/ink are `textPrimary`,
    /// hover/press are the fixed ink mixes, and the workspace panel is a
    /// 12 px, 1 px-border, shadowless surface (desktop-ui-redesign D4).
    /// @req REQ-UI-13
    #[test]
    fn desktop_palette_derives_from_pinned_tokens() {
        let t = AppTheme::build(false, AccentColor::Orange, true);
        assert_eq!(
            t.desktop.action.fill,
            color("#B84415"),
            "light fill is primaryDark"
        );
        assert_eq!(
            t.desktop.danger.fill,
            color("#D93025"),
            "danger is the error token"
        );
        assert_eq!(t.desktop.panel, color("#fff"), "panel is surface");
        assert_eq!(t.desktop.rail, color("#F5F5F7"), "rail is background");
        assert_eq!(t.desktop.focus, color("#1D1D1F"), "focus is textPrimary");
        assert_eq!(t.desktop.ink, color("#1D1D1F"), "ink is textPrimary");
        let ink = desktop::text_ink(false);
        let fill = desktop::parse_color("#B84415");
        assert_eq!(
            t.desktop.action.hover,
            from_rgb(desktop::action_hover_fill(fill, ink)),
            "hover is the 8 % ink mix"
        );
        assert_eq!(
            t.desktop.action.pressed,
            from_rgb(desktop::action_pressed_fill(fill, ink)),
            "pressed is the 14 % ink mix"
        );
        assert_eq!(
            t.desktop.action.ink,
            color("#FFFFFF"),
            "white reads best on the orange fill"
        );

        // Accent as text is resolved against the fill it sits on (REQ-UI-13):
        // light orange only reaches 4.23:1 on the white panel, so the ink
        // falls back to text_primary; emerald clears it and keeps the accent.
        assert_eq!(
            t.desktop.accent_on_panel,
            color("#1D1D1F"),
            "unreadable accent text falls back to text_primary"
        );
        let emerald = AppTheme::build(false, AccentColor::Emerald, true);
        assert_eq!(
            emerald.desktop.accent_on_panel, emerald.primary,
            "a readable accent stays as panel text"
        );
        assert_eq!(
            emerald.desktop.accent_on_rail, emerald.primary,
            "a readable accent stays as rail text"
        );
        // A selected chip paints accent text on the selection fill, not the
        // panel, so it resolves against that fill (REQ-UI-13): light orange
        // only reaches 3.84:1 there and falls back to text_primary.
        assert_eq!(
            t.desktop.accent_on_selection,
            color("#1D1D1F"),
            "unreadable chip accent falls back to text_primary"
        );
        let berry = AppTheme::build(false, AccentColor::Berry, true);
        assert_eq!(
            berry.desktop.accent_on_selection, berry.primary,
            "a readable chip accent stays as the selected chip's ink"
        );
        // A selected card edge is a control boundary on the selection fill
        // (REQ-UI-13): light golden only reaches 2.78:1 there, so the edge
        // falls back to text_primary while orange keeps its accent.
        assert_eq!(
            t.desktop.accent_boundary, t.primary,
            "a readable accent edge stays the accent"
        );
        let golden = AppTheme::build(false, AccentColor::Golden, true);
        assert_eq!(
            golden.desktop.accent_boundary,
            color("#1D1D1F"),
            "an unreadable accent edge falls back to text_primary"
        );

        let d = AppTheme::build(true, AccentColor::Ocean, true);
        assert_eq!(
            d.desktop.action.fill,
            color("#4A90D9"),
            "dark fill is the base accent"
        );
        assert_eq!(
            d.desktop.rail,
            color("#000000"),
            "dark rail is black background"
        );

        let panel = t.workspace_panel();
        assert_eq!(panel.bg, t.desktop.panel);
        assert_eq!(panel.radius, px(12.));
        assert_eq!(panel.border.map(|(w, _)| w), Some(px(1.)));
        assert!(
            panel.shadow.is_none(),
            "the desktop panel carries no shadow"
        );
    }
}
