//! Token contract: every colour value in `tokens.rs` must equal the table in
//! `docs/requirements/04-ui-ux.md` §8, copied verbatim below.

use snackpilot_model::tokens::*;

/// §8 light-theme token table: (token name, light value, dark value).
/// Copied from docs/requirements/04-ui-ux.md §8.
const BASE_TABLE: [(&str, &str, &str); 31] = [
    ("background", "#F5F5F7", "#000000"),
    ("surface", "#fff", "#1C1C1E"),
    ("surfaceVariant", "#EDEDF0", "#2C2C2E"),
    ("textPrimary", "#1D1D1F", "#F5F5F7"),
    ("textSecondary", "#6E6E73", "#A1A1A6"),
    ("textTertiary", "#AEAEB2", "#636366"),
    ("primary", "#D4501A", "#FF6B35"),
    ("primaryDark", "#B84415", "#D4501A"),
    ("primarySurface", "#FFF1EB", "#2A1A10"),
    ("border", "#D6D6D8", "#38383A"),
    ("borderInput", "#CECED0", "#48484A"),
    ("success", "#34A853", "#34A853"),
    ("successSurface", "#EBF5EE", "#142018"),
    ("successText", "#1E7E34", "#5DB075"),
    ("successBorder", "#34A853", "#2E7D32"),
    ("warning", "#F5A623", "#F5A623"),
    ("warningSurface", "#FEF6E6", "#2A1E0E"),
    ("warningText", "#B57A10", "#F5C564"),
    ("warningBorder", "#F5C564", "#B57A10"),
    ("error", "#D93025", "#EA4335"),
    ("errorSurface", "#FCE8E6", "#2A1614"),
    ("errorText", "#B3261E", "#F28B82"),
    ("overlay", "rgba(255,255,255,0.7)", "rgba(0,0,0,0.7)"),
    (
        "glassSurface",
        "rgba(255,255,255,0.70)",
        "rgba(28,28,30,0.72)",
    ),
    (
        "glassSurfaceVariant",
        "rgba(237,237,240,0.60)",
        "rgba(44,44,46,0.60)",
    ),
    (
        "glassHighlight",
        "rgba(255,255,255,0.85)",
        "rgba(255,255,255,0.12)",
    ),
    ("glassShadowEdge", "rgba(0,0,0,0.06)", "rgba(0,0,0,0.40)"),
    (
        "glassSuccess",
        "rgba(52,168,83,0.10)",
        "rgba(52,168,83,0.16)",
    ),
    (
        "glassWarning",
        "rgba(245,166,35,0.10)",
        "rgba(245,166,35,0.16)",
    ),
    ("glassError", "rgba(217,48,37,0.10)", "rgba(234,67,53,0.16)"),
    (
        "glassPrimary",
        "rgba(212,80,26,0.08)",
        "rgba(255,107,53,0.14)",
    ),
];

fn base_tokens() -> [(&'static str, ColorToken); 31] {
    [
        ("background", BACKGROUND),
        ("surface", SURFACE),
        ("surfaceVariant", SURFACE_VARIANT),
        ("textPrimary", TEXT_PRIMARY),
        ("textSecondary", TEXT_SECONDARY),
        ("textTertiary", TEXT_TERTIARY),
        ("primary", PRIMARY),
        ("primaryDark", PRIMARY_DARK),
        ("primarySurface", PRIMARY_SURFACE),
        ("border", BORDER),
        ("borderInput", BORDER_INPUT),
        ("success", SUCCESS),
        ("successSurface", SUCCESS_SURFACE),
        ("successText", SUCCESS_TEXT),
        ("successBorder", SUCCESS_BORDER),
        ("warning", WARNING),
        ("warningSurface", WARNING_SURFACE),
        ("warningText", WARNING_TEXT),
        ("warningBorder", WARNING_BORDER),
        ("error", ERROR),
        ("errorSurface", ERROR_SURFACE),
        ("errorText", ERROR_TEXT),
        ("overlay", OVERLAY),
        ("glassSurface", GLASS_SURFACE),
        ("glassSurfaceVariant", GLASS_SURFACE_VARIANT),
        ("glassHighlight", GLASS_HIGHLIGHT),
        ("glassShadowEdge", GLASS_SHADOW_EDGE),
        ("glassSuccess", GLASS_SUCCESS),
        ("glassWarning", GLASS_WARNING),
        ("glassError", GLASS_ERROR),
        ("glassPrimary", GLASS_PRIMARY),
    ]
}

/// Every base colour token equals its §8 table row, light and dark.
/// @req REQ-UI-05
#[test]
fn base_tokens_match_spec_table() {
    let tokens = base_tokens();
    assert_eq!(tokens.len(), BASE_TABLE.len());
    for (name, light, dark) in BASE_TABLE {
        let (_, token) = tokens
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("token {name} missing from tokens.rs"));
        assert_eq!(token.light, light, "token {name} light value");
        assert_eq!(token.dark, dark, "token {name} dark value");
    }
}

