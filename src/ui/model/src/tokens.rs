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
