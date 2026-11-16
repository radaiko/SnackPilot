//! The one flat design system: colour, spacing, radius, target and type
//! tokens as plain data (no UI framework here).
//!
//! Light and dark values are pinned in `docs/design/DESIGN.md`; the token
//! tests check the table and its WCAG floors. The app layer parses the colour
//! strings into framework colours; this crate stays testable without gpui.

/// A colour with a light- and dark-appearance value.
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

// Surfaces.
pub const BG: ColorToken = ColorToken::both("#E7E9EE", "#121316");
pub const SURFACE: ColorToken = ColorToken::both("#FFFFFF", "#1B1D21");
pub const SURFACE_ALT: ColorToken = ColorToken::both("#F1F3F6", "#24272C");
pub const BORDER: ColorToken = ColorToken::both("#DDE0E6", "#33373D");
pub const BORDER_STRONG: ColorToken = ColorToken::both("#C3C8D0", "#454A52");

// Text.
pub const TEXT: ColorToken = ColorToken::both("#14161A", "#F2F4F7");
pub const TEXT_SECONDARY: ColorToken = ColorToken::both("#5B6270", "#A7AEBA");
pub const TEXT_TERTIARY: ColorToken = ColorToken::both("#7C8492", "#6E7683");

// One accent, fixed: it is never selectable.
pub const ACCENT: ColorToken = ColorToken::both("#B8400E", "#FF7A45");
pub const ACCENT_SURFACE: ColorToken = ColorToken::both("#FDEDE6", "#3A1E12");
pub const ON_ACCENT: ColorToken = ColorToken::both("#FFFFFF", "#1A0D06");

// Status.
pub const SUCCESS: ColorToken = ColorToken::both("#1E7D43", "#4CC38A");
pub const SUCCESS_SURFACE: ColorToken = ColorToken::both("#E6F4EB", "#16301F");
pub const WARNING: ColorToken = ColorToken::both("#A35C00", "#F0A93B");
pub const WARNING_SURFACE: ColorToken = ColorToken::both("#FBF0DF", "#33260E");
pub const ERROR: ColorToken = ColorToken::both("#B3261E", "#F27B6B");
pub const ERROR_SURFACE: ColorToken = ColorToken::both("#FBEAE8", "#351A17");

/// Dialog and loading scrim: 40 % black in both appearances.
pub const SCRIM: ColorToken = ColorToken::fixed("rgba(0,0,0,0.4)");

/// One selectable palette: a full surface/ink/accent set for one appearance.
///
/// Ids are persisted in `dark_theme`/`light_theme`, so they never change.
/// `snackpilot` is the app's own palette and the default on both sides; the
/// others are the editor themes people already know, each carrying the accent
/// that makes it recognisable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub dark: bool,
    pub background: &'static str,
    pub surface: &'static str,
    pub surface_alt: &'static str,
    pub border: &'static str,
    pub border_strong: &'static str,
    pub text: &'static str,
    pub text_secondary: &'static str,
    pub text_tertiary: &'static str,
    pub accent: &'static str,
    pub accent_surface: &'static str,
    pub on_accent: &'static str,
}

/// The preset each appearance starts on and falls back to.
pub const DEFAULT_DARK: &str = "snackpilot";
pub const DEFAULT_LIGHT: &str = "snackpilot";

