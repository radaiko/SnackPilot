//! Background new-menu check algorithm (03-features/notifications-new-menu §3.3-§3.4).
//! Headless: logs into Gourmet, scrapes menus, diffs fingerprints, and returns whether to
//! fire the "new menus" notification (a latch prevents more than one per batch). Credentials
//! and the constructed `GourmetApi` are injected by the host (shell/facade); secure storage
//! and notification delivery are shell concerns.
use crate::datetime::Clock;
use crate::demo::is_demo_credentials;
use crate::domain::Credentials;
use crate::features::AnalyticsSink;
use crate::gourmet::api::GourmetApi;
use crate::notify::fingerprint::{
    compute_fingerprints, detect_new_menus, is_notification_sent, load_known_menus,
    save_known_menus, set_notification_sent,
};
use crate::notify::log::{append_log_entry, LogLevel, LogSubsystem};
use crate::notify::{MenuCheckOutcome, MenuCheckResult, NotificationCommand, MENU_UPDATES_CHANNEL};
use crate::storage::Kv;

const NEW_MENU_ID: &str = "new-menu-notification";
const TITLE: &str = "Neue Menüs verfügbar";
const BODY: &str = "Es gibt neue Menüs. Öffne SnackPilot um sie anzusehen.";

/// §3.3 — the exact task algorithm. `gourmet` is a fresh instance the host constructed;
/// `creds` come from secure storage (host). Logs go to the diagnostic log (no-op unless active).
pub async fn run_background_menu_check(
    gourmet: &GourmetApi,
    kv: &dyn Kv,
    clock: &dyn Clock,
    analytics: Option<&dyn AnalyticsSink>,
    creds: Option<Credentials>,
) -> MenuCheckResult {
    let log = |level: LogLevel, event: &str, detail: Option<&str>| {
        append_log_entry(kv, clock, LogSubsystem::MenuCheck, level, event, detail);
    };

    log(LogLevel::Info, "task_start", None);

    // 2. credentials required.
    let creds = match creds {
        Some(c) => c,
        None => {
            log(LogLevel::Guard, "no_credentials", None);
            return result(MenuCheckOutcome::NoCredentials);
        }
    };
    // 3. demo guard — never send demo creds to the live server.
    if is_demo_credentials(&creds.username, &creds.password) {
        log(LogLevel::Guard, "demo_credentials_skip", None);
        return result(MenuCheckOutcome::DemoSkipped);
    }

    // 4. login.
    log(LogLevel::Info, "login_start", None);
    if let Err(e) = gourmet.login(creds).await {
        log(LogLevel::Error, "task_error", Some(&e.to_string()));
        return result(MenuCheckOutcome::Failed);
    }
    log(LogLevel::Info, "login_success", None);

    // 5. scrape menus.
    let items = match gourmet.get_menus().await {
        Ok(i) => i,
        Err(e) => {
            log(LogLevel::Error, "task_error", Some(&e.to_string()));
            return result(MenuCheckOutcome::Failed);
        }
    };
    log(
        LogLevel::Info,
        "menus_fetched",
        Some(&format!("count={}", items.len())),
    );

    // 6. compare.
    let current = compute_fingerprints(&items);
    let known = load_known_menus(kv);
    let has_new = detect_new_menus(&current, &known);
    let already_sent = is_notification_sent(kv);
    log(
        LogLevel::Info,
        "comparison_result",
        Some(&format!(
            "hasNew={has_new} alreadySent={already_sent} currentCount={} knownCount={}",
            current.len(),
            known.len()
        )),
    );

    // 7. state machine.
    match (has_new, already_sent) {
        (true, false) => {
            let notification = NotificationCommand::FireNow {
                id: NEW_MENU_ID.to_string(),
                title: TITLE.to_string(),
                body: BODY.to_string(),
                channel_id: Some(MENU_UPDATES_CHANNEL.to_string()),
                screen: Some("/(tabs)".to_string()),
            };
            set_notification_sent(kv, true);
            save_known_menus(kv, &current);
            if let Some(a) = analytics {
                a.track("menu.newDetected", vec![]);
            }
            log(LogLevel::Notification, "fired", Some("new menus detected"));
            MenuCheckResult {
                outcome: MenuCheckOutcome::Notified,
                notification: Some(notification),
            }
        }
        (false, true) => {
            set_notification_sent(kv, false);
            save_known_menus(kv, &current);
            log(
                LogLevel::Info,
                "notification_flag_reset",
                Some("menus unchanged, reset for next batch"),
            );
            no_notification(&log, has_new, already_sent)
        }
        _ => no_notification(&log, has_new, already_sent),
    }
}

