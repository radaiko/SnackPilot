//! Design tokens: the colour values and accent definitions from
//! `docs/requirements/04-ui-ux.md` §8, as plain data (no UI framework here).
//!
//! Values are kept exactly as the spec's tables state: `#RGB`/`#RRGGBB`/
//! `#RRGGBBAA` hex strings or `rgba(r,g,b,a)` strings. The app layer parses
//! them into framework colours; this crate stays testable without gpui.

/// A colour with a light- and dark-appearance value (04 §8 token table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorToken {
    pub light: &'static str,
    pub dark: &'static str,
}

impl ColorToken {
    pub const fn both(light: &'static str, dark: &'static str) -> Self {
        Self { light, dark }
    }

    /// A token whose value does not change with appearance.
    pub const fn fixed(value: &'static str) -> Self {
        Self {
            light: value,
            dark: value,
        }
    }
}

/// The four accent-derived colours of one appearance (04 §8 accent table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    pub primary: &'static str,
    pub primary_dark: &'static str,
    pub primary_surface: &'static str,
    /// iOS-glass tinted fill; on Android it equals `primary_surface` (04 §8).
    pub glass_primary: &'static str,
}

impl AccentColors {
    /// On Android the accent's `glassPrimary` equals its `primarySurface`
    /// (04 §8, v1: colors.ts:183-249).
    pub const fn android_glass_primary(&self) -> &'static str {
        self.primary_surface
    }
}

/// One selectable accent (04 §8): id, German label, both appearances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accent {
    pub id: &'static str,
    pub label: &'static str,
    pub light: AccentColors,
    pub dark: AccentColors,
}

/// All five accents, in the order the settings screen shows them (04 §8).
pub const ACCENTS: [Accent; 5] = [
    Accent {
        id: "orange",
        label: "Orange",
        light: AccentColors {
            primary: "#D4501A",
            primary_dark: "#B84415",
            primary_surface: "#FFF1EB",
            glass_primary: "rgba(212,80,26,0.08)",
        },
        dark: AccentColors {
            primary: "#FF6B35",
            primary_dark: "#D4501A",
            primary_surface: "#2A1A10",
            glass_primary: "rgba(255,107,53,0.14)",
        },
    },
    Accent {
        id: "emerald",
        label: "Smaragd",
        light: AccentColors {
            primary: "#2E7D4F",
            primary_dark: "#236B3F",
            primary_surface: "#E8F5ED",
            glass_primary: "rgba(46,125,79,0.08)",
        },
        dark: AccentColors {
            primary: "#4CAF7D",
            primary_dark: "#2E7D4F",
            primary_surface: "#102A1A",
            glass_primary: "rgba(76,175,125,0.14)",
        },
    },
    Accent {
        id: "berry",
        label: "Beere",
        light: AccentColors {
            primary: "#A62547",
            primary_dark: "#8C1E3B",
            primary_surface: "#FCEEF2",
            glass_primary: "rgba(166,37,71,0.08)",
        },
        dark: AccentColors {
            primary: "#E04868",
            primary_dark: "#A62547",
            primary_surface: "#2A1018",
            glass_primary: "rgba(224,72,104,0.14)",
        },
    },
    Accent {
        id: "golden",
        label: "Gold",
        light: AccentColors {
            primary: "#C08B1A",
            primary_dark: "#A07415",
            primary_surface: "#FDF5E3",
            glass_primary: "rgba(192,139,26,0.08)",
        },
        dark: AccentColors {
            primary: "#E8B03E",
            primary_dark: "#C08B1A",
            primary_surface: "#2A2210",
            glass_primary: "rgba(232,176,62,0.14)",
        },
    },
    Accent {
        id: "ocean",
        label: "Ozean",
        light: AccentColors {
            primary: "#2563A8",
            primary_dark: "#1E528C",
            primary_surface: "#EBF2FC",
            glass_primary: "rgba(37,99,168,0.08)",
        },
        dark: AccentColors {
            primary: "#4A90D9",
            primary_dark: "#2563A8",
            primary_surface: "#101A2A",
            glass_primary: "rgba(74,144,217,0.14)",
        },
    },
];