pub const PRESETS: [Preset; 10] = [
    Preset {
        id: "snackpilot",
        label: "SnackPilot",
        dark: false,
        background: "#E7E9EE",
        surface: "#FFFFFF",
        surface_alt: "#F1F3F6",
        border: "#DDE0E6",
        border_strong: "#C3C8D0",
        text: "#14161A",
        text_secondary: "#5B6270",
        text_tertiary: "#7C8492",
        accent: "#B8400E",
        accent_surface: "#FDEDE6",
        on_accent: "#FFFFFF",
    },
    Preset {
        id: "snackpilot",
        label: "SnackPilot",
        dark: true,
        background: "#121316",
        surface: "#1B1D21",
        surface_alt: "#24272C",
        border: "#33373D",
        border_strong: "#454A52",
        text: "#F2F4F7",
        text_secondary: "#A7AEBA",
        text_tertiary: "#6E7683",
        accent: "#FF7A45",
        accent_surface: "#3A1E12",
        on_accent: "#1A0D06",
    },
    Preset {
        id: "one-dark",
        label: "One Dark",
        dark: true,
        background: "#282C34",
        surface: "#21252B",
        surface_alt: "#2F343D",
        border: "#3E4451",
        border_strong: "#4B5263",
        text: "#C8CDD6",
        text_secondary: "#9AA1AD",
        text_tertiary: "#7A8290",
        accent: "#61AFEF",
        accent_surface: "#243342",
        on_accent: "#10151A",
    },
    Preset {
        id: "dracula",
        label: "Dracula",
        dark: true,
        background: "#282A36",
        surface: "#21222C",
        surface_alt: "#343746",
        border: "#44475A",
        border_strong: "#4E5166",
        text: "#F8F8F2",
        text_secondary: "#B9BDD0",
        text_tertiary: "#8B90A6",
        accent: "#BD93F9",
        accent_surface: "#37304A",
        on_accent: "#1A1523",
    },
    Preset {
        id: "nord",
        label: "Nord",
        dark: true,
        background: "#2E3440",
        surface: "#272C35",
        surface_alt: "#3B4252",
        border: "#434C5E",
        border_strong: "#4C566A",
        text: "#ECEFF4",
        text_secondary: "#C2CAD6",
        text_tertiary: "#94A0B2",
        accent: "#88C0D0",
        accent_surface: "#2B3D45",
        on_accent: "#10222A",
    },
    Preset {
        id: "monokai",
        label: "Monokai",
        dark: true,
        background: "#272822",
        surface: "#1E1F1A",
        surface_alt: "#31322B",
        border: "#44453C",
        border_strong: "#56574C",
        text: "#F8F8F2",
        text_secondary: "#C3C4BA",
        text_tertiary: "#9C9D90",
        accent: "#F92672",
        accent_surface: "#3B1D2A",
        on_accent: "#1A0D12",
    },
    Preset {
        id: "github-dark",
        label: "GitHub Dark",
        dark: true,
        background: "#0D1117",
        surface: "#161B22",
        surface_alt: "#1F2630",
        border: "#30363D",
        border_strong: "#484F58",
        text: "#E6EDF3",
        text_secondary: "#A9B4C0",
        text_tertiary: "#7D8590",
        accent: "#58A6FF",
        accent_surface: "#132539",
        on_accent: "#0B1220",
    },
    Preset {
        id: "solarized-dark",
        label: "Solarized Dark",
        dark: true,
        background: "#002B36",
        surface: "#073642",
        surface_alt: "#0B3C49",
        border: "#17414C",
        border_strong: "#25505C",
        text: "#EEE8D5",
        text_secondary: "#A6B5B4",
        text_tertiary: "#839496",
        accent: "#B58900",
        accent_surface: "#2E2913",
        on_accent: "#131007",
    },
    Preset {
        id: "github-light",
        label: "GitHub Light",
        dark: false,
        background: "#F6F8FA",
        surface: "#FFFFFF",
        surface_alt: "#EFF2F5",
        border: "#D0D7DE",
        border_strong: "#AFB8C1",
        text: "#1F2328",
        text_secondary: "#59636E",
        text_tertiary: "#6E7781",
        accent: "#0969DA",
        accent_surface: "#E6EEFB",
        on_accent: "#FFFFFF",
    },
    Preset {
        id: "solarized-light",
        label: "Solarized Light",
        dark: false,
        background: "#FDF6E3",
        surface: "#FFFBF0",
        surface_alt: "#F1EAD6",
        border: "#E0D9C3",
        border_strong: "#C9C2AC",
        text: "#073642",
        text_secondary: "#586E75",
        text_tertiary: "#657B83",
        accent: "#268BD2",
        accent_surface: "#E3EEF7",
        on_accent: "#041E28",
    },
];

/// The preset under `id` for one appearance. An unknown id — a stale or
/// hand-edited key — is the default for that appearance.
pub fn preset(id: &str, dark: bool) -> &'static Preset {
    PRESETS
        .iter()
        .find(|p| p.dark == dark && p.id == id)
        .or_else(|| {
            let default = if dark { DEFAULT_DARK } else { DEFAULT_LIGHT };
            PRESETS.iter().find(|p| p.dark == dark && p.id == default)
        })
        .expect("every appearance has a default preset")
}

/// The presets of one appearance, in menu order (the default first).
pub fn presets(dark: bool) -> impl Iterator<Item = &'static Preset> {
    PRESETS.iter().filter(move |p| p.dark == dark)
}

// Spacing: 4, 8, 12, 16, 24, 32.
pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 24.0;
pub const SPACE_6: f32 = 32.0;

/// Centred single-column content cap.
pub const CONTENT_MAX: f32 = 560.0;
/// Credential form cap inside that column.
pub const FORM_MAX: f32 = 440.0;
/// Two-axis touch target minimum.
pub const TARGET: f32 = 44.0;
/// Primary action height.
pub const ACTION_HEIGHT: f32 = 48.0;

/// Corner radii: card, control, chip.
pub const RADIUS_CARD: f32 = 12.0;
pub const RADIUS_CONTROL: f32 = 10.0;
pub const RADIUS_CHIP: f32 = 8.0;

