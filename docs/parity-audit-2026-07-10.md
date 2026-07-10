# SnackPilot v2 — Parity Audit vs v1.4.5

> Audited 2026-07-10 against `docs/requirements/` (extracted from v1.4.5 @ main 6997c44).
> Method: 5 parallel auditors read the actual v2 code (`src/core/` Rust, `src/ios/` SwiftUI,
> `src/android/` Compose) against their requirement clusters and classified every requirement
> ✅ implemented / 🟡 partial / ❌ missing / ⚠️ deviation, with file evidence.

## Verdict

**The portable Rust core is at or near full parity and is well-tested. The gap to v1.4.5 is
almost entirely in the native shells, OS integration, and release engineering** — much of it
"built in the core but never called by the apps," the rest genuinely unbuilt (background tasks,
location/geofencing, analytics, theming, release pipeline).

| Area | Parity | Where it lives |
|---|---|---|
| Gourmet + Ventopay scraping (ban-critical) | 🟢 **HIGH** — 0 missing | core ✅ |
| Feature logic: menus/orders/billing/caching | 🟢 **HIGH** (core) | core ✅ / shells 🟡 half-wired |
| Notification **decision** logic + demo mode | 🟢 **HIGH** | core ✅ (tested) |
| Demo mode | 🟢 **COMPLETE** | core ✅ |
| Notification **delivery** (immediate + scheduled + daily reminder) | 🟡 **PARTIAL** | shells ✅ foreground / ❌ background |
| Secure credential storage + v1 takeover format | 🟢 **HIGH** (Gourmet) | shells ✅ / Ventopay 🟡 unwired |
| UI/UX (screens, states, theming) | 🔴 **LOW** — vertical slice | shells ❌ |
| Background tasks (menu-check / order-sync / geofence) | 🔴 **MISSING** | shells ❌ |
| Location / geofencing | 🔴 **MISSING** | shells + core-persistence ❌ |
| Analytics (TelemetryDeck) | 🔴 **MISSING** | shells ❌ |
| Testing (core) | 🟢 139 tests | core ✅ |
| Testing (shells) + recorder port | 🔴 **MISSING** | ❌ |
| Release / CI / signing / icons / privacy | 🔴 **MISSING** | ❌ |

`docs/architecture/v2-architecture.md §10` deliberate deviations (mobile-only; desktop/web/
Velopack dropped; reqwest cookie model; explicit Accept header; background-check cookie
isolation) are **not** counted as gaps.

---

## 1. Ban-critical scraping — 🟢 HIGH (0 ❌, 6 ⚠️)

Every ban-critical detail is implemented and unit-pinned: exact request sequences, `ufprt`+
`__ncforminfo` on every Gourmet form, multipart-vs-urlencoded encoding, the 5/3/5/2-field
Gourmet payloads and 11-field Ventopay login, `RememberMe="false"`, hardcoded company UUID /
`BtnLogin` / `languageRadio`, all wire date formats, cookie-jar semantics, filter rules, and
verbatim error strings. **No deviation alters the ban-critical request bytes.**

**Two items to validate on the FIRST live run (before real accounts are used):**
- ⚠️ **Absent User-Agent** (arch §10.2 open question). reqwest sends none; v1 sent the
  platform default. Ban-relevance unresolved — highest-priority live check.
- ⚠️ **V-1: Ventopay login-302 cookie** under `cookie_store=false`. The app jar captures
  Set-Cookie from the final response only (correct per 02 §2.2); if the auth cookie is set on
  the login POST's intermediate 302 rather than the login-page GET, the post-login GET could
  arrive unauthenticated. v1 ran under the same constraint, so it likely works — but it's not
  determinable from source. Validate live.

**Two undocumented, non-ban deviations to reconcile (display-only):**
- G-1: order-item selector is `div.order-item` only; spec is `div.order-item, div[class*="order-item"]` (`gourmet/parser.rs:196`).
- G-2: missing order-date input falls back to epoch 0, not "now" (`gourmet/parser.rs:225`). Cancel POST echoes the raw string, so request bytes are unaffected.