/// §8 accent table: id, German label, then primary/primaryDark/primarySurface/
/// glassPrimary as light/dark pairs. Copied from docs/requirements/04-ui-ux.md §8.
const ACCENT_TABLE: [(&str, &str, [&str; 8]); 5] = [
    (
        "orange",
        "Orange",
        [
            "#D4501A",
            "#FF6B35",
            "#B84415",
            "#D4501A",
            "#FFF1EB",
            "#2A1A10",
            "rgba(212,80,26,0.08)",
            "rgba(255,107,53,0.14)",
        ],
    ),
    (
        "emerald",
        "Smaragd",
        [
            "#2E7D4F",
            "#4CAF7D",
            "#236B3F",
            "#2E7D4F",
            "#E8F5ED",
            "#102A1A",
            "rgba(46,125,79,0.08)",
            "rgba(76,175,125,0.14)",
        ],
    ),
    (
        "berry",
        "Beere",
        [
            "#A62547",
            "#E04868",
            "#8C1E3B",
            "#A62547",
            "#FCEEF2",
            "#2A1018",
            "rgba(166,37,71,0.08)",
            "rgba(224,72,104,0.14)",
        ],
    ),
    (
        "golden",
        "Gold",
        [
            "#C08B1A",
            "#E8B03E",
            "#A07415",
            "#C08B1A",
            "#FDF5E3",
            "#2A2210",
            "rgba(192,139,26,0.08)",
            "rgba(232,176,62,0.14)",
        ],
    ),
    (
        "ocean",
        "Ozean",
        [
            "#2563A8",
            "#4A90D9",
            "#1E528C",
            "#2563A8",
            "#EBF2FC",
            "#101A2A",
            "rgba(37,99,168,0.08)",
            "rgba(74,144,217,0.14)",
        ],
    ),
];

/// The five accents carry the §8 ids, German labels and all eight colour
/// values each, in settings-screen order.
/// @req REQ-UI-05
#[test]
fn accents_match_spec_table() {
    assert_eq!(ACCENTS.len(), ACCENT_TABLE.len());
    for (i, (id, label, values)) in ACCENT_TABLE.iter().enumerate() {
        let a = &ACCENTS[i];
        assert_eq!(a.id, *id, "accent {i} id");
        assert_eq!(a.label, *label, "accent {i} label (German, 04 §8)");
        let got = [
            a.light.primary,
            a.dark.primary,
            a.light.primary_dark,
            a.dark.primary_dark,
            a.light.primary_surface,
            a.dark.primary_surface,
            a.light.glass_primary,
            a.dark.glass_primary,
        ];
        assert_eq!(got, *values, "accent {id} colours");
    }
    // The German labels shown in settings, verbatim.
    let labels: Vec<&str> = ACCENTS.iter().map(|a| a.label).collect();
    assert_eq!(labels, ["Orange", "Smaragd", "Beere", "Gold", "Ozean"]);
}

