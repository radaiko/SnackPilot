//! Menus tab (REQ-SCR-01): cached-first loading, day navigation, pending cart,
//! submit with progress, category ordering, dish cross-reference. Demo mode
//! only — no test reaches a real host.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use snackpilot_core::domain::{MenuCategory, MenuItem, MenuSnapshot, OrderProgress};
use snackpilot_model::{
    category_heading, category_rank, nearest_day, progress_label, AppModel, FakePlatform,
};

const DEMO_USER: &str = "demo";
const DEMO_PASS: &str = "demo1234!";

fn temp_dir(tag: &str) -> PathBuf {
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "snackpilot-model-menus-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn demo_model_in(dir: PathBuf) -> (AppModel, Arc<FakePlatform>) {
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(dir, fake.clone()).unwrap();
    model.login(DEMO_USER, DEMO_PASS).unwrap();
    (model, fake)
}

fn demo_model() -> (AppModel, Arc<FakePlatform>) {
    demo_model_in(temp_dir("login"))
}

fn snapshot(model: &AppModel) -> MenuSnapshot {
    model.state().menu_snapshot.clone().expect("menus loaded")
}

fn items_for(model: &AppModel, day: &str) -> Vec<MenuItem> {
    snapshot(model)
        .items
        .into_iter()
        .filter(|i| i.day == day)
        .collect()
}

/// The first menu item of `day` in the given category.
fn item_in(model: &AppModel, day: &str, category: MenuCategory) -> MenuItem {
    items_for(model, day)
        .into_iter()
        .find(|i| i.category == category)
        .unwrap_or_else(|| panic!("no {} item on {day}", category.display()))
}

#[test]
/// @req REQ-SCR-09
fn login_publishes_menus_and_syncs_selected_day() {
    let (model, _fake) = demo_model();
    let state = model.state();
    let snap = state.menu_snapshot.clone().expect("menus after login");
    assert!(!snap.available_dates.is_empty(), "demo offers days");
    assert_eq!(
        state.selected_day,
        Some(nearest_day(&snap.available_dates)),
        "login syncs the selected day to the nearest available day"
    );
    let day = state.selected_day.unwrap();
    assert!(!items_for(&model, &day).is_empty(), "the day has menus");
}

#[test]
/// @req REQ-SCR-01
fn menus_are_cached_to_disk_and_served_without_a_session() {
    let dir = temp_dir("cached-first");
    let (model, _fake) = demo_model_in(dir.clone());
    let cached_days = snapshot(&model).available_dates;
    drop(model);

    // A fresh core over the same storage serves the menus from the cache with
    // no login and no fetch (loadSession's force=false path is backed by this).
    let fake = Arc::new(FakePlatform::new());
    let model = AppModel::new(dir, fake).unwrap();
    model.core().load_cached_menus();
    let snap = model.core().menu_snapshot();
    assert_eq!(
        snap.available_dates, cached_days,
        "the on-disk cache replays the fetched days without a fetch"
    );
    assert!(
        model.state().menu_snapshot.is_none(),
        "hub untouched by the cache load"
    );
}

#[test]
/// @req REQ-SCR-01
fn day_navigation_walks_and_clamps() {
    let (model, _fake) = demo_model();
    let dates = snapshot(&model).available_dates;
    let start = nearest_day(&dates);
    model.select_day(&start);
    let idx = dates.iter().position(|d| *d == start).unwrap();

    model.next_day();
    assert_eq!(
        model.state().selected_day.as_deref(),
        Some(dates[idx + 1].as_str())
    );
    model.prev_day();
    assert_eq!(model.state().selected_day.as_deref(), Some(start.as_str()));

    model.select_day(&dates[0]);
    model.prev_day();
    assert_eq!(
        model.state().selected_day.as_deref(),
        Some(dates[0].as_str()),
        "prev_day clamps at the first day"
    );

    model.select_day(dates.last().unwrap());
    model.next_day();
    assert_eq!(
        model.state().selected_day.as_deref(),
        dates.last().map(String::as_str),
        "next_day clamps at the last day"
    );

    model.select_day(dates.last().unwrap());
    model.go_to_today();
    assert_eq!(
        model.state().selected_day.as_deref(),
        Some(start.as_str()),
        "go_to_today jumps to the nearest day"
    );
}

#[test]
/// @req REQ-SCR-01
fn toggle_tracks_pending_and_clear_pending_resets() {
    let (model, _fake) = demo_model();
    let day = model.state().selected_day.unwrap();
    let item = item_in(&model, &day, MenuCategory::Menu1);

    model.toggle(&item);
    let snap = snapshot(&model);
    assert!(model.has_pending_changes(), "toggle queues an order");
    assert_eq!(snap.pending_orders.len(), 1);
    assert!(model.state().menu_snapshot.unwrap().pending_orders[0].contains(&item.id));

    model.toggle(&item);
    assert!(!model.has_pending_changes(), "toggling again un-queues it");
    assert!(!model
        .state()
        .menu_snapshot
        .unwrap()
        .pending_orders
        .contains(&snap.pending_orders[0]));

    model.toggle(&item);
    assert!(model.has_pending_changes());
    model.clear_pending();
    let snap = snapshot(&model);
    assert!(
        snap.pending_orders.is_empty() && snap.pending_cancellations.is_empty(),
        "clear_pending empties both queues"
    );
    assert!(!model.has_pending_changes());
}