---

## 2. Feature logic (menus / orders / billing / caching) — core HIGH, shells half-wired

**Core ✅ (faithful + tested):** composite keys, 4h TTL fetch + re-entrancy guard, the full
`submit_orders` pipeline (cancellation→positionId resolution, cutoff filtering, optimistic
update, cancel→add→confirm→refresh, revert-on-failure, `order.submitted` analytics payload),
availability merge-refresh incl. the 800ms min-visibility timer and v1's key-collision quirk,
upcoming/past split at device-local midnight, dual-source billing with month options / totals /
past-month skip, and the corruption asymmetry (menus/orders delete corrupt key, billing keeps).

**🟡 Shell gaps — core capability exists at an `ffi.rs` entry point but no app calls it:**
- **No cache-first display / startup-from-cache** — `load_cached_menus/orders/billing_months`
  never called; screens are blank until the network returns (caching §4).
- **Background availability refresh dead** — `refresh_availability` + the "Aktualisiere…"
  banner never invoked (menus §3.3, §7).
- **No focus-refresh lifecycle** — shells fetch once in `loadSession()`; no per-tab
  refresh-on-focus (menus §8 / orders §7 / billing §7).
- **Ordering cutoff not enforced in UI** — `is_ordering_cutoff` never called; menu cards are
  always tappable; no Geschlossen/Ausverkauft/disabled states (menus §6.1).
- **Confirm-orders UX missing** — no unconfirmed banner / bulk "Bestätigen" (orders §5.3);
  Android `AppViewModel` lacks `confirmOrders` entirely.
- **Billing** — no source filter (Alle/Kantine/Automaten), no unified date-descending merged
  list, no combined Gesamt/Belege/Zuschuss summary bar (billing §6).
- **Day navigation + category grouping** — shells render all days as a flat list; no
  single-day DayNavigator/swipe, no fixed-order category grouping (menus §4, §5).

