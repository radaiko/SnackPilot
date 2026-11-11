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

#[test]
/// @req REQ-SCR-03
fn billing_context_clears_hidden_preview() {
    let model = demo_model("hidden-preview");
    model.select_month(1);
    model.set_billing_source(BillingSource::All);

    // Preview an Automaten receipt, then filter it out of the visible list.
    let ventopay_id = model
        .merged_entries()
        .iter()
        .find(|e| e.id.starts_with("v-"))
        .map(|e| e.id.clone())
        .expect("demo month has an Automaten receipt");
    model.open_preview(ListKind::Receipts, &ventopay_id);
    model.set_cursor(ListKind::Receipts, Some(ventopay_id.clone()));

    model.set_billing_source(BillingSource::Gourmet);

    assert!(
        model.merged_entries().iter().all(|e| e.id != ventopay_id),
        "the Kantine filter must hide the Automaten receipt"
    );
    assert_eq!(
        model.preview(ListKind::Receipts),
        None,
        "a preview of a now-hidden receipt must be cleared, not kept"
    );
    assert_eq!(
        model.cursor(ListKind::Receipts).as_deref(),
        Some(ventopay_id.as_str()),
        "clearing the hidden preview keeps the cursor identity"
    );
}

#[test]
/// @req REQ-SCR-03
fn same_date_amount_receipts_keep_distinct_ids() {
    let model = demo_model("same-date-amount");
    let mut same_day_pairs = 0;

    for offset in 0..3u8 {
        model.select_month(offset);
        let entries = model.merged_entries();
        assert!(
            !entries.is_empty(),
            "demo month {offset} must have receipts"
        );

        // A merged month is newest-first and one row per receipt.
        let mut ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            entries.len(),
            "receipt IDs must be unique within a month"
        );

        // Two receipts of one day (a Kantine bill and an Automaten
        // transaction) must keep distinct identities even though they share
        // the day and may format to the same amount text. The demo's price
        // ranges (Gourmet >= 2,70 after the 1,50 subsidy, Automaten
        // 0,50–2,50) never coincide exactly, so the guard here is the
        // stronger one: identity never derives from date or amount. A
        // date/amount-keyed ID would collide for the next pair that does.
        let mut per_day: std::collections::HashMap<String, Vec<&str>> = Default::default();
        for e in &entries {
            per_day
                .entry(snackpilot_model::day_key_from_epoch_ms(e.epoch_ms))
                .or_default()
                .push(e.id.as_str());
        }
        let mut day_amount: std::collections::HashMap<(String, u64), Vec<&str>> =
            Default::default();
        for e in &entries {
            day_amount
                .entry((
                    snackpilot_model::day_key_from_epoch_ms(e.epoch_ms),
                    e.amount.to_bits(),
                ))
                .or_default()
                .push(e.id.as_str());
        }
        for group in day_amount.values() {
            assert_eq!(
                group.len(),
                1,
                "demo data gained an identical date+amount pair {group:?}; same-date \
                 receipts must still keep distinct IDs"
            );
        }
        for (day, group) in &per_day {
            if group.len() > 1 {
                same_day_pairs += 1;
                assert_eq!(
                    group.iter().collect::<std::collections::HashSet<_>>().len(),
                    group.len(),
                    "receipts on {day} must keep distinct IDs: {group:?}"
                );
            }
        }

        // Identity is the source's own receipt number, never the date or the
        // formatted amount: the merged list cannot lose a receipt to a
        // date/amount key.
        let gourmet = model.state().gourmet_month.clone().expect("Gourmet month");
        let ventopay = model
            .state()
            .ventopay_month
            .clone()
            .expect("Automaten month");
        for e in &entries {
            let day = snackpilot_model::day_key_from_epoch_ms(e.epoch_ms);
            let amount = snackpilot_model::format_eur(e.amount);
            assert!(
                !e.id.contains(&day) && !e.id.contains(&amount),
                "receipt ID {} must not encode date {day} or amount {amount}",
                e.id
            );
            if let Some(bill) = gourmet
                .bills
                .iter()
                .find(|b| e.id == format!("g-{}", b.bill_nr))
            {
                assert_eq!(bill.bill_date_epoch_ms, e.epoch_ms);
            } else if let Some(tx) = ventopay
                .transactions
                .iter()
                .find(|t| e.id == format!("v-{}", t.id))
            {
                assert_eq!(tx.date_epoch_ms, e.epoch_ms);
            } else {
                panic!("receipt ID {} is not a source-native identity", e.id);
            }
        }

        // Stable across a re-merge.
        let again: Vec<String> = model.merged_entries().into_iter().map(|e| e.id).collect();
        assert_eq!(
            entries.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            again,
            "receipt identities must stay stable across a re-merge"
        );
    }

    assert!(
        same_day_pairs > 0,
        "no demo month had receipts of both sources on one day; the \
         same-date case would be untested"
    );
}