fn result(outcome: MenuCheckOutcome) -> MenuCheckResult {
    MenuCheckResult {
        outcome,
        notification: None,
    }
}

fn no_notification(
    log: &impl Fn(LogLevel, &str, Option<&str>),
    has_new: bool,
    already_sent: bool,
) -> MenuCheckResult {
    log(
        LogLevel::Guard,
        "no_notification",
        Some(&format!("hasNew={has_new} alreadySent={already_sent}")),
    );
    result(MenuCheckOutcome::NoNotification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::SystemClock;
    use crate::http::{CapturingTransport, HttpResponse};
    use crate::notify::fingerprint::is_notification_sent;
    use crate::storage::MemoryKv;
    use std::sync::Arc;

    const LOGIN_PAGE: &str = include_str!("../../tests/fixtures/gourmet/login-page.html");
    const LOGIN_OK: &str = include_str!("../../tests/fixtures/gourmet/login-success.html");
    const MENUS_0: &str = include_str!("../../tests/fixtures/gourmet/menus-page-0.html");
    const MENUS_1: &str = include_str!("../../tests/fixtures/gourmet/menus-page-1.html");

    fn ok(body: &str) -> HttpResponse {
        HttpResponse {
            status: 200,
            headers: vec![],
            body: body.into(),
        }
    }
    fn creds(u: &str, p: &str) -> Credentials {
        Credentials {
            username: u.into(),
            password: p.into(),
        }
    }

    #[tokio::test]
    async fn no_credentials_returns_no_credentials() {
        let t = Arc::new(CapturingTransport::new());
        let g = GourmetApi::new(t.clone());
        let kv = MemoryKv::new();
        let r = run_background_menu_check(&g, &kv, &SystemClock, None, None).await;
        assert_eq!(r.outcome, MenuCheckOutcome::NoCredentials);
        assert_eq!(t.requests().len(), 0);
    }

    #[tokio::test]
    async fn demo_credentials_skipped_without_network() {
        let t = Arc::new(CapturingTransport::new());
        let g = GourmetApi::new(t.clone());
        let kv = MemoryKv::new();
        let r = run_background_menu_check(
            &g,
            &kv,
            &SystemClock,
            None,
            Some(creds("demo", "demo1234!")),
        )
        .await;
        assert_eq!(r.outcome, MenuCheckOutcome::DemoSkipped);
        assert_eq!(t.requests().len(), 0);
    }

    #[tokio::test]
    async fn first_run_detects_and_fires_then_second_run_resets_latch() {
        let t = Arc::new(CapturingTransport::new());
        let g = GourmetApi::new(t.clone());
        let kv = MemoryKv::new();
        // run 1: login + menus, known empty → hasNew, notify.
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_OK));
        t.queue_response(ok(MENUS_0));
        t.queue_response(ok(MENUS_1));
        let r1 =
            run_background_menu_check(&g, &kv, &SystemClock, None, Some(creds("u", "p"))).await;
        assert_eq!(r1.outcome, MenuCheckOutcome::Notified);
        match r1.notification.unwrap() {
            NotificationCommand::FireNow { id, channel_id, .. } => {
                assert_eq!(id, NEW_MENU_ID);
                assert_eq!(channel_id.as_deref(), Some(MENU_UPDATES_CHANNEL));
            }
            _ => panic!("expected FireNow"),
        }
        assert!(is_notification_sent(&kv)); // latch set

        // run 2: same menus, known now == current, alreadySent=true → resets latch, no notify.
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok(LOGIN_OK));
        t.queue_response(ok(MENUS_0));
        t.queue_response(ok(MENUS_1));
        let r2 =
            run_background_menu_check(&g, &kv, &SystemClock, None, Some(creds("u", "p"))).await;
        assert_eq!(r2.outcome, MenuCheckOutcome::NoNotification);
        assert!(r2.notification.is_none());
        assert!(!is_notification_sent(&kv)); // latch reset
    }

    #[tokio::test]
    async fn login_failure_returns_failed() {
        let t = Arc::new(CapturingTransport::new());
        let g = GourmetApi::new(t.clone());
        let kv = MemoryKv::new();
        t.queue_response(ok(LOGIN_PAGE));
        t.queue_response(ok("<html>login form again</html>")); // no /einstellungen/ → login fails
        let r =
            run_background_menu_check(&g, &kv, &SystemClock, None, Some(creds("u", "bad"))).await;
        assert_eq!(r.outcome, MenuCheckOutcome::Failed);
    }
}
