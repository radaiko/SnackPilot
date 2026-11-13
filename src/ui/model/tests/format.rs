//! de-AT formatting (REQ-UI-06): the spec's pinned currency and date forms
//! (04-ui-ux.md:324-326, 654, 758-759). Pure functions, no host access.

use snackpilot_model::{
    day_key_from_epoch_ms, format_date_long, format_day_short, format_eur, format_time, today_key,
};

/// 2026-02-08 23:30 UTC = 2026-02-09 00:30 Europe/Vienna (a Monday): proves
/// the conversions run in Vienna, not UTC.
const MONDAY_EARLY: i64 = 1_770_593_400_000;
/// 2026-02-09 12:45 UTC = 13:45 Vienna.
const MONDAY_AFTERNOON: i64 = 1_770_641_100_000;

#[test]
/// @req REQ-UI-06
fn format_eur_uses_the_pinned_de_at_currency_form() {
    assert_eq!(format_eur(12.34), "€ 12,34", "04-ui-ux.md:324-326");
    assert_eq!(format_eur(0.0), "€ 0,00");
    assert_eq!(format_eur(1234.5), "€ 1.234,50", "dot thousands grouping");
    assert_eq!(format_eur(999999.99), "€ 999.999,99");
    assert_eq!(format_eur(-5.5), "-€ 5,50", "sign before the symbol");
    assert_eq!(format_eur(0.005), "€ 0,01", "half-cent rounds to the cent");
}

#[test]
/// @req REQ-UI-06
fn day_short_matches_the_day_navigator_form() {
    assert_eq!(format_day_short("2026-02-09"), "Mo., 9. Feb.");
    assert_eq!(format_day_short("2026-01-01"), "Do., 1. Jän.", "de-AT Jän.");
    assert_eq!(format_day_short("2026-03-01"), "So., 1. März");
    assert_eq!(
        format_day_short("not-a-date"),
        "not-a-date",
        "fallback is the raw key"
    );
}

#[test]
/// @req REQ-UI-06
fn bill_card_date_and_time_use_vienna_local_time() {
    assert_eq!(format_date_long(MONDAY_EARLY), "Mo., 9. Feb. 2026");
    assert_eq!(format_time(MONDAY_EARLY), "00:30");
    assert_eq!(format_date_long(MONDAY_AFTERNOON), "Mo., 9. Feb. 2026");
    assert_eq!(format_time(MONDAY_AFTERNOON), "13:45");
    assert_eq!(day_key_from_epoch_ms(MONDAY_EARLY), "2026-02-09");
}

#[test]
/// @req REQ-UI-06
fn mobile_billing_samples_keep_the_incumbent_formatter() {
    // Task 1.5 regression samples for the compact Abrechnung surface: full
    // de-AT dates and month names through the existing helpers, and the
    // currency form at zero, negative, large and sub-cent values. The
    // formatter source is untouched; these are the strings a receipt card and
    // the summary paint. Full month tab labels are `MonthOption.label` from the
    // core and are asserted in the billing suite.
    assert_eq!(format_date_long(1_768_474_800_000), "Do., 15. Jän. 2026");
    assert_eq!(format_time(1_768_474_800_000), "12:00");
    assert_eq!(format_date_long(1_772_439_300_000), "Mo., 2. März 2026");
    assert_eq!(format_date_long(1_798_756_200_000), "Do., 31. Dez. 2026");
    assert_eq!(format_time(1_798_756_200_000), "23:30");
    assert_eq!(format_date_long(1_783_159_200_000), "Sa., 4. Juli 2026");

    assert_eq!(format_eur(0.0), "€ 0,00", "an empty month stays zero");
    assert_eq!(
        format_eur(-1_234.5),
        "-€ 1.234,50",
        "a credit keeps its sign"
    );
    assert_eq!(format_eur(1_234_567.89), "€ 1.234.567,89", "large amount");
    assert_eq!(format_eur(-0.004), "€ 0,00", "sub-cent rounds to zero");
}

#[test]
/// @req REQ-UI-06
fn today_key_is_a_vienna_day_key() {
    let key = today_key();
    assert!(
        key.len() == 10
            && key.bytes().enumerate().all(|(i, b)| if i == 4 || i == 7 {
                b == b'-'
            } else {
                b.is_ascii_digit()
            }),
        "today_key must be YYYY-MM-DD, got {key}"
    );
}