/// The accent record for one core accent choice.
pub fn accent(a: crate::state::AccentColor) -> &'static Accent {
    &ACCENTS[a as usize]
}

// Base colour tokens, 04 §8 token table. The accent-derived tokens above
// replace primary/primaryDark/primarySurface/glassPrimary per accent choice;
// the constants here carry the default Orange values from the table.

/// Screen background — pure black in dark mode (04 §8).
pub const BACKGROUND: ColorToken = ColorToken::both("#F5F5F7", "#000000");
pub const SURFACE: ColorToken = ColorToken::both("#fff", "#1C1C1E");
pub const SURFACE_VARIANT: ColorToken = ColorToken::both("#EDEDF0", "#2C2C2E");
pub const TEXT_PRIMARY: ColorToken = ColorToken::both("#1D1D1F", "#F5F5F7");
pub const TEXT_SECONDARY: ColorToken = ColorToken::both("#6E6E73", "#A1A1A6");
pub const TEXT_TERTIARY: ColorToken = ColorToken::both("#AEAEB2", "#636366");
pub const PRIMARY: ColorToken = ColorToken::both("#D4501A", "#FF6B35");
pub const PRIMARY_DARK: ColorToken = ColorToken::both("#B84415", "#D4501A");
pub const PRIMARY_SURFACE: ColorToken = ColorToken::both("#FFF1EB", "#2A1A10");
pub const BORDER: ColorToken = ColorToken::both("#D6D6D8", "#38383A");
pub const BORDER_INPUT: ColorToken = ColorToken::both("#CECED0", "#48484A");
pub const SUCCESS: ColorToken = ColorToken::fixed("#34A853");
pub const SUCCESS_SURFACE: ColorToken = ColorToken::both("#EBF5EE", "#142018");
pub const SUCCESS_TEXT: ColorToken = ColorToken::both("#1E7E34", "#5DB075");
pub const SUCCESS_BORDER: ColorToken = ColorToken::both("#34A853", "#2E7D32");
pub const WARNING: ColorToken = ColorToken::fixed("#F5A623");
pub const WARNING_SURFACE: ColorToken = ColorToken::both("#FEF6E6", "#2A1E0E");
pub const WARNING_TEXT: ColorToken = ColorToken::both("#B57A10", "#F5C564");
pub const WARNING_BORDER: ColorToken = ColorToken::both("#F5C564", "#B57A10");
pub const ERROR: ColorToken = ColorToken::both("#D93025", "#EA4335");
pub const ERROR_SURFACE: ColorToken = ColorToken::both("#FCE8E6", "#2A1614");
pub const ERROR_TEXT: ColorToken = ColorToken::both("#B3261E", "#F28B82");
pub const OVERLAY: ColorToken = ColorToken::both("rgba(255,255,255,0.7)", "rgba(0,0,0,0.7)");

// iOS-glass translucent fills (04 §8). On Android these fall back to the
// opaque values below (v1: colors.ts:95-104, 145-154); `Theme` applies that
// split via `useFlatStyle`.

pub const GLASS_SURFACE: ColorToken =
    ColorToken::both("rgba(255,255,255,0.70)", "rgba(28,28,30,0.72)");
pub const GLASS_SURFACE_VARIANT: ColorToken =
    ColorToken::both("rgba(237,237,240,0.60)", "rgba(44,44,46,0.60)");
pub const GLASS_HIGHLIGHT: ColorToken =
    ColorToken::both("rgba(255,255,255,0.85)", "rgba(255,255,255,0.12)");