/// Icon sizes and the one stroke width.
pub const ICON: f32 = 20.0;
pub const ICON_META: f32 = 18.0;
pub const STROKE: f32 = 2.0;

/// Focus ring stroke and offset.
pub const FOCUS_WIDTH: f32 = 2.0;
pub const FOCUS_GAP: f32 = 2.0;

/// One of the five text roles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeRole {
    pub size: f32,
    pub line_height: f32,
}

pub const TITLE: TypeRole = TypeRole {
    size: 26.0,
    line_height: 32.0,
};
pub const HEADLINE: TypeRole = TypeRole {
    size: 18.0,
    line_height: 24.0,
};
pub const BODY: TypeRole = TypeRole {
    size: 16.0,
    line_height: 22.0,
};
pub const BODY_STRONG: TypeRole = TypeRole {
    size: 16.0,
    line_height: 22.0,
};
pub const META: TypeRole = TypeRole {
    size: 12.0,
    line_height: 18.0,
};

/// WCAG AA thresholds the token tests check.
pub const TEXT_MIN_CONTRAST: f32 = 4.5;
pub const CONTROL_MIN_CONTRAST: f32 = 3.0;

/// Mobile column geometry at one host and logical viewport width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub gutter: f32,
    pub content: f32,
    pub form: f32,
}

/// The single-column projection: 16 px gutters up to the 560 px cap, then
/// centred gutters; the form is the 440 px cap inside it. `None` when the
/// model's host/layout selection is the wide desktop workspace (720 px and
/// above on a desktop host), which lays out wider than this column.
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

#[cfg(test)]
mod preset_tests {
    use super::*;
    use crate::desktop::{contrast_ratio, parse_color};

    fn ratio(a: &str, b: &str) -> f32 {
        contrast_ratio(parse_color(a), parse_color(b))
    }

    /// Every preset keeps the WCAG floors the app's own palette holds: body and
    /// secondary text readable on the field, the accent visible as a control,
    /// and the label on the accent readable. A theme that fails these is not
    /// shippable, whatever it looks like in its editor.
    #[test]
    fn every_preset_meets_the_contrast_floors() {
        for preset in PRESETS {
            let id = preset.id;
            assert!(
                ratio(preset.text, preset.background) >= TEXT_MIN_CONTRAST,
                "{id}: text on background {:.2}",
                ratio(preset.text, preset.background)
            );
            assert!(
                ratio(preset.text_secondary, preset.background) >= TEXT_MIN_CONTRAST,
                "{id}: secondary on background {:.2}",
                ratio(preset.text_secondary, preset.background)
            );
            assert!(
                ratio(preset.accent, preset.background) >= CONTROL_MIN_CONTRAST,
                "{id}: accent on background {:.2}",
                ratio(preset.accent, preset.background)
            );
            assert!(
                ratio(preset.on_accent, preset.accent) >= TEXT_MIN_CONTRAST,
                "{id}: on_accent on accent {:.2}",
                ratio(preset.on_accent, preset.accent)
            );
        }
    }

    /// The app's own palette is the default on both sides and is offered in
    /// each list, so a fresh install and a reset land on it.
    #[test]
    fn each_appearance_starts_on_the_default_preset() {
        assert_eq!(preset("", true).id, DEFAULT_DARK);
        assert_eq!(preset("", false).id, DEFAULT_LIGHT);
        assert!(presets(true).any(|p| p.id == DEFAULT_DARK));
        assert!(presets(false).any(|p| p.id == DEFAULT_LIGHT));
    }

    /// Ids are persisted, so an unknown one (stale key, hand edit) resolves to
    /// that appearance's default instead of a missing theme.
    #[test]
    fn an_unknown_id_falls_back_to_the_default() {
        assert_eq!(preset("no-such-theme", true).id, DEFAULT_DARK);
        assert_eq!(preset("no-such-theme", false).id, DEFAULT_LIGHT);
        // A light id never resolves in the dark list, and the other way round.
        assert_eq!(preset("github-light", true).id, DEFAULT_DARK);
        assert_eq!(preset("nord", false).id, DEFAULT_LIGHT);
    }

    /// The two lists are exactly the presets of that appearance, and no id
    /// appears twice within one list.
    #[test]
    fn the_lists_split_by_appearance_without_duplicates() {
        for dark in [true, false] {
            let list: Vec<_> = presets(dark).collect();
            assert!(!list.is_empty());
            assert!(list.iter().all(|p| p.dark == dark));
            let mut ids: Vec<_> = list.iter().map(|p| p.id).collect();
            ids.sort_unstable();
            let unique = ids.len();
            ids.dedup();
            assert_eq!(ids.len(), unique, "duplicate preset id in one list");
            assert_eq!(
                list.len(),
                PRESETS.iter().filter(|p| p.dark == dark).count()
            );
        }
    }
}
