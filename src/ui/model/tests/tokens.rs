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
