//! Billing tab (REQ-SCR-03): month tabs, merged newest-first list, totals,
//! source chips and pull-to-refresh force. Demo mode only — no test reaches
//! a real host.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use snackpilot_model::navigation::ListKind;
use snackpilot_model::{AppModel, BillingSource, FakePlatform};

const DEMO_USER: &str = "demo";
const DEMO_PASS: &str = "demo1234!";

/// Enough wall-clock distance for `fetched_at` (ms) to differ between fetches.
const TICK: Duration = Duration::from_millis(15);

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-billing-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn demo_model(tag: &str) -> AppModel {
    let model = AppModel::new(temp_dir(tag), Arc::new(FakePlatform::new())).unwrap();
    model.login(DEMO_USER, DEMO_PASS).unwrap();
    model
}

#[test]
/// @req REQ-SCR-03
fn month_options_are_current_plus_two_past() {
    let model = demo_model("months");
    let options = &model.state().month_options;
    let offsets: Vec<u8> = options.iter().map(|m| m.offset).collect();
    assert_eq!(offsets, vec![0, 1, 2], "three month tabs, current + 2 past");
    for m in options {
        assert!(
            m.key.len() == 7
                && &m.key[4..5] == "-"
                && m.key.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "month key must be YYYY-MM, got {}",
            m.key
        );
        assert!(!m.label.is_empty(), "month tab needs a label");
    }
}

#[test]
/// @req REQ-SCR-03
fn select_month_merges_both_sources_newest_first() {
    let model = demo_model("merge");
    model.select_month(1);
    let state = model.state();
    assert!(state.gourmet_month.is_some(), "Kantine month not loaded");
    assert!(state.ventopay_month.is_some(), "Automaten month not loaded");

    let entries = model.merged_entries();
    assert!(!entries.is_empty(), "merged list empty for a demo month");
    assert!(
        entries.iter().any(|e| e.id.starts_with("g-")),
        "no Gourmet entries in the merged list"
    );
    assert!(
        entries.iter().any(|e| e.id.starts_with("v-")),
        "no Ventopay entries in the merged list"
    );
    assert!(
        entries.windows(2).all(|w| w[0].epoch_ms >= w[1].epoch_ms),
        "merged list is not sorted newest first"
    );
}

#[test]
/// @req REQ-SCR-03
fn totals_combine_both_sources() {
    let model = demo_model("totals");
    model.select_month(1);
    let state = model.state();
    let gourmet = state.gourmet_month.clone().unwrap();
    let ventopay = state.ventopay_month.clone().unwrap();

    let totals = model.billing_totals();
    assert!(
        (totals.gesamt - (gourmet.total_billing + ventopay.total)).abs() < 1e-6,
        "Gesamt must be the two source totals: {} vs {}+{}",
        totals.gesamt,
        gourmet.total_billing,
        ventopay.total
    );
    assert_eq!(
        totals.belege,
        model.merged_entries().len(),
        "Belege = entry count"
    );
    assert!(
        (totals.zuschuss - gourmet.total_subsidy).abs() < 1e-6,
        "Zuschuss comes from the Gourmet month"
    );
}

#[test]
/// @req REQ-SCR-03
fn source_filter_gates_entries_and_totals() {
    let model = demo_model("filter");
    model.select_month(1);
    let state = model.state();
    let gourmet = state.gourmet_month.clone().unwrap();
    let ventopay = state.ventopay_month.clone().unwrap();

    // Kantine only: no Automaten entries, and their share of the totals is zero.
    model.set_billing_source(BillingSource::Gourmet);
    let entries = model.merged_entries();
    assert!(
        entries.iter().all(|e| e.id.starts_with("g-")),
        "Kantine filter leaked Ventopay entries"
    );
    let totals = model.billing_totals();
    assert!(
        (totals.gesamt - gourmet.total_billing).abs() < 1e-6,
        "Kantine filter must zero the Automaten share: {}",
        totals.gesamt
    );

    // Automaten only: no bills, and the Kantine-only Zuschuss is zero.
    model.set_billing_source(BillingSource::Ventopay);
    let entries = model.merged_entries();
    assert!(
        entries.iter().all(|e| e.id.starts_with("v-")),
        "Automaten filter leaked Gourmet entries"
    );
    let totals = model.billing_totals();
    assert!(
        (totals.gesamt - ventopay.total).abs() < 1e-6,
        "Automaten filter must zero the Kantine share: {}",
        totals.gesamt
    );
    assert!(
        totals.zuschuss.abs() < 1e-6,
        "Zuschuss must be hidden with the Kantine side: {}",
        totals.zuschuss
    );

    model.set_billing_source(BillingSource::All);
    assert_eq!(
        model.merged_entries().len(),
        gourmet.bills.len() + ventopay.transactions.len(),
        "Alle shows both sides"
    );
}