pub const GLASS_SHADOW_EDGE: ColorToken = ColorToken::both("rgba(0,0,0,0.06)", "rgba(0,0,0,0.40)");
pub const GLASS_SUCCESS: ColorToken =
    ColorToken::both("rgba(52,168,83,0.10)", "rgba(52,168,83,0.16)");
pub const GLASS_WARNING: ColorToken =
    ColorToken::both("rgba(245,166,35,0.10)", "rgba(245,166,35,0.16)");
pub const GLASS_ERROR: ColorToken =
    ColorToken::both("rgba(217,48,37,0.10)", "rgba(234,67,53,0.16)");
/// Glass tint of the default Orange accent; accents override it (04 §8).
pub const GLASS_PRIMARY: ColorToken =
    ColorToken::both("rgba(212,80,26,0.08)", "rgba(255,107,53,0.14)");

/// Android opaque fallbacks for the glass tokens (04 §8).
pub const ANDROID_GLASS_SURFACE: ColorToken = ColorToken::both("#ffffff", "#1C1C1E");
pub const ANDROID_GLASS_SURFACE_VARIANT: ColorToken = ColorToken::both("#EDEDF0", "#2C2C2E");
pub const ANDROID_GLASS_HIGHLIGHT: ColorToken = ColorToken::both("#D6D6D8", "#38383A");
pub const ANDROID_GLASS_SHADOW_EDGE: ColorToken = ColorToken::both("#D6D6D8", "#000000");

/// Blur material names and intensities (04 §8); only iOS blurs, so the
/// intensities are the glass-dialect values.
pub const BLUR_TINT_LIGHT: &str = "systemThinMaterial";
pub const BLUR_TINT_DARK: &str = "systemThickMaterialDark";
pub const BLUR_INTENSITY: (u32, u32) = (40, 50);
pub const BLUR_INTENSITY_STRONG: (u32, u32) = (80, 90);
pub const BLUR_INTENSITY_SUBTLE: (u32, u32) = (25, 30);

/// Mobile Day Tray extension (mobile-ui-rework design, shared token table).
///
/// Append-only: every value below is new, and no base token or accent above
/// changes. This is a **mobile presentation projection**, never
/// `theme.desktop`: only the generic WCAG colour arithmetic is shared with the
/// desktop module, no desktop palette output is reused as a mobile value.
/// @req REQ-UI-15
pub mod mobile {
    use super::{
        AccentColors, ColorToken, BACKGROUND, ERROR_TEXT, SUCCESS_TEXT, SURFACE, SURFACE_VARIANT,
        TEXT_PRIMARY, TEXT_SECONDARY, WARNING_TEXT,
    };
    use crate::desktop::{contrast_ratio, mix_toward, parse_color, Rgb};

    /// Named spacing scale (`mobile.space.*`), logical px.
    pub const SPACE_1: f32 = 4.0;
    pub const SPACE_2: f32 = 8.0;
    pub const SPACE_3: f32 = 12.0;
    pub const SPACE_4: f32 = 16.0;
    pub const SPACE_5: f32 = 20.0;
    pub const SPACE_6: f32 = 24.0;
    pub const SPACE_8: f32 = 32.0;

    /// Centred single-column content cap (`mobile.contentMax`).
    pub const CONTENT_MAX: f32 = 560.0;
    /// Credential form cap (`mobile.formMax`).
    pub const FORM_MAX: f32 = 440.0;
    /// Two-axis touch target minimum (`mobile.target`).
    pub const TARGET: f32 = 44.0;
    /// Primary action height (`mobile.actionHeight`).
    pub const ACTION_HEIGHT: f32 = 48.0;

    /// One of the three text roles (`mobile.type.*`).
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct TypeRole {
        pub size: f32,
        pub line_height: f32,
    }

    pub const TYPE_HEADLINE: TypeRole = TypeRole {
        size: 24.0,
        line_height: 32.0,
    };
    pub const TYPE_BODY: TypeRole = TypeRole {
        size: 16.0,
        line_height: 24.0,
    };
    pub const TYPE_META: TypeRole = TypeRole {
        size: 14.0,
        line_height: 20.0,
    };

