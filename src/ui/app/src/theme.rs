//! Theme resolution and the platform style recipes (openspec: gpui-kit-ui-rewrite,
//! task 4.1; REQ-UI-05).
//!
//! The colour values live as plain data in `snackpilot_model::tokens` (04 §8);
//! this layer parses them into gpui colours, resolves the `System`/`Hell`/
//! `Dunkel` preference against the OS appearance, and exposes the §7 recipes
//! with the iOS-glass versus flat-Android split (v1 `useFlatStyle`).

use gpui_kit::{point, px, BoxShadow, Div, Hsla, Pixels, Rgba, Styled};
use snackpilot_model::tokens;
use snackpilot_model::{AccentColor, ThemePreference};

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
        (
            (n[0] * 255.0).round() as u32,
            (n[1] * 255.0).round() as u32,
            (n[2] * 255.0).round() as u32,
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