/// §8 Android fallbacks: glass tokens resolve to these opaque values on
/// Android, and each accent's glassPrimary equals its primarySurface there.
/// @req REQ-UI-05
#[test]
fn android_glass_fallbacks_match_spec() {
    assert_eq!(ANDROID_GLASS_SURFACE.light, "#ffffff");
    assert_eq!(ANDROID_GLASS_SURFACE.dark, "#1C1C1E");
    assert_eq!(ANDROID_GLASS_SURFACE_VARIANT.light, "#EDEDF0");
    assert_eq!(ANDROID_GLASS_SURFACE_VARIANT.dark, "#2C2C2E");
    assert_eq!(ANDROID_GLASS_HIGHLIGHT.light, "#D6D6D8");
    assert_eq!(ANDROID_GLASS_HIGHLIGHT.dark, "#38383A");
    assert_eq!(ANDROID_GLASS_SHADOW_EDGE.light, "#D6D6D8");
    assert_eq!(ANDROID_GLASS_SHADOW_EDGE.dark, "#000000");
    for a in ACCENTS.iter() {
        assert_eq!(a.light.android_glass_primary(), a.light.primary_surface);
        assert_eq!(a.dark.android_glass_primary(), a.dark.primary_surface);
    }
}

/// Default (Orange) accent constants agree with the accent table, and the
/// accent lookup maps each core AccentColor to its record.
/// @req REQ-UI-05
#[test]
fn default_accent_constants_and_lookup() {
    assert_eq!(PRIMARY.light, ACCENTS[0].light.primary);
    assert_eq!(PRIMARY.dark, ACCENTS[0].dark.primary);
    assert_eq!(PRIMARY_DARK.light, ACCENTS[0].light.primary_dark);
    assert_eq!(PRIMARY_SURFACE.dark, ACCENTS[0].dark.primary_surface);
    use snackpilot_model::AccentColor;
    for (i, a) in [
        AccentColor::Orange,
        AccentColor::Emerald,
        AccentColor::Berry,
        AccentColor::Golden,
        AccentColor::Ocean,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(accent(*a), &ACCENTS[i], "accent lookup {i}");
    }
}

/// §8 blur material names and intensities.
/// @req REQ-UI-05
#[test]
fn blur_tokens_match_spec() {
    assert_eq!(BLUR_TINT_LIGHT, "systemThinMaterial");
    assert_eq!(BLUR_TINT_DARK, "systemThickMaterialDark");
    assert_eq!(BLUR_INTENSITY, (40, 50));
    assert_eq!(BLUR_INTENSITY_STRONG, (80, 90));
    assert_eq!(BLUR_INTENSITY_SUBTLE, (25, 30));
}

/// Desktop extension (desktop-ui-redesign D4): every derived desktop fill is
/// computed only from pinned tokens above — filled actions use
/// `primaryDark`/`primary`, destructive uses `error`, panel uses `surface`,
/// rail uses `background`, inks use `textPrimary` — and hover/press are the
/// fixed mixes toward `textPrimary`. No new colour literals enter the desktop
/// shell; the mobile tables above stay byte-identical.
/// @req REQ-UI-13
#[test]
fn desktop_extension_derives_from_pinned_tokens() {
    use snackpilot_model::desktop::{
        action_fill, action_hover_fill, action_pressed_fill, danger_fill, focus_ink, mix_toward,
        panel_fill, parse_color, rail_fill, text_ink, ACTION_HOVER_MIX, ACTION_PRESSED_MIX,
    };
    for dark in [false, true] {
        let ink = parse_color(if dark {
            TEXT_PRIMARY.dark
        } else {
            TEXT_PRIMARY.light
        });
        assert_eq!(text_ink(dark), ink, "text ink is the pinned textPrimary");
        assert_eq!(focus_ink(dark), ink, "focus ring is the pinned textPrimary");
        assert_eq!(
            panel_fill(dark),
            parse_color(if dark { SURFACE.dark } else { SURFACE.light }),
            "panel is the pinned surface"
        );
        assert_eq!(
            rail_fill(dark),
            parse_color(if dark {
                BACKGROUND.dark
            } else {
                BACKGROUND.light
            }),
            "rail is the pinned background"
        );
        assert_eq!(
            danger_fill(dark),
            parse_color(if dark { ERROR.dark } else { ERROR.light }),
            "destructive fill is the pinned error"
        );
        for a in ACCENTS.iter() {
            let fill = action_fill(if dark { &a.dark } else { &a.light }, dark);
            let expected = parse_color(if dark {
                a.dark.primary
            } else {
                a.light.primary_dark
            });
            assert_eq!(fill, expected, "{} filled action rest fill", a.id);
            assert_eq!(
                action_hover_fill(fill, ink),
                mix_toward(fill, ink, ACTION_HOVER_MIX),
                "{} hover is the 8 % mix",
                a.id
            );
            assert_eq!(
                action_pressed_fill(fill, ink),
                mix_toward(fill, ink, ACTION_PRESSED_MIX),
                "{} pressed is the 14 % mix",
                a.id
            );
        }
    }
}