#[test]
/// @req REQ-SCR-01
fn fab_label_follows_the_pending_queues() {
    let (model, _fake) = demo_model();
    let day = model.state().selected_day.unwrap();
    assert_eq!(model.fab_label(), "Bestellen (0)", "idle label");

    let a = item_in(&model, &day, MenuCategory::Menu1);
    model.toggle(&a);
    assert_eq!(model.fab_label(), "Bestellen (1)");
    let b = item_in(&model, &day, MenuCategory::Menu2);
    model.toggle(&b);
    assert_eq!(model.fab_label(), "Bestellen (2)");

    model.submit_orders();
    let item = item_in(&model, &day, MenuCategory::Menu1);
    model.toggle(&item);
    assert_eq!(
        snapshot(&model).pending_cancellations.len(),
        1,
        "toggling an ordered item queues a cancellation"
    );
    assert_eq!(model.fab_label(), "Stornieren (1)", "cancellations only");

    let other = item_in(&model, &day, MenuCategory::Menu3);
    model.toggle(&other);
    assert_eq!(
        model.fab_label(),
        "Änderungen bestätigen (2)",
        "both queues confirm together"
    );
}

#[test]
/// @req REQ-SCR-01
fn categories_are_ordered_and_soup_heading_suppressed() {
    let (model, _fake) = demo_model();
    let day = model.state().selected_day.unwrap();
    let groups = model.categories_for_day(&day);
    let order: Vec<MenuCategory> = groups.iter().map(|(c, _)| *c).collect();
    assert_eq!(
        order,
        vec![
            MenuCategory::Menu1,
            MenuCategory::Menu2,
            MenuCategory::Menu3,
            MenuCategory::SoupAndSalad,
        ],
        "fixed category order, empty groups omitted"
    );
    assert_eq!(
        category_heading(MenuCategory::SoupAndSalad),
        None,
        "SUPPE & SALAT hidden"
    );
    assert_eq!(category_heading(MenuCategory::Menu1), Some("MENÜ I"));

    use MenuCategory as C;
    assert!(
        category_rank(C::Menu1) < category_rank(C::Menu2)
            && category_rank(C::Menu2) < category_rank(C::Menu3)
            && category_rank(C::Menu3) < category_rank(C::SoupAndSalad)
            && category_rank(C::SoupAndSalad) < category_rank(C::Unknown),
        "MENÜ I … UNKNOWN ordering"
    );
}

#[test]
/// @req REQ-SCR-01
fn submit_reports_progress_and_orders_the_item() {
    let (model, _fake) = demo_model();
    let day = model.state().selected_day.unwrap();
    let item = item_in(&model, &day, MenuCategory::Menu1);

    let phases = Arc::new(Mutex::new(Vec::<String>::new()));
    model.subscribe(Box::new({
        let phases = phases.clone();
        move |s: &snackpilot_model::State| {
            if let Some(p) = &s.order_progress {
                phases.lock().unwrap().push(p.clone());
            }
        }
    }));

    model.toggle(&item);
    model.submit_orders();

    let got = phases.lock().unwrap();
    assert!(
        got.iter().any(|l| l == "Wird in den Warenkorb gelegt..."),
        "cart phase reported: {got:?}"
    );
    assert!(
        got.iter().any(|l| l == "Bestellung wird bestätigt..."),
        "confirm phase reported: {got:?}"
    );
    drop(got);
    assert!(
        model.state().order_progress.is_none(),
        "progress cleared after submit"
    );

    let split = model.state().orders_split.clone().expect("orders loaded");
    assert_eq!(split.upcoming.len(), 1, "the order landed in upcoming");
    let order = &split.upcoming[0];
    assert!(model.is_ordered(&item), "the menu item shows as ordered");
    assert_eq!(
        model.dish(order).as_deref(),
        Some(item.subtitle.as_str()),
        "the order's dish comes from the matching menu item"
    );
}

#[test]
/// @req REQ-SCR-01
fn is_ordered_cross_references_the_orders_list() {
    let (model, _fake) = demo_model();
    let day = model.state().selected_day.unwrap();
    let item = item_in(&model, &day, MenuCategory::Menu2);
    assert!(!model.is_ordered(&item), "not ordered before submit");

    model.toggle(&item);
    model.submit_orders();
    assert!(
        model.is_ordered(&item),
        "ordered after submit via the orders list"
    );

    let other_day = snapshot(&model)
        .available_dates
        .iter()
        .find(|d| **d != day)
        .cloned()
        .unwrap();
    let same_title = MenuItem {
        day: other_day,
        ..item.clone()
    };
    assert!(
        !model.is_ordered(&same_title),
        "the cross-reference is per day"
    );
}

#[test]
/// @req REQ-SCR-01
fn cutoff_is_true_only_past_the_day() {
    let (model, _fake) = demo_model();
    assert!(model.is_cutoff("2020-01-01"), "a past day is closed");
    let dates = snapshot(&model).available_dates;
    assert!(
        !model.is_cutoff(dates.last().unwrap()),
        "a future day is still open"
    );
}

#[test]
/// @req REQ-UI-06
fn progress_labels_are_the_pinned_german_strings() {
    assert_eq!(
        progress_label(OrderProgress::Adding),
        "Wird in den Warenkorb gelegt..."
    );
    assert_eq!(
        progress_label(OrderProgress::Confirming),
        "Bestellung wird bestätigt..."
    );
    assert_eq!(
        progress_label(OrderProgress::Cancelling),
        "Bestellung wird storniert..."
    );
    assert_eq!(
        progress_label(OrderProgress::Refreshing),
        "Menüs werden aktualisiert..."
    );
}
