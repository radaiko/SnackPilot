//! One flat theme for every platform: parse the pinned tokens into gpui
//! colours and expose the type, spacing, radius and target scales.
//!
//! There is no accent selection, no glass dialect and no per-platform recipe
//! split: desktop uses the same components at wider widths.

use gpui::{hsla, InteractiveElement};
use gpui_kit::{px, Div, Hsla, Pixels, Rgba, StyleRefinement, Styled};
use snackpilot_model::desktop::{ControlState, Rgb};
use snackpilot_model::tokens;
use snackpilot_model::ThemePreference;

/// Convert a model-side `Rgb` into a gpui colour.
pub fn from_rgb(c: Rgb) -> Hsla {
    Rgba {
        r: c.r as f32 / 255.0,
        g: c.g as f32 / 255.0,
        b: c.b as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

/// Parse a token value (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` or
/// `rgba(r,g,b,a)`) into a gpui colour. Unparsable input is a token-table bug,
/// so it panics — the token tests pin the table.
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

/// The spacing, type, radius, target and icon scale.
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub space_1: f32,
    pub space_2: f32,
    pub space_3: f32,
    pub space_4: f32,
    pub space_5: f32,
    pub space_6: f32,
    pub title: f32,
    pub title_line: f32,
    pub headline: f32,
    pub headline_line: f32,
    pub body: f32,
    pub body_line: f32,
    pub meta: f32,
    pub meta_line: f32,
    pub target: f32,
    pub action_height: f32,
    pub radius_card: f32,
    pub radius_control: f32,
    pub radius_chip: f32,
    pub focus_width: f32,
    pub focus_gap: f32,
    pub icon: f32,
    pub icon_meta: f32,
}

impl Scale {
    fn from_tokens() -> Self {
        Self {
            space_1: tokens::SPACE_1,
            space_2: tokens::SPACE_2,
            space_3: tokens::SPACE_3,
            space_4: tokens::SPACE_4,
            space_5: tokens::SPACE_5,
            space_6: tokens::SPACE_6,
            title: tokens::TITLE.size,
            title_line: tokens::TITLE.line_height,
            headline: tokens::HEADLINE.size,
            headline_line: tokens::HEADLINE.line_height,
            body: tokens::BODY.size,
            body_line: tokens::BODY.line_height,
            meta: tokens::META.size,
            meta_line: tokens::META.line_height,
            target: tokens::TARGET,
            action_height: tokens::ACTION_HEIGHT,
            radius_card: tokens::RADIUS_CARD,
            radius_control: tokens::RADIUS_CONTROL,
            radius_chip: tokens::RADIUS_CHIP,
            focus_width: tokens::FOCUS_WIDTH,
            focus_gap: tokens::FOCUS_GAP,
            icon: tokens::ICON,
            icon_meta: tokens::ICON_META,
        }
    }
}

/// One surface recipe: fill, radius and optional 1 px border.
#[derive(Debug, Clone)]
pub struct Recipe {
    pub bg: Hsla,
    pub radius: Pixels,
    pub border: Option<(Pixels, Hsla)>,
}

/// Every token resolved for one appearance.
#[derive(Debug, Clone)]
pub struct AppTheme {
    pub dark: bool,

    pub background: Hsla,
    pub surface: Hsla,
    pub surface_alt: Hsla,
    pub border: Hsla,
    pub border_strong: Hsla,

    pub text_primary: Hsla,
    pub text_secondary: Hsla,
    pub text_tertiary: Hsla,

    pub accent: Hsla,
    pub accent_surface: Hsla,
    pub on_accent: Hsla,
    /// Hover/pressed accents for pointer feedback.
    pub accent_hover: Hsla,
    pub accent_pressed: Hsla,

    pub success: Hsla,
    pub success_surface: Hsla,
    pub warning: Hsla,
    pub warning_surface: Hsla,
    pub error: Hsla,
    pub error_surface: Hsla,
    pub on_error: Hsla,
    pub error_hover: Hsla,
    pub error_pressed: Hsla,
    pub scrim: Hsla,

    pub scale: Scale,
}

impl AppTheme {
    /// Resolve a preference against the OS appearance: `System` follows
    /// `os_dark`, `Hell`/`Dunkel` force their scheme.
    pub fn resolve(preference: ThemePreference, os_dark: bool) -> Self {
        let dark = match preference {
            ThemePreference::System => os_dark,
            ThemePreference::Light => false,
            ThemePreference::Dark => true,
        };
        Self::build(dark)
    }

    /// Build one appearance directly (the gallery shows both side by side).
    pub fn build(dark: bool) -> Self {
        let pick = |t: tokens::ColorToken| color(if dark { t.dark } else { t.light });
        let accent = pick(tokens::ACCENT);
        let error = pick(tokens::ERROR);
        let text = pick(tokens::TEXT);
        Self {
            dark,
            background: pick(tokens::BG),
            surface: pick(tokens::SURFACE),
            surface_alt: pick(tokens::SURFACE_ALT),
            border: pick(tokens::BORDER),
            border_strong: pick(tokens::BORDER_STRONG),
            text_primary: text,
            text_secondary: pick(tokens::TEXT_SECONDARY),
            text_tertiary: pick(tokens::TEXT_TERTIARY),
            accent,
            accent_surface: pick(tokens::ACCENT_SURFACE),
            on_accent: pick(tokens::ON_ACCENT),
            accent_hover: darken(accent, 0.06),
            accent_pressed: darken(accent, 0.12),
            success: pick(tokens::SUCCESS),
            success_surface: pick(tokens::SUCCESS_SURFACE),
            warning: pick(tokens::WARNING),
            warning_surface: pick(tokens::WARNING_SURFACE),
            error,
            error_surface: pick(tokens::ERROR_SURFACE),
            on_error: pick(tokens::ON_ACCENT),
            error_hover: darken(error, 0.06),
            error_pressed: darken(error, 0.12),
            scrim: pick(tokens::SCRIM),
            scale: Scale::from_tokens(),
        }
    }

    pub fn card_surface(&self) -> Recipe {
        Recipe {
            bg: self.surface,
            radius: px(self.scale.radius_card),
            border: Some((px(1.), self.border)),
        }
    }

    pub fn banner_surface(&self) -> Recipe {
        Recipe {
            bg: self.surface,
            radius: px(self.scale.radius_card),
            border: Some((px(1.), self.border)),
        }
    }

    pub fn tinted_banner(&self, bg: Hsla) -> Recipe {
        Recipe {
            bg,
            radius: px(self.scale.radius_card),
            border: Some((px(1.), self.border)),
        }
    }

    pub fn button_primary(&self) -> Recipe {
        Recipe {
            bg: self.accent,
            radius: px(self.scale.radius_control),
            border: None,
        }
    }

    pub fn button_secondary(&self) -> Recipe {
        Recipe {
            bg: self.surface,
            radius: px(self.scale.radius_control),
            border: Some((px(1.), self.text_tertiary)),
        }
    }

    pub fn button_danger(&self) -> Recipe {
        Recipe {
            bg: self.error,
            radius: px(self.scale.radius_control),
            border: None,
        }
    }

    pub fn circle_button(&self) -> Recipe {
        Recipe {
            bg: self.surface_alt,
            radius: px(self.scale.radius_control),
            border: None,
        }
    }

    /// Fill of a passive control in `state`.
    pub fn control_fill(&self, state: ControlState) -> Hsla {
        match state {
            ControlState::Hover => self.surface_alt,
            ControlState::Selected => self.accent_surface,
            ControlState::Rest | ControlState::Focus | ControlState::Disabled => self.surface,
        }
    }

    /// Ink of a passive control in `state`: disabled is tertiary.
    pub fn control_ink(&self, state: ControlState) -> Hsla {
        if state == ControlState::Disabled {
            self.text_secondary
        } else {
            self.text_primary
        }
    }

    /// Boundary of a passive control: the accent ring while focused.
    pub fn control_border(&self, state: ControlState) -> Hsla {
        if state.ring_visible() {
            self.accent
        } else {
            self.text_tertiary
        }
    }

    /// One desktop workspace panel: surface, 12 px radius, 1 px border.
    pub fn workspace_panel(&self) -> Recipe {
        Recipe {
            bg: self.surface,
            radius: px(self.scale.radius_card),
            border: Some((px(1.), self.border)),
        }
    }

    /// Fill-only hover feedback: a 4 % text mix into the resting fill.
    pub fn hover(&self, fill: Hsla) -> Hsla {
        mix_with_text(fill, self.text_primary, 0.04)
    }

    /// Fill-only press feedback: an 8 % text mix into the resting fill.
    pub fn pressed(&self, fill: Hsla) -> Hsla {
        mix_with_text(fill, self.text_primary, 0.08)
    }
}

/// Mix `amount` of `text` into `fill`, keeping the fill's alpha.
fn mix_with_text(fill: Hsla, text: Hsla, amount: f32) -> Hsla {
    let a: Rgba = fill.into();
    let t: Rgba = text.into();
    let mix = |f: f32, t: f32| f * (1.0 - amount) + t * amount;
    Rgba {
        r: mix(a.r, t.r),
        g: mix(a.g, t.g),
        b: mix(a.b, t.b),
        a: a.a,
    }
    .into()
}

/// Hover/press shade: drop the lightness channel by `amount` (0..1).
pub fn darken(c: Hsla, amount: f32) -> Hsla {
    Hsla {
        l: (c.l - amount).clamp(0.0, 1.0),
        ..c
    }
}

/// Focus recipe: a 2 px ring in the resolved focus ink on the target's own
/// shape, applied with `.focus(theme::desktop_focus(ink))`.
pub fn desktop_focus(ink: Hsla) -> impl FnOnce(StyleRefinement) -> StyleRefinement {
    move |s| s.border(px(tokens::FOCUS_WIDTH)).border_color(ink)
}

/// Reserve the 2 + 2 px focus ring around a control; the frame is identical
/// in every state, so focusing never shifts layout or clips the ring.
pub fn focus_frame(div: Div, state: ControlState, radius: f32, theme: &AppTheme) -> Div {
    let ring = if state.ring_visible() {
        theme.accent
    } else {
        hsla(0., 0., 0., 0.)
    };
    div.p(px(tokens::FOCUS_GAP))
        .rounded(px(radius + tokens::FOCUS_GAP + tokens::FOCUS_WIDTH))
        .border(px(tokens::FOCUS_WIDTH))
        .border_color(ring)
        .focus(desktop_focus(theme.accent))
}

/// Byte-wise `Rgb` for a painted gpui colour measured on `backdrop`: a
/// translucent fill is composited first.
pub fn to_rgb(c: Hsla, backdrop: Hsla) -> Rgb {
    let fill: Rgba = c.into();
    let back: Rgba = backdrop.into();
    let a = fill.a.clamp(0.0, 1.0);
    let q = |x: f32| (x * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgb::new(
        q(fill.r * a + back.r * (1.0 - a)),
        q(fill.g * a + back.g * (1.0 - a)),
        q(fill.b * a + back.b * (1.0 - a)),
    )
}

/// Style a `Div` with a recipe: background, radius and border.
pub fn apply(div: Div, r: &Recipe) -> Div {
    let div = div.bg(r.bg).rounded(r.radius);
    match r.border {
        Some((w, c)) => div.border(w).border_color(c),
        None => div,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(value: &str) -> (u32, u32, u32, u32) {
        let rgba: Rgba = color(value).into();
        let q = |x: f32| (x * 255.0).round() as u32;
        (q(rgba.r), q(rgba.g), q(rgba.b), q(rgba.a))
    }

    #[test]
    fn color_parses_the_accent_value() {
        assert_eq!(rgb("#C2450F"), (194, 69, 15, 255));
        assert_eq!(rgb("rgba(0, 0, 0, 0.4)"), (0, 0, 0, 102));
    }

    #[test]
    fn resolve_follows_and_forces_appearance() {
        let dark_text = AppTheme::build(true).text_primary;
        let light_text = AppTheme::build(false).text_primary;
        assert_ne!(dark_text, light_text);
        assert_eq!(
            AppTheme::resolve(ThemePreference::System, true).text_primary,
            dark_text
        );
        assert_eq!(
            AppTheme::resolve(ThemePreference::System, false).text_primary,
            light_text
        );
        assert_eq!(
            AppTheme::resolve(ThemePreference::Dark, false).text_primary,
            dark_text
        );
        assert_eq!(
            AppTheme::resolve(ThemePreference::Light, true).text_primary,
            light_text
        );
    }

    #[test]
    fn recipes_use_the_one_flat_system() {
        let t = AppTheme::build(false);
        assert_eq!(t.card_surface().radius, px(12.));
        assert_eq!(t.card_surface().border.map(|(w, _)| w), Some(px(1.)));
        assert!(t.button_primary().border.is_none());
        assert_eq!(t.button_secondary().radius, px(10.));
        assert_eq!(t.scale.headline, 20.);
        assert_eq!(t.scale.body, 17.);
        assert_eq!(t.scale.meta, 13.);
    }

    /// @req REQ-UI-12
    #[test]
    fn hover_and_press_mix_the_text_ink() {
        let t = AppTheme::build(false);
        let fill: Rgba = t.surface.into();
        let hover: Rgba = t.hover(t.surface).into();
        let press: Rgba = t.pressed(t.surface).into();
        let text: Rgba = t.text_primary.into();
        let expect = |amount: f32, channel: f32, ink: f32| channel * (1.0 - amount) + ink * amount;
        assert!((hover.r - expect(0.04, fill.r, text.r)).abs() < 0.0001);
        assert!((press.r - expect(0.08, fill.r, text.r)).abs() < 0.0001);
        assert!((press.r - fill.r).abs() > (hover.r - fill.r).abs());
    }

    #[test]
    fn control_states_paint_the_flat_palette() {
        for dark in [false, true] {
            let t = AppTheme::build(dark);
            assert_eq!(t.control_fill(ControlState::Rest), t.surface);
            assert_eq!(t.control_fill(ControlState::Hover), t.surface_alt);
            assert_eq!(t.control_fill(ControlState::Selected), t.accent_surface);
            assert_eq!(t.control_ink(ControlState::Disabled), t.text_secondary);
            assert_eq!(t.control_border(ControlState::Focus), t.accent);
            assert_eq!(t.control_border(ControlState::Rest), t.text_tertiary);
        }
    }
}
