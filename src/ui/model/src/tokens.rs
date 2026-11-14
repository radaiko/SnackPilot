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
pub const BG: ColorToken = ColorToken::both("#F4F5F7", "#121316");
pub const SURFACE: ColorToken = ColorToken::both("#FFFFFF", "#1B1D21");
pub const SURFACE_ALT: ColorToken = ColorToken::both("#EDEFF3", "#24272C");
pub const BORDER: ColorToken = ColorToken::both("#DDE0E6", "#33373D");
pub const BORDER_STRONG: ColorToken = ColorToken::both("#C3C8D0", "#454A52");

// Text.
pub const TEXT: ColorToken = ColorToken::both("#14161A", "#F2F4F7");
pub const TEXT_SECONDARY: ColorToken = ColorToken::both("#5B6270", "#A7AEBA");
pub const TEXT_TERTIARY: ColorToken = ColorToken::both("#7C8492", "#6E7683");

// One accent, fixed: it is never selectable.
pub const ACCENT: ColorToken = ColorToken::both("#C2450F", "#FF7A45");
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
