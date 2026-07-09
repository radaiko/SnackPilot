# SnackPilot v2.0 — Native Rewrite Design

**Date:** 2026-07-08
**Status:** Approved pending user review
**Baseline:** v1.4.5 on `main`

## 1. Goal & Scope

SnackPilot v2.0 is a ground-up rewrite of the app as two native mobile apps — iOS in Swift (SwiftUI) and Android in Kotlin (Jetpack Compose) — sharing a single Rust core library that owns all portable logic, most critically the account-ban-sensitive web scraping.

**Feature scope: full parity with v1.4.5.** Everything the current app does, v2.0 does:

- Gourmet (Kantine) login, session management, user info extraction
- Ventopay (Automaten) login and session management
- Menu browsing with pagination, categories, allergens, day navigation
- Ordering (add to cart) and order cancellation (edit-mode flow)
- Ordered-menus list with approval status
- Billing from both sources (Gourmet billing API + Ventopay transactions) with filters
- Menu/order/billing caching for offline display and fast startup
- Themes / appearance settings (incl. alternate app icons per v1)
- Demo mode (canned data, no network, for store review and trying the app)
- Analytics (same events/provider as v1)
- Notification suite:
  - New-menu detection (background fetch + fingerprint diffing + toast/notification)
  - Daily order reminder
  - Cancel reminder
  - Location-based notifications
- Notification debug log (as in v1)

**Platform scope: mobile only.** The Tauri desktop app and the web target are dropped. Desktop users stay on v1.4.5. The Rust core deliberately keeps a future desktop v2 cheap, but it is out of scope for v2.0.

**Minimum OS versions:** iOS 17+, Android 10+ (API 29).

## 2. Phase 0 — Requirements Extraction (against `main`)

Before any v2 code is written, all requirements embodied in the v1 codebase are extracted into a self-contained documentation tree. Sources: the entire `src/app` source, the 13 design docs in `docs/plans/`, `analysis/playwright-findings.md`, `CLAUDE.md`, and the test suite (which encodes expected behavior).

The extraction is executed by a multi-agent workflow (parallel readers per subsystem), followed by an **adversarial verification pass**: independent agents diff each produced doc against the actual code, hunting for omissions, invented details, and contradictions. Docs must be precise enough that an implementer with no access to the v1 source can produce byte-identical HTTP requests.

### Docs tree (the entire content of the v2 branch's first commit, plus repo scaffolding)

```
docs/
├── requirements/
│   ├── 00-overview.md              # Product, users, platforms, v1→v2 rationale, glossary
│   ├── 01-gourmet-scraping.md      # Exact request sequences, headers, form encoding
│   │                               #   (multipart/form-data), ufprt + __ncforminfo,
│   │                               #   selectors, date formats, JSON APIs, ban rules
│   ├── 02-ventopay-scraping.md     # ASP.NET viewstate flow, hardcoded company ID,
│   │                               #   transaction list/detail parsing, filter rules
│   ├── 03-features/
│   │   ├── menus.md                # Browsing, categories, pagination, day navigation
│   │   ├── orders.md               # Cart, ordering, cancellation, edit-mode state machine
│   │   ├── billing.md              # Both sources, merge/display rules, filters
│   │   ├── caching.md              # What is cached, invalidation, offline behavior
│   │   ├── notifications-new-menu.md      # Fingerprinting, background check, toast
│   │   ├── notifications-daily-reminder.md
│   │   ├── notifications-cancel-reminder.md
│   │   ├── notifications-location.md
│   │   ├── notification-log.md     # Debug log storage & UI
│   │   ├── themes.md               # Appearance modes, colors, alternate app icons
│   │   ├── demo-mode.md            # Activation, canned data, behavior differences
│   │   ├── settings.md             # All settings screens and stored preferences
│   │   └── analytics.md            # Provider, events, opt-in/out behavior
│   ├── 04-ui-ux.md                 # Screen inventory, navigation graph, components,
│   │                               #   platform-specific styling rules
│   ├── 05-platform-services.md     # Secure storage keys/format (incl. v1 takeover),
│   │                               #   background task scheduling, permissions, location
│   ├── 06-testing.md               # Record & replay strategy, fixture sanitization,
│   │                               #   recorder script requirements
│   └── 07-release.md               # Store distribution, same-app-ID update path,
│                                   #   versioning, CI requirements
├── architecture/
│   └── v2-architecture.md          # This architecture, expanded: core API surface,
│                                   #   FFI boundary, threading, storage paths
└── fixtures/                       # Sanitized HTML fixtures carried over from v1's
                                    #   test suite (later moved to core test assets)
```