**Core-side follow-ups exposed by this cluster:**
- `OrderProgress` has no FFI **read** path (the `ProgressListener` callback exists but shells
  pass null and can't poll a phase) — needed for the submit-progress banner.
- Minor: stores swallow cache-write errors (`let _ = save_*`); v1 surfaces some as user errors
  (caching §5). Edge case.

---

## 3. Notifications + demo + analytics

**Demo mode — ✅ COMPLETE & faithful** (magic-cred activation + guard, demo Gourmet/Ventopay
APIs, verbatim dish pools, JS-exact LCG, per-instance caching, module-level order counter).

**Notification decision logic — ✅ HIGH in core** (fingerprint + menu-check state machine,
daily/cancel reminder windows, geofence Enter/Exit, log storage engine) — all unit-pinned.

**Delivery / OS integration — the gaps:**
- 🟡 **Daily reminder**: delivery wired (iOS `UNTimeIntervalNotificationTrigger`, Android
  `AlarmManager`), toggle+time in settings. ❌ but only fires when the app was foregrounded
  before target time — no background re-schedule.
- ❌ **Cancel reminder**: decision done; **no trigger path wired** (`cancel_reminder_command`
  never called) — can never fire.
- ❌ **New-menu background check**: `run_menu_check` invoked only from a DEBUG button; no
  BGTaskScheduler/WorkManager 15-min cadence; no foreground detect-ack + `NewMenuToast`
  (`acknowledge_menus` not implemented in core either).
- ❌ **Geofencing**: decision done; **no region registration**, no company-location
  persistence (`set_company_location`/`company_location` absent from `ffi.rs`), no
  location-settings UI, no location permissions.
- 🟡 **Log**: storage + activate/clear/entries wired. ❌ no e-mail export
  (`log_format_for_email` unimplemented; no mail composer to `aiko@spitzbub.app`), no 12h
  option, no active/expired state UX; only `menu-check` of 4 subsystems emits.
- **⚠️ Undocumented daily-reminder deviations to reconcile:** settings persisted in shell
  prefs (not core KV per arch §4.4); default 08:00 + free DatePicker vs v1's 11:00 default in
  an 11:00–13:45 15-min chip range; disabling **cancels** the pending reminder vs v1's
  deliberate no-retract (§10).

**Analytics — ❌ MISSING (biggest single hole here).** No TelemetryDeck transport (sink is
`nil`/`null`), no app ID/clientUser/endpoint, and only ~1 of 13 event types has an emission
point (`menu.newDetected`, itself dropped because the sink is nil). The `AnalyticsSink` seam
is the only piece present.

---

## 4. UI/UX — 🔴 LOW (vertical slice; iOS ≈ Android in coverage)

The shells implement the happy-path skeleton of ~2.5 of 4 tabs plus a diagnostics-flavored
Settings. **None** of the cuts below are documented as deliberate. Biggest first:

- ❌ **Login-wall violates the "no wall" model** (settings §3.7). Unauthenticated users should
  land in the tabs with per-tab `Nicht angemeldet` states — instead they're blocked, which
  makes Settings/Billing/sub-screens unreachable and those empty states unbuilt.
- ❌ **Automaten/Ventopay login screen entirely absent** (04 §3.6). `ventopayLogin` is only
  called with demo creds → **a real user cannot authenticate the Automaten billing source at all.**
- ❌ **Entire theming/appearance system** (themes.md): no Appearance screen, no Design
  (System/Hell/Dunkel) override, no 5 accent colors, no accent/theme persistence, no alternate
  app icons. ⚠️ Android's Material You dynamic color actively **contradicts** the spec's fixed
  brand palette.
- ❌ **Settings is the wrong screen** — a dev/diagnostics panel instead of the spec's nav list
  (Kantine/Automaten/Darstellung/Benachrichtigungen chevron rows + Datenschutz alert).
- ❌ **Notifications is not a dedicated screen**; **Standort-Benachrichtigungen** (geofence UI)
  wholly missing; log lacks e-mail send + 12h.
- 🟡 **Menüs**: no DayNavigator/swipe, no NewMenuToast, no refresh/progress banners, no
  category grouping, no card badges/dimming/cutoff states; static bottom bar instead of the
  FAB with `Bestellen (n)`/`Stornieren (n)` label; iOS uses a standard TabView instead of the
  spec's floating GlassTabBar.
- 🟡 **Bestellungen**: List sections `Anstehend/Vergangen` instead of `Kommende (n)/Vergangene (n)`
  segments; no unconfirmed banner; no cancel confirmation dialog (cancels immediately); no
  status badges.
- 🟡 **Abrechnung**: no source filter chips, no summary bar, two fixed sections instead of one
  merged descending list, BillCard shows only the first item, currency `12,34 €` vs de-AT `€ 12,34`.
- 🟡 Much German copy deviates from the verbatim-required strings; several native alert/dialog
  flows (Datenschutz, permission, login-failure) unwired.

---

## 5. Platform services & native config — 🔴 mostly MISSING

- ✅ **Credential storage format is byte-compatible for in-place v1 takeover** — iOS Keychain
  (`app:no-auth`, account/generic = key, AfterFirstUnlock) and Android SecureStore (AES-256-GCM
  envelope, `key_v1-*`, keystore alias) match §1.4/§1.5 exactly.
- 🟡 **Ventopay credential pair unwired** — stores define the keys but expose only
  `saveGourmet/savedGourmet`; Ventopay persistence/takeover is inert (ties to the missing
  Automaten login).
- ❌ **Background tasks** (BGTaskScheduler / WorkManager) — none. Largest platform gap.
- ❌ **Location / geofencing + permissions** — none (`ACCESS_*_LOCATION`,
  `FOREGROUND_SERVICE_LOCATION`, usage-description strings all absent).
- 🟡→❌ **Native config** — missing `UIBackgroundModes`, `BGTaskSchedulerPermittedIdentifiers`,
  `ITSAppUsesNonExemptEncryption`, orientation lock, `NSLocation*UsageDescription`, the
  `snackpilot` URL scheme (both platforms), Android adaptive-icon resources,
  `RECEIVE_BOOT_COMPLETED`/`WAKE_LOCK`.
- ⚠️ **Notification channel names deviate** (`Bestell-Erinnerungen` vs v1 `Bestellungs-Erinnerungen`;
  `Menü-Updates` vs `Neue Menüs`); vibration pattern `[0,250,250,250]` not set; `data.screen`
  deep-linking unwired. Channel **ids** + importances ✅.

---

## 6. Testing — core strong, shells & recorder missing

- ✅ 139 fixture-driven Rust tests — a strict upgrade over v1 (which shipped no CI tests).
- ⚠️ Fixtures duplicated in `src/core/tests/fixtures/` (byte-identical to `docs/fixtures/`,
  `build.rs` guards drift) — two copies to maintain.
- ❌ **Recorder port** (`record-fixtures` as a Rust dev binary) — not done.
- ❌ **Shell tests** — no XCTest / Android test source sets; shell orchestration untested.
- ⚠️ CI runs core tests on `ubuntu-latest` only (§5.1 wants both target arches).

---

## 7. Release / distribution / CI — 🔴 MISSING

Only `core-test.yml` exists. Absent: iOS `ios/v*` build+submit and Android `android/v*`
AAB+Play jobs, `ship.sh` version/tag orchestration + build-number auto-increment, Android
release **signing config**, `security-audit.yml` (weekly `cargo audit`), `dependabot.yml`,
the icon pipeline (`tools/icon-tools/` + assets + alternate icons), and `docs/privacy.html` on
the v2 branch. Store identity (`dev.radaiko.gourmetclient`, `SnackPilot`, `2.0.0`) is ✅.

---

## 8. Prioritized roadmap to v1.4.5 parity

**P0 — ban safety / correctness (do first, needs a real account for a controlled test):**
1. Live-validate the scraping port: absent User-Agent (§10.2) and V-1 Ventopay 302 cookie.
2. Fix undocumented scraping deviations G-1 (order-item selector) and G-2 (order-date fallback).

**P1 — make the app actually usable for a real (non-demo) user:**
3. Automaten/Ventopay login screen + wire `ventopayLogin` + persist Ventopay creds.
4. Remove the login wall → no-wall tabs with per-tab `Nicht angemeldet` states.
5. Wire the already-built core into the shells: cache-first display, focus/background
   refresh, `is_ordering_cutoff` card states, confirm-orders banner, billing summary/filter,
   day navigation + category grouping. (Mostly UI work over existing FFI; add `order_progress()`
   getter to the core.)

**P2 — platform services & notification suite completion (needs real devices to verify):**
6. Background tasks: BGTaskScheduler (iOS) + WorkManager (Android) for menu-check, order-sync,
   and daily/cancel-reminder re-scheduling; native config (background modes, BG identifiers).
7. Location/geofencing: region registration + permissions + company-location persistence
   (add `set_company_location`/`company_location`/`acknowledge_menus`/`log_format_for_email`
   to the core FFI) + the Standort settings UI + new-menu toast + log e-mail export.
8. Reconcile the daily-reminder deviations (11:00 default + chip range + no-retract) and
   notification channel names/vibration.

**P3 — theming, analytics, release:**
9. Full theming system (light/dark/system + 5 accents + persistence + alternate icons);
   decide whether to keep Android Material You (currently conflicts with the spec).
10. Analytics: TelemetryDeck transport via the `AnalyticsSink` seam + the 13 event emission points.
11. Release engineering: iOS/Android build+sign+ship CI, `ship.sh`, security-audit, dependabot,
    icon pipeline, privacy policy, shell tests, recorder-script port.

**Bottom line:** the hard, risky, ban-sensitive part (the core) is done and tested; what remains
is a large but mostly mechanical build-out of the native shells, OS integration, and release
plumbing — none of which changes the ban-critical request bytes.