/// Mobile Day Tray geometry (mobile-ui-rework design "Common mobile
/// contract"): 16 px gutters up to the 560 px content cap, the 440 px form
/// cap, and the model's host/layout selection deciding when the projection
/// applies at all. Phone/tablet stay single-column at every width; a desktop
/// host only gets it below the 720 px breakpoint.
/// @req REQ-UI-15
#[test]
fn mobile_day_tray_geometry() {
    use snackpilot_model::desktop::HostKind;
    use snackpilot_model::tokens::mobile as m;

    // (viewport, content, gutter, form)
    let mobile_cases = [
        (360., 328., 16., 328.),
        (390., 358., 16., 358.),
        (719., 560., 79.5, 440.),
        (720., 560., 80., 440.),
        (768., 560., 104., 440.),
        (1280., 560., 360., 440.),
    ];
    for (width, content, gutter, form) in mobile_cases {
        for host in [HostKind::Phone, HostKind::Tablet] {
            let geometry = m::geometry(host, width)
                .unwrap_or_else(|| panic!("{host:?} is always the mobile column at {width}"));
            assert_eq!(geometry.content, content, "{host:?} content at {width}");
            assert_eq!(geometry.gutter, gutter, "{host:?} gutter at {width}");
            assert_eq!(geometry.form, form, "{host:?} form cap at {width}");
        }
    }

    // A desktop host is the flat Day Tray only below 720; at the breakpoint
    // it is the wide Desktop Ledger, whose geometry this extension never
    // supplies (REQ-UI-07).
    for width in [360., 390., 719.] {
        let geometry = m::geometry(HostKind::Desktop, width)
            .unwrap_or_else(|| panic!("desktop below 720 uses the Day Tray column at {width}"));
        let phone = m::geometry(HostKind::Phone, width).expect("phone column");
        assert_eq!(geometry, phone, "same column at {width}");
    }
    for width in [720., 768., 1280.] {
        assert!(
            m::geometry(HostKind::Desktop, width).is_none(),
            "desktop at {width} keeps the wide Ledger, not the mobile projection"
        );
    }

    // The named scale and caps are exactly the shared table.
    assert_eq!(
        [
            m::SPACE_1,
            m::SPACE_2,
            m::SPACE_3,
            m::SPACE_4,
            m::SPACE_5,
            m::SPACE_6,
            m::SPACE_8
        ],
        [4., 8., 12., 16., 20., 24., 32.]
    );
    assert_eq!((m::CONTENT_MAX, m::FORM_MAX), (560., 440.));
    assert_eq!((m::TARGET, m::ACTION_HEIGHT), (44., 48.));
    assert_eq!(
        (m::TYPE_HEADLINE.size, m::TYPE_HEADLINE.line_height),
        (24., 32.)
    );
    assert_eq!((m::TYPE_BODY.size, m::TYPE_BODY.line_height), (16., 24.));
    assert_eq!((m::TYPE_META.size, m::TYPE_META.line_height), (14., 20.));
    assert_eq!((m::RADIUS_OPTION, m::RADIUS_STATUS), (8., 8.));
    assert_eq!((m::FOCUS_WIDTH, m::FOCUS_GAP), (2., 2.));
}

