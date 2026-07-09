# 00 — Overview

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This document orients an implementer who has **no access to the v1 source**: what the
product is, who uses it, what v2 targets, why the rewrite happens, the domain vocabulary
used across all requirements docs, and an index of every document in this tree.

---

## 1. What SnackPilot is

SnackPilot is a mobile app for **company cafeteria menu ordering and expense tracking**.
It has **no backend of its own**. All data comes from **web scraping two external
websites** — the app submits the same HTML forms and JSON endpoints a browser would,
parses the returned HTML, and renders the results natively
(v1: CLAUDE.md "Critical Warning"; src/app/src-rn/api/).

The two external systems:

| System | Internal name | Base URL | Purpose | Spec |
|---|---|---|---|---|
| Gourmet "a la Click" portal | **Kantine** | `https://alaclickneu.gourmet.at` (v1: src/app/src-rn/utils/constants.ts:1) | Weekly menu plans, placing/cancelling lunch orders, canteen billing | [01-gourmet-scraping.md](01-gourmet-scraping.md) |
| Ventopay "mocca.website" portal | **Automaten** | `https://my.ventopay.com/mocca.website` (v1: src/app/src-rn/utils/constants.ts:16) | Transaction/billing data from cafeteria POS terminals and vending machines | [02-ventopay-scraping.md](02-ventopay-scraping.md) |

**The scraping is safety-critical.** Both sites detect "incomplete" or malformed
submissions as bot behavior; the Gourmet backend **bans user accounts** for deviations
from the exact request shape (missing hidden form fields, wrong content-type, wrong
parameter values, reordered requests). The scraping specs (docs 01 and 02) are therefore
written to byte-level precision and must be implemented exactly — never "improved"
(v1: CLAUDE.md "Things That Will Break Accounts").