#[test]
/// @req REQ-SCR-03
fn reload_forces_refetch_past_the_cache_skip() {
    let model = demo_model("reload");
    model.select_month(1);
    let first = model.state().gourmet_month.clone().unwrap();
    assert!(!first.bills.is_empty(), "demo past month must have bills");

    // Re-selecting a cached past month is served from cache (billing §4.1).
    std::thread::sleep(TICK);
    model.select_month(0);
    model.select_month(1);
    let cached = model.state().gourmet_month.clone().unwrap();
    assert_eq!(
        cached.fetched_at, first.fetched_at,
        "re-selecting a cached past month must not refetch"
    );

    // Pull-to-refresh forces the fetch and replaces the stale entry.
    std::thread::sleep(TICK);
    model.reload_billing();
    let refreshed = model.state().gourmet_month.clone().unwrap();
    assert!(
        refreshed.fetched_at > first.fetched_at,
        "reload_billing must force a refetch: {} vs {}",
        refreshed.fetched_at,
        first.fetched_at
    );
}

#[test]
/// @req REQ-SCR-03
fn empty_month_renders_no_entries_and_zero_totals() {
    // A fresh model has no month loaded and no source authenticated: the
    // Abrechnung tab must render empty without panicking.
    let model = AppModel::new(temp_dir("empty"), Arc::new(FakePlatform::new())).unwrap();
    assert!(
        model.merged_entries().is_empty(),
        "fresh model must show no entries"
    );
    let totals = model.billing_totals();
    assert_eq!(
        (totals.gesamt, totals.belege, totals.zuschuss),
        (0.0, 0, 0.0),
        "empty month must total to zero across the board"
    );

    // Logged in, then the shown side is removed: Ventopay logout clears its
    // month and auth, and with the Automaten filter on, the remaining view is
    // the same empty state — still zero, no panic.
    let model = demo_model("empty-filtered");
    model.ventopay_logout();
    model.set_billing_source(BillingSource::Ventopay);
    assert!(
        model.merged_entries().is_empty(),
        "a logged-out source must contribute no entries"
    );
    let totals = model.billing_totals();
    assert_eq!(
        (totals.gesamt, totals.belege, totals.zuschuss),
        (0.0, 0, 0.0),
        "the filtered-out view must total to zero"
    );
}

#[test]
/// @req REQ-SCR-03
fn receipt_identities_are_stable_and_context_change_clears_only_them() {
    let model = demo_model("receipt-identity");
    model.select_month(1);
    let first = model.merged_entries();
    assert!(!first.is_empty(), "demo month must have entries");

    // Newest-first, prefixed identities, stable across a re-merge.
    assert!(
        first.windows(2).all(|w| w[0].epoch_ms >= w[1].epoch_ms),
        "merged list is not sorted newest first"
    );
    for entry in &first {
        assert!(
            entry.id.starts_with("g-") || entry.id.starts_with("v-"),
            "unexpected receipt identity {}",
            entry.id
        );
    }
    let again = model.merged_entries();
    assert_eq!(
        first.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        again.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        "receipt identities must be stable across a re-merge"
    );

    // A context change clears only the receipts preview; the cursor stays.
    let id = first[0].id.clone();
    model.open_preview(ListKind::Receipts, &id);
    model.set_cursor(ListKind::Receipts, Some(id.clone()));
    model.set_billing_source(BillingSource::Gourmet);
    assert_eq!(
        model.preview(ListKind::Receipts),
        None,
        "a source change must clear the receipts preview"
    );
    assert_eq!(
        model.cursor(ListKind::Receipts).as_deref(),
        Some(id.as_str()),
        "a context change must keep the cursor"
    );

    // A month change clears the receipts preview the same way.
    model.set_billing_source(BillingSource::All);
    model.open_preview(ListKind::Receipts, &id);
    model.select_month(0);
    assert_eq!(
        model.preview(ListKind::Receipts),
        None,
        "a month change must clear the receipts preview"
    );
}
