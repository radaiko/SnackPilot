//! The one flat design system: the palette, scales and WCAG floors that
//! `docs/design/DESIGN.md` pins. No accent variants, no glass, no dialect.

use snackpilot_model::desktop::{contrast_ratio, parse_color};
use snackpilot_model::tokens::*;

/// The palette table: every token equals its pinned light/dark value.
#[test]
fn palette_matches_the_pinned_table() {
    let table: [(&str, ColorToken, &str, &str); 18] = [
        ("bg", BG, "#F4F5F7", "#121316"),
        ("surface", SURFACE, "#FFFFFF", "#1B1D21"),
        ("surface_alt", SURFACE_ALT, "#EDEFF3", "#24272C"),
        ("border", BORDER, "#DDE0E6", "#33373D"),
        ("border_strong", BORDER_STRONG, "#C3C8D0", "#454A52"),
        ("text", TEXT, "#14161A", "#F2F4F7"),
        ("text_secondary", TEXT_SECONDARY, "#5B6270", "#A7AEBA"),
        ("text_tertiary", TEXT_TERTIARY, "#7C8492", "#6E7683"),
        ("accent", ACCENT, "#C2450F", "#FF7A45"),
        ("accent_surface", ACCENT_SURFACE, "#FDEDE6", "#3A1E12"),
        ("on_accent", ON_ACCENT, "#FFFFFF", "#1A0D06"),
        ("success", SUCCESS, "#1E7D43", "#4CC38A"),
        ("success_surface", SUCCESS_SURFACE, "#E6F4EB", "#16301F"),
        ("warning", WARNING, "#A35C00", "#F0A93B"),
        ("warning_surface", WARNING_SURFACE, "#FBF0DF", "#33260E"),
        ("error", ERROR, "#B3261E", "#F27B6B"),
        ("error_surface", ERROR_SURFACE, "#FBEAE8", "#351A17"),
        ("scrim", SCRIM, "rgba(0,0,0,0.4)", "rgba(0,0,0,0.4)"),
    ];
    for (name, token, light, dark) in table {
        assert_eq!(token.light, light, "{name} light");
        assert_eq!(token.dark, dark, "{name} dark");
    }
}

/// The one scale: spacing, radii, targets, icons and type roles.
#[test]
fn scale_matches_the_pinned_table() {
    assert_eq!(
        [SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5, SPACE_6],
        [4., 8., 12., 16., 24., 32.]
    );
    assert_eq!([RADIUS_CARD, RADIUS_CONTROL, RADIUS_CHIP], [12., 10., 8.]);
    assert_eq!((TARGET, ACTION_HEIGHT), (44., 48.));
    assert_eq!([ICON, ICON_META, STROKE], [20., 18., 2.]);
    assert_eq!((FOCUS_WIDTH, FOCUS_GAP), (2., 2.));
    assert_eq!((CONTENT_MAX, FORM_MAX), (560., 440.));
    assert_eq!((TITLE.size, TITLE.line_height), (28., 36.));
    assert_eq!((HEADLINE.size, HEADLINE.line_height), (20., 28.));
    assert_eq!((BODY.size, BODY.line_height), (17., 26.));
    assert_eq!((BODY_STRONG.size, BODY_STRONG.line_height), (17., 26.));
    assert_eq!((META.size, META.line_height), (13., 20.));
}

fn token(t: ColorToken, dark: bool) -> snackpilot_model::desktop::Rgb {
    parse_color(if dark { t.dark } else { t.light })
}

/// Content text and the accent carry 4.5:1 on the surfaces they paint on,
/// light and dark; the muted metadata role keeps the 3:1 control floor.
#[test]
fn text_and_accent_meet_the_contrast_floors() {
    for dark in [false, true] {
        for surface in [
            token(BG, dark),
            token(SURFACE, dark),
            token(SURFACE_ALT, dark),
        ] {
            for text in [TEXT, TEXT_SECONDARY] {
                assert!(
                    contrast_ratio(token(text, dark), surface) >= TEXT_MIN_CONTRAST,
                    "{text:?} on {surface:?} (dark={dark})"
                );
            }
            assert!(
                contrast_ratio(token(TEXT_TERTIARY, dark), surface) >= CONTROL_MIN_CONTRAST,
                "tertiary metadata on {surface:?} (dark={dark})"
            );
        }
        // The accent is guaranteed as text on the plain background and surface.
        for surface in [token(BG, dark), token(SURFACE, dark)] {
            assert!(
                contrast_ratio(token(ACCENT, dark), surface) >= TEXT_MIN_CONTRAST,
                "accent text on {surface:?} (dark={dark})"
            );
        }
        assert!(
            contrast_ratio(token(ON_ACCENT, dark), token(ACCENT, dark)) >= TEXT_MIN_CONTRAST,
            "on_accent on the accent (dark={dark})"
        );
        assert!(
            contrast_ratio(token(TEXT, dark), token(ACCENT_SURFACE, dark)) >= TEXT_MIN_CONTRAST,
            "text on the accent surface (dark={dark})"
        );
    }
}

/// Status text reads 4.5:1 on its own toned surface, and its boundary reads
/// 3:1 on the plain surface in both appearances.
#[test]
fn status_colours_meet_their_floors() {
    for dark in [false, true] {
        for (name, ink, surface) in [
            ("success", SUCCESS, SUCCESS_SURFACE),
            ("warning", WARNING, WARNING_SURFACE),
            ("error", ERROR, ERROR_SURFACE),
        ] {
            assert!(
                contrast_ratio(token(ink, dark), token(surface, dark)) >= TEXT_MIN_CONTRAST,
                "{name} ink on its surface (dark={dark})"
            );
            assert!(
                contrast_ratio(token(ink, dark), token(SURFACE, dark)) >= CONTROL_MIN_CONTRAST,
                "{name} boundary on the surface (dark={dark})"
            );
        }
    }
}

/// Geometry: 16 px gutters up to the 560 px cap, the 440 px form cap, and the
/// wide desktop workspace opting out.
#[test]
fn single_column_geometry() {
    use snackpilot_model::desktop::HostKind;

    let cases = [
        (360., 328., 16., 328.),
        (390., 358., 16., 358.),
        (719., 560., 79.5, 440.),
        (768., 560., 104., 440.),
        (1280., 560., 360., 440.),
    ];
    for (width, content, gutter, form) in cases {
        for host in [HostKind::Phone, HostKind::Tablet] {
            let geometry = geometry(host, width).expect("phone column");
            assert_eq!(
                (geometry.content, geometry.gutter, geometry.form),
                (content, gutter, form)
            );
        }
    }
    for width in [720., 768., 1280.] {
        assert!(geometry(HostKind::Desktop, width).is_none());
    }
}