## 3. Fresh Start — Orphan Branch

- `git checkout --orphan v2` from `main`.
- First commit contains **only**: the `docs/` tree above, a v2 `README.md` (must keep the credit line linking to https://github.com/patrickl92/GourmetClient), a v2 `CLAUDE.md`, `.gitignore`, `.env.example` (same credential format as v1).
- No v1 source code on the branch. v1 history remains untouched on `main`.
- The v2 branch replaces `main` when v2.0 releases; until then `main` continues to represent the shipped v1.

## 4. Architecture — Rust Core, Native Shells

### Repository layout (on the v2 branch, built up after the docs commit)

```
docs/                  # Requirements & architecture (from Phase 0)
src/core/              # snackpilot-core: Rust crate + UniFFI bindings
src/ios/               # SwiftUI app (Xcode project; core via XCFramework + SPM)
src/android/           # Kotlin/Compose app (Gradle; core as AAR via cargo-ndk)
```

### Rust core (`snackpilot-core`) — thick

Owns everything portable:

- **HTTP clients:** `reqwest` with a dedicated cookie store per service (Gourmet, Ventopay). Replicates the documented request sequences *exactly*: same order, headers, `multipart/form-data` encoding, `ufprt` + `__ncforminfo` on every Gourmet form POST, ASP.NET viewstate fields on Ventopay, `RememberMe` as literal string `"false"`, hardcoded Ventopay company UUID, no artificial delays.
- **HTML parsing:** `scraper` crate implementing the documented selectors (v1's Cheerio selectors transfer 1:1 as CSS selectors).
- **API layers:** high-level Gourmet and Ventopay operations mirroring v1's `gourmetApi.ts` / `ventopayApi.ts`.
- **Domain models:** menus, orders, billing, transactions, user info.
- **Caching:** menu/order/billing caches persisted under a storage directory injected by the host app at initialization.
- **Menu fingerprinting & change detection:** the new-menu diffing logic from v1's `menuFingerprint.ts` / `menuChangeStorage.ts`.
- **Notification decision logic:** given current state + time + settings, "which notifications should fire now" — the portable halves of v1's `backgroundMenuCheck.ts`, `dailyReminderCheck.ts`, `cancelReminderCheck.ts`.
- **Demo mode:** canned data and demo API implementations (v1's `demoData.ts`, `demoGourmetApi.ts`, `demoVentopayApi.ts`).

**FFI boundary:** UniFFI, using its async support. The core exposes a facade of async operations plus plain-data records; hosts inject: storage path, credentials (from native secure storage), and current settings. The core never touches secure storage, notification APIs, or location — those are host concerns.

**Behavior change vs v1 (deliberate, must be documented):** v1's Gourmet client on iOS relies on NSURLSession's native cookie handling. In v2, reqwest's cookie store handles cookies identically on both platforms. Same documented request sequence, one implementation, validated against recorded fixtures.

### Native shells — UI + platform glue only

| Concern | iOS (Swift/SwiftUI) | Android (Kotlin/Compose) |
|---|---|---|
| UI | SwiftUI, tab navigation mirroring v1 (Menus, Orders, Billing, Settings) | Jetpack Compose, Material 3 |
| Secure storage | Keychain | Keystore + EncryptedSharedPreferences |
| Background execution | BGTaskScheduler (app refresh) | WorkManager (periodic work) |
| Notifications | UNUserNotificationCenter | NotificationManager + channels |
| Location | CoreLocation (region monitoring) | FusedLocationProvider / geofencing |
| Alternate app icons | `setAlternateIconName` | activity-alias |

Native code calls the core for all data and decisions; it schedules OS work, renders UI, and delivers notifications the core tells it to deliver.

## 5. Credential Takeover from v1 (best-effort)

v2 keeps v1's bundle ID / application ID, so app data survives the store update. On first launch, v2 attempts to import v1's saved logins so users don't re-enter credentials:

- **iOS:** v1 (expo-secure-store) writes generic-password Keychain items, which persist across app updates. Phase 0 must document the exact service/account names and value encoding v1 uses; v2 reads them once, re-saves into its own Keychain entries, then deletes the legacy items.
- **Android:** v1 (expo-secure-store) stores values in SharedPreferences encrypted with an Android-Keystore-backed key; both survive updates. v2 must replicate expo-secure-store's decryption path (prefs file name, key alias, cipher scheme — documented in Phase 0 from the library version v1 ships) to import once, then migrate to its own storage.

This is **best-effort**: if import fails for any reason, v2 falls back to the normal login screen. Import must never crash or block startup.

## 6. Testing

- **Rust core carries the contract test suite** — v1's record & replay strategy ported: sanitized HTML fixtures (carried over from v1, re-recordable via a recorder script using `.env` credentials) drive tests for parsers, both clients (request-shape assertions: method, URL, headers, encoding, form fields), and API orchestration. These tests are the guarantee that the fragile scraping behavior matches the spec.
- **Native shells get thin tests:** smoke/UI tests for navigation and rendering, unit tests for platform glue (storage adapters, scheduling mappers). Business logic is not duplicated natively, so it is not tested natively.
- Both apps ship the same tested core binary; drift between platforms is structurally impossible for everything inside the core.

## 7. Distribution & Release

- **Same bundle ID / application ID as v1** — v2.0 ships as a normal store update; credential takeover per §5.
- Version `2.0.0`; App Store + Play Store as today (v1's `docs/app-store-release.md` process carries over where still applicable).
- **CI (GitHub Actions):** core build + test (Rust, both target architectures), iOS app build, Android app build. Release workflows produce store artifacts.
- Velopack/desktop release machinery is retired with the desktop app.

## 8. Execution Order

1. **Phase 0 — Requirements extraction** (multi-agent workflow against `main`) → docs tree, verified adversarially against the code.
2. **Branch setup** — orphan `v2` branch; first commit = docs + scaffolding (README with GourmetClient credit, CLAUDE.md, .gitignore, .env.example).
3. **Rust core** — models, clients, parsers, APIs, caching, fingerprinting, decision logic, demo mode; fixture test suite green.
4. **iOS app** — SwiftUI shell over the core; visual verification on the iOS Simulator.
5. **Android app** — Compose shell over the core; verification on emulator.
6. **Parity audit** — feature-by-feature check of both apps against `docs/requirements/`, including notifications and credential takeover.

Each step gets its own implementation plan; this spec is the umbrella design.

## 9. Risks & Mitigations

- **Scraping regressions ban accounts.** Mitigation: docs written to byte-level precision, fixture-driven request-shape tests, single Rust implementation, manual verification with a real account before release.
- **reqwest TLS/cookie behavior differs from v1's platform HTTP stacks.** Mitigation: fixtures assert request shape; first live test performed carefully with a low-value account; cookie flow documented in Phase 0.
- **expo-secure-store internals (Android) may resist replication.** Mitigation: takeover is best-effort by design; fallback is the login screen.
- **Background execution is less deterministic natively than expected.** iOS BGTaskScheduler gives no timing guarantees — same as v1's expo background fetch; the notification decision logic in the core is idempotent and tolerant of irregular invocation.
- **Two new UI codebases.** Mitigation: v1's screens are simple (4 tabs + a few sub-screens); `04-ui-ux.md` is the shared blueprint; parity audit at the end.