    /// `mobile.radius.option` / `mobile.radius.status`.
    pub const RADIUS_OPTION: f32 = 8.0;
    pub const RADIUS_STATUS: f32 = 8.0;
    /// Independent focus ring stroke and the gap that reserves its room.
    pub const FOCUS_WIDTH: f32 = 2.0;
    pub const FOCUS_GAP: f32 = 2.0;
    /// Pressed fill is this much `textPrimary` mixed into the resting fill.
    pub const PRESSED_MIX: f32 = 0.14;

    /// WCAG AA thresholds used by the derived-ink projections.
    pub const TEXT_MIN_CONTRAST: f32 = 4.5;
    pub const CONTROL_MIN_CONTRAST: f32 = 3.0;

    /// Mobile column geometry at one host and logical viewport width.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Geometry {
        pub gutter: f32,
        pub content: f32,
        pub form: f32,
    }

    /// The mobile single-column projection: 16 px gutters up to the 560 px
    /// cap, then centred gutters; the form is the 440 px cap inside it.
    /// `None` when the model's host/layout selection is the wide Desktop
    /// Ledger (desktop at 720 px and above), which this extension never
    /// restyles.
    pub fn geometry(host: crate::desktop::HostKind, width: f32) -> Option<Geometry> {
        use crate::desktop::{layout_mode, LayoutMode};
        if layout_mode(host, width) != LayoutMode::Phone {
            return None;
        }
        let content = (width - 2.0 * SPACE_4).clamp(0.0, CONTENT_MAX);
        let gutter = (width - content) / 2.0;
        Some(Geometry {
            gutter,
            content,
            form: content.min(FORM_MAX),
        })
    }

    fn token(value: ColorToken, dark: bool) -> Rgb {
        parse_color(if dark { value.dark } else { value.light })
    }

    /// Parses a token fill, keeping an `rgba(...)` alpha.
    fn parse_with_alpha(value: &str) -> (Rgb, f32) {
        let value = value.trim();
        if let Some(inner) = value
            .strip_prefix("rgba(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let mut parts = inner.split(',');
            let channel = |s: Option<&str>| {
                s.and_then(|s| s.trim().parse::<f32>().ok())
                    .unwrap_or(0.0)
                    .clamp(0.0, 255.0) as u8
            };
            let color = Rgb::new(
                channel(parts.next()),
                channel(parts.next()),
                channel(parts.next()),
            );
            let alpha = parts
                .next()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            return (color, alpha);
        }
        (parse_color(value), 1.0)
    }

    /// Composites a token fill over an actual opaque backing fill, so contrast
    /// is measured on what the screen paints, not on the translucent value.
    pub fn composite_over(fill: &str, backdrop: Rgb) -> Rgb {
        let (color, alpha) = parse_with_alpha(fill);
        if alpha >= 1.0 {
            return color;
        }
        mix_toward(backdrop, color, alpha)
    }

    fn at_least(fill: Rgb, preferred: Rgb, min: f32, dark: bool) -> Rgb {
        if contrast_ratio(preferred, fill) >= min {
            preferred
        } else {
            token(TEXT_PRIMARY, dark)
        }
    }

    /// `mobile.focusInk`: the pinned primary text ink, tested at 3:1 against
    /// the actual adjacent fills.
    pub fn focus_ink(dark: bool) -> Rgb {
        token(TEXT_PRIMARY, dark)
    }

    /// `mobile.secondaryInk(fill)`: pinned secondary ink while it reads at
    /// 4.5:1 on the actual composited fill, otherwise primary ink.
    pub fn secondary_ink(fill: Rgb, dark: bool) -> Rgb {
        at_least(fill, token(TEXT_SECONDARY, dark), TEXT_MIN_CONTRAST, dark)
    }

    /// `mobile.statusInk(fill, preferred)`: an existing semantic ink while it
    /// reads at 4.5:1, otherwise primary ink. No new semantic hue is invented.
    pub fn status_ink(fill: Rgb, preferred: Rgb, dark: bool) -> Rgb {
        at_least(fill, preferred, TEXT_MIN_CONTRAST, dark)
    }

    /// `mobile.dangerInk(fill)`: pinned error ink while it reads at 4.5:1.
    pub fn danger_ink(fill: Rgb, dark: bool) -> Rgb {
        at_least(fill, token(ERROR_TEXT, dark), TEXT_MIN_CONTRAST, dark)
    }

    /// `mobile.actionFill`: the selected accent's `primaryDark` in light
    /// appearance, its `primary` in dark; the pinned values do not change.
    pub fn action_fill(accent: &AccentColors, dark: bool) -> Rgb {
        parse_color(if dark {
            accent.primary
        } else {
            accent.primary_dark
        })
    }

    /// `mobile.actionInk(fill)`: existing white or black, whichever reads
    /// better on this fill.
    pub fn action_ink(fill: Rgb) -> Rgb {
        let white = parse_color(SURFACE.light);
        let black = parse_color(BACKGROUND.dark);
        if contrast_ratio(white, fill) >= contrast_ratio(black, fill) {
            white
        } else {
            black
        }
    }

    /// The raw `mobile.selectionFill` token: iOS glass `GLASS_PRIMARY`, flat
    /// Android the accent's opaque `primarySurface`.
    pub fn selection_token(accent: &AccentColors, _dark: bool, flat: bool) -> &'static str {
        if flat {
            accent.primary_surface
        } else {
            accent.glass_primary
        }
    }

    /// `mobile.selectionFill` composited over its actual backing fill.
    pub fn selection_fill(accent: &AccentColors, dark: bool, flat: bool, backdrop: Rgb) -> Rgb {
        composite_over(selection_token(accent, dark, flat), backdrop)
    }

    /// `mobile.selectionInk`: pinned primary text ink.
    pub fn selection_ink(dark: bool) -> Rgb {
        token(TEXT_PRIMARY, dark)
    }

    /// `mobile.selectionBoundary`: the accent primary while it reaches 3:1
    /// against the selection fill **and** its neighbouring fill, otherwise
    /// primary text ink.
    pub fn selection_boundary(accent: &AccentColors, dark: bool, flat: bool, backdrop: Rgb) -> Rgb {
        let fill = selection_fill(accent, dark, flat, backdrop);
        let preferred = parse_color(accent.primary);
        if contrast_ratio(preferred, fill) >= CONTROL_MIN_CONTRAST
            && contrast_ratio(preferred, backdrop) >= CONTROL_MIN_CONTRAST
        {
            preferred
        } else {
            token(TEXT_PRIMARY, dark)
        }
    }

    /// `mobile.pressedFill(fill)`: 14 % `textPrimary` mixed into the resting
    /// fill. Callers recompute readable ink with the helpers above.
    pub fn pressed_fill(fill: Rgb, dark: bool) -> Rgb {
        mix_toward(fill, token(TEXT_PRIMARY, dark), PRESSED_MIX)
    }

    /// `mobile.disabledFill`: the pinned surface variant.
    pub fn disabled_fill(dark: bool) -> Rgb {
        token(SURFACE_VARIANT, dark)
    }

    /// `mobile.disabledInk`: readable secondary projection on that fill.
    pub fn disabled_ink(dark: bool) -> Rgb {
        secondary_ink(disabled_fill(dark), dark)
    }

    /// Semantic inks available to [`status_ink`] before its fallback.
    pub fn semantic_status_ink(kind: &str, dark: bool) -> Rgb {
        let value = match kind {
            "success" => SUCCESS_TEXT,
            "warning" => WARNING_TEXT,
            _ => ERROR_TEXT,
        };
        token(value, dark)
    }
}