/// Mobile ink and boundary projections for every shared role over the actual
/// composited fills: all five accents, light/dark, glass/flat, rest and 14 %
/// pressed, plus semantic status, danger, disabled and focus. The derived
/// values are checked on the fill the screen really paints, so a translucent
/// selection is measured over its backing surface.
/// @req REQ-UI-15
#[test]
fn mobile_day_tray_contrast_all_accents_and_composited_fills() {
    use snackpilot_model::desktop::{contrast_ratio, parse_color};
    use snackpilot_model::tokens::mobile as m;
    use snackpilot_model::tokens::{
        BACKGROUND, ERROR, ERROR_SURFACE, ERROR_TEXT, GLASS_ERROR, GLASS_SUCCESS, GLASS_WARNING,
        SUCCESS_SURFACE, SUCCESS_TEXT, SURFACE, SURFACE_VARIANT, TEXT_PRIMARY, TEXT_SECONDARY,
        WARNING_SURFACE, WARNING_TEXT,
    };

    for dark in [false, true] {
        let surface = parse_color(if dark { SURFACE.dark } else { SURFACE.light });
        let background = parse_color(if dark {
            BACKGROUND.dark
        } else {
            BACKGROUND.light
        });
        let variant = parse_color(if dark {
            SURFACE_VARIANT.dark
        } else {
            SURFACE_VARIANT.light
        });

        // Readable secondary ink on every plain resting fill.
        for fill in [surface, background, variant] {
            assert!(
                contrast_ratio(m::secondary_ink(fill, dark), fill) >= m::TEXT_MIN_CONTRAST,
                "secondary ink on {fill:?} (dark={dark})"
            );
        }

        for accent in ACCENTS.iter() {
            let colors = if dark { &accent.dark } else { &accent.light };
            for flat in [false, true] {
                // Selection over both actual backings: the card surface and
                // the screen background.
                for backdrop in [surface, background] {
                    let fill = m::selection_fill(colors, dark, flat, backdrop);
                    assert!(
                        contrast_ratio(m::selection_ink(dark), fill) >= m::TEXT_MIN_CONTRAST,
                        "{} selection ink on composited fill (dark={dark}, flat={flat})",
                        accent.id
                    );
                    let boundary = m::selection_boundary(colors, dark, flat, backdrop);
                    assert!(
                        contrast_ratio(boundary, fill) >= m::CONTROL_MIN_CONTRAST,
                        "{} selection boundary on the fill (dark={dark}, flat={flat})",
                        accent.id
                    );
                    assert!(
                        contrast_ratio(boundary, backdrop) >= m::CONTROL_MIN_CONTRAST,
                        "{} selection boundary on the neighbouring fill (dark={dark}, flat={flat})",
                        accent.id
                    );
                }

                // Filled primary action and its 14 % pressed fill.
                let action = m::action_fill(colors, dark);
                assert!(
                    contrast_ratio(m::action_ink(action), action) >= m::TEXT_MIN_CONTRAST,
                    "{} action ink on the rest fill (dark={dark})",
                    accent.id
                );
                let pressed = m::pressed_fill(action, dark);
                assert!(
                    contrast_ratio(m::action_ink(pressed), pressed) >= m::TEXT_MIN_CONTRAST,
                    "{} action ink on the pressed fill (dark={dark})",
                    accent.id
                );
                // Secondary ink recomputed on the pressed surface fill.
                let pressed_surface = m::pressed_fill(surface, dark);
                assert!(
                    contrast_ratio(m::secondary_ink(pressed_surface, dark), pressed_surface)
                        >= m::TEXT_MIN_CONTRAST,
                    "{} secondary ink on the pressed surface (dark={dark})",
                    accent.id
                );
                // Focus ink stays independent of the accent at 3:1 on the
                // surfaces the reserved ring is painted over (never inside a
                // filled action: the 2 px gap keeps it off that fill).
                for adjacent in [surface, background, variant] {
                    assert!(
                        contrast_ratio(m::focus_ink(dark), adjacent) >= m::CONTROL_MIN_CONTRAST,
                        "{} focus ink on {adjacent:?} (dark={dark})",
                        accent.id
                    );
                }
            }
        }

        // Semantic status inks fall back to primary text when the pinned hue
        // cannot carry 4.5:1 on the actual (glass-composited) status fill.
        for (preferred, surface_token, glass) in [
            (SUCCESS_TEXT, SUCCESS_SURFACE, GLASS_SUCCESS),
            (WARNING_TEXT, WARNING_SURFACE, GLASS_WARNING),
            (ERROR_TEXT, ERROR_SURFACE, GLASS_ERROR),
        ] {
            let surface_fill = parse_color(if dark {
                surface_token.dark
            } else {
                surface_token.light
            });
            let composited =
                m::composite_over(if dark { glass.dark } else { glass.light }, surface_fill);
            for fill in [surface_fill, composited] {
                let ink = m::status_ink(
                    fill,
                    parse_color(if dark {
                        preferred.dark
                    } else {
                        preferred.light
                    }),
                    dark,
                );
                assert!(
                    contrast_ratio(ink, fill) >= m::TEXT_MIN_CONTRAST,
                    "status ink on {fill:?} (dark={dark})"
                );
            }
        }

        // Danger ink on the neutral/outline fills it is used on, and the
        // disabled context staying readable.
        let error_surface = parse_color(if dark {
            ERROR_SURFACE.dark
        } else {
            ERROR_SURFACE.light
        });
        for fill in [surface, variant, error_surface] {
            assert!(
                contrast_ratio(m::danger_ink(fill, dark), fill) >= m::TEXT_MIN_CONTRAST,
                "danger ink on a neutral or semantic surface (dark={dark})"
            );
        }
        // The pinned error fill itself only ever carries white/black action
        // ink; danger ink there falls back to primary text rather than
        // inventing a hue.
        let error = parse_color(if dark { ERROR.dark } else { ERROR.light });
        assert_eq!(
            m::danger_ink(error, dark),
            parse_color(if dark {
                TEXT_PRIMARY.dark
            } else {
                TEXT_PRIMARY.light
            }),
            "danger ink falls back to the pinned textPrimary, not a new hue"
        );
        let disabled = m::disabled_fill(dark);
        assert!(
            contrast_ratio(m::disabled_ink(dark), disabled) >= m::TEXT_MIN_CONTRAST,
            "disabled ink stays readable on its fill (dark={dark})"
        );
    }

    // The derived inks are projections of pinned tokens: no new colour value
    // is introduced when the preferred ink cannot be used.
    let dark = false;
    let surface = parse_color(SURFACE.light);
    assert_eq!(
        m::secondary_ink(surface, dark),
        parse_color(TEXT_SECONDARY.light),
        "readable secondary ink is the pinned textSecondary"
    );
    assert_eq!(
        m::focus_ink(dark),
        parse_color(TEXT_PRIMARY.light),
        "focus ink is the pinned textPrimary"
    );
    assert!(
        contrast_ratio(m::focus_ink(dark), surface) >= m::CONTROL_MIN_CONTRAST,
        "focus ink clears 3:1 on the resting surface"
    );
}

/// The mobile extension is append-only: the base token table and the five
/// accents above it stay byte-identical, so adding the Day Tray projection
/// cannot move a pinned colour or the desktop palette.
/// @req REQ-UI-15
#[test]
fn mobile_extension_preserves_base_values() {
    let tokens = base_tokens();
    for (name, light, dark) in BASE_TABLE {
        let (_, token) = tokens
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("token {name} missing after the mobile extension"));
        assert_eq!(token.light, light, "token {name} light value");
        assert_eq!(token.dark, dark, "token {name} dark value");
    }
    for (i, (id, label, values)) in ACCENT_TABLE.iter().enumerate() {
        let accent = &ACCENTS[i];
        assert_eq!(accent.id, *id, "accent {i} id");
        assert_eq!(accent.label, *label, "accent {i} label");
        let got = [
            accent.light.primary,
            accent.dark.primary,
            accent.light.primary_dark,
            accent.dark.primary_dark,
            accent.light.primary_surface,
            accent.dark.primary_surface,
            accent.light.glass_primary,
            accent.dark.glass_primary,
        ];
        assert_eq!(got, *values, "accent {id} colours");
    }
    assert_eq!(BASE_TABLE.len(), 31, "the pinned base table is unchanged");
}