On top of the scraped data, the app provides: menu browsing with day navigation and
categories, a pending-selection cart with an ordering-cutoff rule, order approval and
cancellation (an edit-mode form state machine), a merged billing view over both sources,
offline caching, a notification suite (new-menu detection, daily order reminder, cancel
reminder, location/geofence reminders), a diagnostic notification log, themes with
alternate app icons, anonymous TelemetryDeck analytics, and a demo mode with canned data
for store review (see the [document index](#7-document-index)).

SnackPilot v1 is a fork/rewrite lineage of
[GourmetClient](https://github.com/patrickl92/GourmetClient) by patrickl92; the README
must always credit that project (v1: CLAUDE.md "README Requirements", README.md
"Credits").

## 2. Users

- **Employees of a single company** whose canteen runs on Gourmet "a la Click" and whose
  POS/vending machines run on Ventopay. The Ventopay company is hardcoded:
  `0da8d3ec-0178-47d5-9ccd-a996f04acb61` (v1: src/app/src-rn/utils/constants.ts:20).
  The app is not multi-tenant.
- Users authenticate with their **personal existing accounts** on each of the two
  external systems (two independent credential pairs). Credentials are stored on-device
  in platform secure storage; there is no SnackPilot account
  (v1: src/app/src-rn/store/authStore.ts, src/app/src-rn/utils/secureStorage.ts; see
  05-platform-services.md §1).
- The audience is **German-speaking**: all user-facing text is German and must be
  reproduced verbatim (tabs: "Menüs", "Bestellungen", "Abrechnung", "Einstellungen";
  see 04-ui-ux.md).
- **App-store reviewers / triers** use demo mode: logging in with username `demo` /
  password `demo1234!` activates canned data with no network access
  (v1: src/app/src-rn/utils/constants.ts:22-28; see 03-features/demo-mode.md).

## 3. v2 platform scope

Per the approved design (docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md):

| Component | v2 target |
|---|---|
| iOS app | Swift + SwiftUI, **iOS 17+** |
| Android app | Kotlin + Jetpack Compose (Material 3), **Android 10+ (API 29)** |
| Shared core | `snackpilot-core` Rust crate, exposed to both apps via UniFFI (async facade + plain-data records) |

The Rust core owns everything portable: HTTP clients (one `reqwest` cookie store per
service), HTML parsing, high-level API layers, domain models, caching, menu
fingerprinting/change detection, notification decision logic, and demo mode. The native
shells own UI, secure storage, background scheduling, notification delivery, location,
and alternate app icons — the core never touches those (design doc §4).

**Store identity is preserved:** v2 ships under the same bundle ID / application ID as
v1 — `dev.radaiko.gourmetclient` on both iOS and Android (v1: src/app/app.json:18,32) —
so it arrives as a normal store update, with best-effort credential takeover from v1's
expo-secure-store data on first launch (design doc §5; formats in
05-platform-services.md §1). Feature scope is **full parity with v1.4.5** (design doc §1).

### Dropped in v2

- **Tauri desktop app** (macOS/Windows/Linux) and the **web target** — desktop users stay
  on v1.4.5. All `src/desktop/`, `.web.ts`/`.web.tsx` platform branches, desktop wide
  layouts, and the Velopack auto-update machinery are retired (design doc §1, §7;
  07-release.md §8). Each requirements doc marks affected behavior with a
  "Dropped in v2" note and documents only the mobile behavior.
- Known deliberate behavior change: v1's Gourmet client relies on the platform HTTP
  stack's native cookie handling (NSURLSession on iOS) via `withCredentials`; v2 uses
  one reqwest cookie store on both platforms, following the same documented request
  sequence (design doc §4; 01-gourmet-scraping.md §2).

## 4. v1 → v2 rationale

From the design doc (§1, §4, §6, §9) plus fragility evidence in the v1 repo:

1. **Protect the ban-sensitive scraping.** A single Rust implementation of the request
   sequences, validated by fixture-driven request-shape tests, replaces per-platform JS
   behavior. Both apps ship the same tested core binary, so cross-platform drift in the
   fragile scraping layer is structurally impossible (design doc §6).
2. **Escape JS-ecosystem fragility.** Example from v1: the HTML parser (cheerio) must be
   pinned to exactly `1.0.0-rc.12` because every later version imports
   `node:stream`/`undici`, which Metro cannot resolve for React Native — breaking the app
   bundle at build time while Jest (running in Node) stays green (v1: CLAUDE.md "Tech
   Stack").
3. **First-class platform services.** The notification suite depends on background
   execution, geofencing, notification channels, and alternate app icons; v2 uses the
   native APIs directly (BGTaskScheduler / WorkManager, UNUserNotificationCenter /
   NotificationManager, CoreLocation / FusedLocationProvider, `setAlternateIconName` /
   activity-alias) instead of Expo abstractions (design doc §4).
4. **Cheap future desktop.** The Rust core deliberately keeps a later desktop v2
   inexpensive, though it is out of scope for v2.0 (design doc §1).
5. **Seamless migration.** Same store listings, same app IDs, version `2.0.0`, one-time
   credential import so users do not re-enter logins (design doc §5, §7).

## 5. Feature scope (parity checklist)

Everything v1.4.5 does, v2.0 does (design doc §1):

- Gourmet (Kantine) login, session management, user-info extraction — 01
- Ventopay (Automaten) login and session management — 02
- Menu browsing: pagination, categories, allergens, day navigation — 03-features/menus.md
- Ordering (add to cart) and cancellation (edit-mode flow) — 03-features/orders.md
- Ordered-menus list with approval status — 03-features/orders.md
- Billing from both sources, merged, with filters — 03-features/billing.md
- Menu/order/billing caching for offline display and fast startup — 03-features/caching.md
- Themes / appearance incl. alternate app icons — 03-features/themes.md
- Demo mode (canned data, no network) — 03-features/demo-mode.md
- Analytics (TelemetryDeck, same events) — 03-features/analytics.md
- Notification suite: new-menu detection, daily order reminder, cancel reminder,
  location-based — 03-features/notifications-*.md
- Notification debug log — 03-features/notification-log.md

## 6. Glossary

Domain and site vocabulary used throughout these docs. German UI terms are kept verbatim.

| Term | Meaning |
|---|---|
| **Kantine** | The company canteen; in-app name for the Gourmet system. Settings row "Kantine-Zugangsdaten", login screen `kantine-login` (v1: src/app/app/kantine-login.tsx; 03-features/settings.md). |
| **Automaten** | German "vending machines"; in-app name for the Ventopay system (POS terminals + vending machines). Settings row "Automaten-Zugangsdaten", login screen `automaten-login`. |
| **Gourmet** | GOURMET, the Austrian catering operator; here, its "a la Click" ordering portal at `https://alaclickneu.gourmet.at` (v1: src/app/src-rn/utils/constants.ts:1). An Umbraco-based site: every form is `enctype="multipart/form-data"` and carries `ufprt` + `__ncforminfo` hidden fields. |
| **Ventopay** | Operator of the "mocca" POS system; portal at `https://my.ventopay.com/mocca.website` (v1: src/app/src-rn/utils/constants.ts:16). A classic ASP.NET WebForms site (`__VIEWSTATE` et al.). |
| **Company ID** | Hardcoded Ventopay company UUID `0da8d3ec-0178-47d5-9ccd-a996f04acb61`, sent as `DropDownList1` in the Ventopay login form (v1: src/app/src-rn/utils/constants.ts:20). Must never change. |
| **`ufprt`** | Umbraco form-protection (CSRF) token; hidden input present in every Gourmet form. Must be freshly extracted and echoed with every form POST (01-gourmet-scraping.md §3). |
| **`__ncforminfo`** | Second hidden anti-bot field in every Gourmet form. Omitting it is detected as bot behavior and triggers account bans (v1: CLAUDE.md; 01-gourmet-scraping.md §3). |
| **ASP.NET state fields** | `__VIEWSTATE`, `__VIEWSTATEGENERATOR`, `__EVENTVALIDATION` — hidden inputs on Ventopay pages that must be extracted and echoed on POST (02-ventopay-scraping.md §3). |
| **User info / EaterId / ShopModelId / StaffGroupId** | Identifiers extracted from the Gourmet page after login (`#eater`, `#shopModel`, `#staffGroup` values, plus `.loginname` username); required parameters of the Gourmet JSON APIs (v1: src/app/src-rn/types/menu.ts:26-31; 01-gourmet-scraping.md §5). |
| **Menu item** | One offered dish on one day: id, day, title, subtitle, allergens, available, ordered, category, price (v1: src/app/src-rn/types/menu.ts:1-11). |
| **Menu categories** | `MENÜ I`, `MENÜ II`, `MENÜ III`, `SUPPE & SALAT`, `UNKNOWN` (v1: src/app/src-rn/types/menu.ts:13-19). Detected from the title via regex `/MEN(?:Ü|U)\s+([I]{1,3})/i` and the literal pattern `SUPPE & SALAT` (v1: src/app/src-rn/api/gourmetParser.ts:7-8). Note: CLAUDE.md documents the regex as `MENÜ\s+([I]{1,3})` — the code (case-insensitive, `U` or `Ü`) wins. |
| **Menu ID** | Gourmet's `data-id` on a menu item. **Per-category, not per-item**: all MENÜ I items on a page share one ID, so an item is only unique as (menu ID, day) (v1: CLAUDE.md; 03-features/menus.md §1). |
| **Composite item key** | App-level unique key for a menu item: `"{menuId}|{localDateKey(day)}"` with the local date formatted `YYYY-MM-DD` without timezone conversion (v1: src/app/src-rn/store/menuStore.ts:56-58, src/app/src-rn/utils/dateUtils.ts:45-50; 03-features/menus.md §1). |
| **Allergens** | Single-letter codes (comma-separated in one `li.allergen` element) attached to a menu item (v1: CLAUDE.md; 01-gourmet-scraping.md §8.2). |
| **Ordered menu / order** | A placed lunch order, one per position on the Gourmet orders page (`/bestellungen`): positionId, eatingCycleId, date, title, subtitle, approved (v1: src/app/src-rn/types/order.ts:1-8). |
| **Position ID** | Gourmet's identifier for one order line, from `input[name=cp_PositionId]`; the key used to cancel that order (01-gourmet-scraping.md §9). |
| **Eating cycle (ID)** | Gourmet's identifier for the meal slot an order belongs to, from `input[name=cp_EatingCycleId_{positionId}]`; echoed back in the cancel form (v1: src/app/src-rn/api/gourmetApi.ts:276-281; 01-gourmet-scraping.md §9). |
| **Approved / confirmed** | An order the canteen has confirmed (detected via a `.fa-check` or `.checkmark` element on the orders page — there is no `.confirmed` class despite CLAUDE.md; see 01-gourmet-scraping §9.1). Unapproved orders can still be confirmed or cancelled (03-features/orders.md §5). |
| **Edit mode** | A server-side toggle state on the Gourmet orders page (form class `form-toggleEditMode`, field `editMode=True`) that must be entered to cancel orders; v1 implements an inverted-state machine around it (01-gourmet-scraping.md §9.2; 03-features/orders.md §6). |
| **Cart / pending selections** | App-local set of menu selections not yet submitted; submitted via the `AddToMenuesCart` JSON API (03-features/menus.md §6). |
| **Ordering cutoff** | Rule blocking orders/cancellations for a menu day: past days always blocked; the current day blocked from **09:00 Europe/Vienna wall-clock time** (v1: src/app/src-rn/utils/dateUtils.ts:66-99; 03-features/menus.md §6.2). The 09:00 deadline also drives the cancel reminder (v1: src/app/src-rn/utils/constants.ts:40-42). |
| **Bill / GourmetBill** | One canteen receipt from the `GetMyBillings` JSON API: billNr, billDate, location, items, billing total; items carry subsidy and discount amounts (v1: src/app/src-rn/types/billing.ts). |
| **Subsidy** | Employer contribution deducted from a billing item's gross price (`subsidy` field; v1: src/app/src-rn/types/billing.ts:8). |
| **Transaction (Ventopay)** | One Automaten purchase: id, date, amount, restaurant, location (v1: src/app/src-rn/types/ventopay.ts:1-8), parsed from `Transaktionen.aspx`. |
| **Gourmet filter (Ventopay)** | Rule excluding Ventopay transactions that duplicate canteen bills (restaurant/location-based; exact rule in 02-ventopay-scraping.md §6.5 — the code differs from CLAUDE.md's wording; code wins). |
| **Month key** | `YYYY-MM` string identifying a billing month in state and cache keys, e.g. `billing_2026-07`, `ventopay_billing_2026-07` (v1: src/app/src-rn/types/billing.ts:24; 03-features/caching.md §2). |
| **Menu fingerprint** | Hash-based snapshot of the visible menu set used to detect newly published menus in background checks (v1: src/app/src-rn/utils/menuFingerprint.ts; 03-features/notifications-new-menu.md §1). |
| **Geofence** | 500 m radius region around the saved company location; entering/exiting drives location-based order reminders (v1: src/app/src-rn/utils/constants.ts:31-33; 03-features/notifications-location.md). |
| **Notification log** | Time-limited on-device diagnostic log of notification decision runs, exportable via e-mail (03-features/notification-log.md). |
| **Demo mode** | App state activated by logging in with `demo` / `demo1234!`: deterministic canned data, no network (v1: src/app/src-rn/utils/constants.ts:22-28; 03-features/demo-mode.md). |
| **Accent color** | One of five theme accents: `orange` (default, "Orange"), `emerald` ("Smaragd"), `berry` ("Beere"), `golden` ("Gold"), `ocean` ("Ozean"); also selects the matching alternate app icon (03-features/themes.md §3, §6). |
| **Tabs** | The four root screens: "Menüs" (menus), "Bestellungen" (orders), "Abrechnung" (billing), "Einstellungen" (settings) (04-ui-ux.md §1). |
| **TelemetryDeck** | The anonymous analytics provider; every install reports the same constant client-user value (03-features/analytics.md §1). |
| **Record & replay** | Test strategy: sanitized HTML fixtures recorded from the live sites drive parser/client/API tests offline (06-testing.md §2). |

## 7. Document index

All under `docs/requirements/`. Every doc begins with the same extraction header and
carries inline provenance references into v1 at main @ 6997c44.

| Doc | One-line summary |
|---|---|
| [00-overview.md](00-overview.md) | This document: product, users, v2 scope, rationale, glossary, index. |
| [01-gourmet-scraping.md](01-gourmet-scraping.md) | Byte-level Gourmet (Kantine) spec: HTTP client rules, multipart form-token rules (`ufprt` + `__ncforminfo`), login/logout/session-expiry flows, user-info extraction, menu pagination and parsing, orders page and the edit-mode inverted-state machine, confirm/cancel flows, `AddToMenuesCart` and `GetMyBillings` JSON APIs, date formats, ban rules, and code-vs-docs discrepancies. |
| [02-ventopay-scraping.md](02-ventopay-scraping.md) | Byte-level Ventopay (Automaten) spec: manual cookie-jar semantics, ASP.NET viewstate login with the hardcoded company UUID, logout, transaction fetching by date range, transactions-page parsing (title/amount, German numbers, timestamps), the Gourmet duplicate filter, ban rules, and code-vs-CLAUDE.md discrepancies. |
| [03-features/menus.md](03-features/menus.md) | Menu browsing: data model and composite key, 4-hour cache TTL and refresh flows, day navigation (bar + swipe), category grouping/order, card states, pending-order cart, submit flow with 09:00 Vienna cutoff, loading/error states. |
| [03-features/orders.md](03-features/orders.md) | Ordering and cancellation: ordered-menus list, submission pipeline (resolution, cutoff filter, optimistic update, network sequence, revert), approval/confirm semantics, edit-mode cancellation (single and batch), orders-screen UX. |
| [03-features/billing.md](03-features/billing.md) | Billing from both sources: data models, month options and month→range/offset mapping, fetch flows, persistence, merged unified entry list with source filter and totals, display rules and edge cases. |
| [03-features/caching.md](03-features/caching.md) | Cache inventory and serialization formats (`menus_items`, `orders_list`, `billing_{YYYY-MM}`, `ventopay_billing_{YYYY-MM}`), staleness/invalidation rules, startup-from-cache behavior per tab, write-through points, corrupt-entry handling. |
| [03-features/notifications-new-menu.md](03-features/notifications-new-menu.md) | New-menu detection: exact fingerprint algorithm and serialization, background check task and cadence, OS notification content, foreground detection/acknowledgment, in-app toast, settings gating. |
| [03-features/notifications-daily-reminder.md](03-features/notifications-daily-reminder.md) | Daily order reminder: pre-scheduled local notification model, trigger points, ordered decision guards, scheduling and dedupe semantics, content, settings and persistence. |
| [03-features/notifications-cancel-reminder.md](03-features/notifications-cancel-reminder.md) | Cancel reminder: decision logic, trigger points, timing against the 09:00 cancellation deadline, dedupe, notification content, settings gating, edge cases. |
| [03-features/notifications-location.md](03-features/notifications-location.md) | Location-based notifications: location store, 500 m geofence definition and lifecycle, enter/exit behavior, "no order" notification timing/dedupe/retraction, permission flow, platform config, battery notes. |
| [03-features/notification-log.md](03-features/notification-log.md) | Notification debug log: entry model, activation window and storage, what each task logs, e-mail export format, log screen UX states. |
| [03-features/themes.md](03-features/themes.md) | Themes and appearance: light/dark/system resolution, exact light/dark palettes, the five accent themes with exact values, appearance screen, alternate app icons (mapping, mechanism, asset inventory and design spec). |
| [03-features/demo-mode.md](03-features/demo-mode.md) | Demo mode: activation via demo credentials, persistence and stickiness quirk, demo Gourmet/Ventopay API contracts, deterministic PRNG canned-data generators, behavioral differences vs live mode. |
| [03-features/settings.md](03-features/settings.md) | Settings and login screens: navigation, settings-tab rows, both auth state machines (`login`, `loginWithSaved`, `logout`, startup auto-login), credential storage keys, Kantine/Automaten login screens, appearance and notifications screens, emitted analytics. |
| [03-features/analytics.md](03-features/analytics.md) | Analytics: TelemetryDeck provider/endpoint and wire format, delivery semantics, enablement gating, default payload, full event catalog, PII handling, testability. |
| [04-ui-ux.md](04-ui-ux.md) | UI/UX blueprint: screen inventory and navigation graph, bootstrap, tab-bar variants (iOS glass pill, Android standard), per-screen layouts with verbatim German strings, shared components, state matrix, animation constants, platform style recipes, color tokens. |
| [05-platform-services.md](05-platform-services.md) | Platform services: secure-storage keys and the exact expo-secure-store 55.0.15 iOS Keychain / Android formats needed for credential takeover, AsyncStorage keys, platform detection, background tasks and scheduler config, notification permissions/handler/channels, location permissions, `app.json`-derived native config, localization. |
| [06-testing.md](06-testing.md) | Testing: record & replay strategy, the 13 sanitized HTML fixtures, recorder-script sequences and sanitization rules, v1's 27-file suite inventory, and what the v2 Rust core suite must replicate (request-shape, parser, orchestration assertions). |
| [07-release.md](07-release.md) | Release and CI: immutable store identities, versioning, per-store release process (`ship.sh`, EAS as v1 mechanism), CI inventory and v2 requirements, icon-generation pipeline, privacy-policy hosting, retired desktop/Velopack machinery. |
| [appendix-source-map.md](appendix-source-map.md) | Coverage matrix: every v1 source file mapped to its owning requirements doc(s) or explicitly marked dropped/not applicable. |

Related material outside this directory:

- `docs/fixtures/` — the 13 sanitized HTML/JSON fixtures carried over from v1's test
  suite (9 Gourmet, 4 Ventopay), referenced by 06-testing.md.
- `docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md` — the approved v2
  architecture/design this requirements tree serves.

## 8. Conventions used across these docs

- **Provenance**: facts cite their v1 source inline as
  `(v1: src/app/src-rn/api/gourmetClient.ts:42)`, resolved against main @ 6997c44.
- **Code wins**: where v1 code contradicts CLAUDE.md or a design doc, the code's behavior
  is specified and the discrepancy is called out (each scraping/feature doc has a
  discrepancies section). Do not treat v1's CLAUDE.md as authoritative on its own.
- **"Dropped in v2"**: marks v1 behavior (desktop/web branches, Tauri, Velopack) that v2
  intentionally omits; the mobile behavior is what is specified.
- **"v1 mechanism"**: marks descriptions of *how* v1 (Expo/React Native) implemented
  something where that mechanism matters for fidelity (e.g. NSURLSession cookie handling,
  expo-secure-store storage formats, AsyncStorage). Requirements are otherwise stated
  platform-neutrally (WHAT, not the RN HOW).
- **Verbatim values**: URLs, selectors, regexes, key names, formats, and user-facing
  German strings are copied exactly from code, never paraphrased.
