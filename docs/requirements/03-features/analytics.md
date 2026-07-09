# Analytics

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

SnackPilot sends anonymous usage analytics to **TelemetryDeck**. There is no user-level
identity: every install reports the same constant client-user value, so no individual
device or person can be distinguished. Analytics is strictly fire-and-forget — a failure
to send must never crash, block, or otherwise affect the app.

Related docs: screen/feature flows that *trigger* events are specified in
`03-features/menus.md`, `03-features/orders.md`, `03-features/settings.md`,
`03-features/notifications-daily-reminder.md`, `03-features/notifications-location.md`,
and `03-features/notifications-new-menu.md`. This doc owns the event schema and transport.

---

## 1. Provider and endpoint

| Item | Value | Provenance |
|---|---|---|
| Provider | TelemetryDeck | (v1: src/app/src-rn/utils/analytics.ts:2) |
| App ID | `BA25F62D-0154-4A92-BF85-29FC5FDDA3EC` | (v1: src/app/src-rn/utils/analytics.ts:4) |
| Ingest endpoint | `https://nom.telemetrydeck.com/v2/` (SDK default; v1 does not override `target`) | (v1: node_modules/@telemetrydeck/sdk@2.0.4 dist/telemetrydeck.js:77) |
| `clientUser` (pre-hash) | literal string `anonymous` | (v1: src/app/src-rn/utils/analytics.ts:8) |
| Salt | none (empty string, SDK default) | (v1: src/app/src-rn/utils/analytics.ts:6-10) |
| `testMode` | `false` (explicit, so the wrapper's localhost auto-detection never applies) | (v1: src/app/src-rn/utils/analytics.ts:9) |

**v1 mechanism:** the JS SDK `@typedigital/telemetrydeck-react@0.5.0` (wrapping
`@telemetrydeck/sdk@2.0.4`) is instantiated once at module load of `analytics.ts`
(v1: src/app/src-rn/utils/analytics.ts:6-10; src/app/package.json:20). Importing the module
also installs a WebCrypto `crypto.subtle.digest` polyfill backed by `expo-crypto`
because React Native/Hermes lacks WebCrypto — the SDK needs SHA-256 to hash `clientUser`
(v1: src/app/src-rn/utils/cryptoPolyfill.ts:1-35). This polyfill is an RN workaround only;
v2 platforms have native SHA-256. In v2, TelemetryDeck's official native Swift/Kotlin SDKs
may be used instead of hand-rolling the wire protocol, **provided** the event names,
payload keys, app ID, and anonymity guarantees below are preserved.

### 1.1 Wire format (as produced by v1's SDK)

Each signal is one HTTP request:

- **POST** `https://nom.telemetrydeck.com/v2/`
- Header: `Content-Type: application/json`
- Body: a **JSON array containing one signal object**:

```json
[
  {
    "clientUser": "2f183a4e64493af3f377f745eda502363cd3e7ef6e4d266d444758de0a85fcc8",
    "sessionID": "<random>",
    "appID": "BA25F62D-0154-4A92-BF85-29FC5FDDA3EC",
    "type": "<event name>",
    "telemetryClientVersion": "JavaScriptSDK 2.0.4",
    "payload": { "<key>": "<string value>", "...": "..." }
  }
]
```

(v1: node_modules/@telemetrydeck/sdk@2.0.4 dist/telemetrydeck.js:159-238)

- `clientUser` on the wire is `SHA-256(clientUser + salt)` as lowercase hex. With v1's
  constant `anonymous` and empty salt this is always
  `2f183a4e64493af3f377f745eda502363cd3e7ef6e4d266d444758de0a85fcc8` — **identical for
  every install**.
- `sessionID` is generated once per process launch by the SDK:
  `(0 | (Math.random() * 9e6)).toString(36)` (low-entropy base-36 string; regenerated on
  every app relaunch, NOT rotated on foreground/background)
  (v1: node_modules/@telemetrydeck/sdk@2.0.4 dist/telemetrydeck.js:2-4, 95).
- `payload` is omitted entirely when the merged payload is empty.
- `isTestMode` is omitted (v1 sets `testMode: false`); `receivedAt` is omitted (v1 uses
  the immediate-send `signal()` path, never the queue).
- No `tdReactVersion` key appears in payloads: the react wrapper adds it only via its
  `useTelemetryDeck()` hook / `payloadEnhancer`, which v1 bypasses by calling `td.signal()`
  directly (v1: src/app/src-rn/utils/analytics.ts:23;
  node_modules/@typedigital/telemetrydeck-react@0.5.0 dist).
- Payload value coercion (SDK): key `floatValue` → parsed float; `Date` → ISO string;
  string kept as-is; other objects → `JSON.stringify`; everything else → string-coerced.
  In v1 every value is already passed as a string, so this is moot but documented for
  fidelity (v1: node_modules/@telemetrydeck/sdk@2.0.4 dist/telemetrydeck.js:200-222).

### 1.2 Delivery semantics

`trackSignal(type, payload?)` is the single app-side entry point. It merges the
**default payload** (section 3) under the event-specific payload (event keys win on
collision), sends immediately, and swallows all errors — both synchronous throws and
promise rejections (v1: src/app/src-rn/utils/analytics.ts:21-30).

Requirements:
- No retries, no offline queueing, no batching — a failed send is silently dropped.
- Analytics must never crash the app or surface an error to the user.
- Sends are non-blocking (fire-and-forget); no app flow awaits an analytics result.

---

## 2. Enablement gating (no opt-in/opt-out)

**There is no user-facing opt-in or opt-out.** No settings toggle, no App Tracking
Transparency prompt, no consent dialog exists anywhere in v1.

The only gate is build type: the root layout mounts `AnalyticsProvider` **only in
release builds** (v1: src/app/app/_layout.tsx:109-119):

- `__DEV__` (debug) builds: the provider is not mounted → no lifecycle signals
  (`app.launched` / `app.foregrounded` / `app.backgrounded`) and the default payload is
  never populated. **However**, the `trackSignal(...)` call sites in stores and screens
  still execute in dev builds and send bare signals (event payload only, no default
  parameters) to the production TelemetryDeck app ID with `testMode: false`. This is the
  actual v1 code behavior; whether it was intended could not be determined from source
  (recorded as an open question).
- Release builds: provider mounted, full behavior below.

### 2.1 User disclosure (informational only)

The Settings tab has a "Datenschutz" link that shows an alert titled `Datenschutz` with
this exact German text (v1: src/app/app/(tabs)/settings.tsx:138-149):

> Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung.
> Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen
> Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder
> Abrechnungsdaten erfasst.

The store privacy policy (`docs/privacy.html`, carried to v2 per `07-release`) likewise
discloses TelemetryDeck as an anonymous, cookieless analytics service and states that no
analytics ID exists, that stored data cannot be attributed to an individual, and that
individual data deletion is therefore not *required* (v1: docs/privacy.html:70-125). Any v2 change to the event set or payloads must stay
consistent with these disclosures (no personal data, no passwords, no menu choices, no
billing data).

---

## 3. Default payload (merged into every signal)

Set once at app launch by the analytics provider, after which it is merged under every
event payload (v1: src/app/src-rn/components/AnalyticsProvider.tsx:85-124;
src/app/src-rn/utils/analytics.ts:13-26). All values are strings.

| Key | Value (exact derivation) |
|---|---|
| `TelemetryDeck.AppInfo.version` | app version from build config (e.g. `1.4.5`); `unknown` if unavailable |
| `TelemetryDeck.AppInfo.buildNumber` | iOS build number, else Android versionCode as string, else `unknown` |
| `TelemetryDeck.AppInfo.versionAndBuildNumber` | `` `${appVersion} ${buildNumber}` `` (space-separated) |
| `TelemetryDeck.Device.platform` | `ios` or `android` (lowercase) |
| `TelemetryDeck.Device.operatingSystem` | `iOS` or `Android` (v1: AnalyticsProvider.tsx:9-11) |
| `TelemetryDeck.Device.systemVersion` | iOS: OS version string; Android: OS release version (e.g. `17.2`, `14`) (v1: AnalyticsProvider.tsx:13-18) |
| `TelemetryDeck.Device.systemMajorVersion` | systemVersion up to first `.` (v1: AnalyticsProvider.tsx:20-22) |
| `TelemetryDeck.Device.systemMajorMinorVersion` | first two dot-segments joined with `.`; the full string if it has fewer than 2 segments (v1: AnalyticsProvider.tsx:24-27) |
| `TelemetryDeck.Device.modelName` | device model name (e.g. `iPhone 15 Pro`), `unknown` if unavailable |
| `TelemetryDeck.Device.brand` | device brand, `unknown` if unavailable |
| `TelemetryDeck.Device.screenResolutionWidth` | app window width in logical points (not pixels), stringified |
| `TelemetryDeck.Device.screenResolutionHeight` | app window height in logical points, stringified |
| `TelemetryDeck.Device.screenScaleFactor` | display scale factor (e.g. `3`) |
| `TelemetryDeck.Device.orientation` | `Portrait` if height >= width else `Landscape`, at launch (v1: AnalyticsProvider.tsx:29-32) |
| `TelemetryDeck.Device.timeZone` | IANA time zone from system calendar, falling back to `Intl` resolved time zone, else `unknown` (v1: AnalyticsProvider.tsx:103) |
| `TelemetryDeck.RunContext.isSimulator` | `true`/`false` — running on simulator/emulator (negation of physical-device check) |
| `TelemetryDeck.RunContext.isDebug` | `true`/`false` — debug build flag (always `false` in practice; provider only mounts in release, see §2) |
| `TelemetryDeck.RunContext.locale` | full language tag of the first system locale (e.g. `de-AT`), else `unknown` |
| `TelemetryDeck.RunContext.language` | language code (e.g. `de`), else `unknown` |
| `TelemetryDeck.UserPreference.colorScheme` | `Dark` if system scheme is dark, otherwise `Light` (null/light both map to `Light`) (v1: AnalyticsProvider.tsx:112) |
| `TelemetryDeck.UserPreference.language` | same language code as above |
| `TelemetryDeck.UserPreference.layoutDirection` | `rightToLeft` if locale text direction is `rtl`, else `leftToRight` (v1: AnalyticsProvider.tsx:114) |
| `TelemetryDeck.UserPreference.region` | locale region code (e.g. `AT`), else `unknown` |
| `TelemetryDeck.SDK.name` | literal `JavaScriptSDK` — v1 value; the correct v2 value if using native SDKs is an open question |

### 3.1 Accessibility parameters (async, appended to defaults)

Collected asynchronously before the defaults are committed
(v1: src/app/src-rn/components/AnalyticsProvider.tsx:34-60). All values `true`/`false`.
If any query throws, collection stops and whatever was gathered so far is used.

Both platforms:
- `TelemetryDeck.Accessibility.isReduceMotionEnabled`
- `TelemetryDeck.Accessibility.isScreenReaderEnabled`

iOS only (additionally):
- `TelemetryDeck.Accessibility.isBoldTextEnabled`
- `TelemetryDeck.Accessibility.isInvertColorsEnabled`
- `TelemetryDeck.Accessibility.isReduceTransparencyEnabled`
- `TelemetryDeck.Accessibility.isDarkerSystemColorsEnabled`

### 3.2 Initialization ordering (v1 behavior)

The defaults (including accessibility) are committed and **then** `app.launched` fires,
so `app.launched` always carries the full default payload
(v1: src/app/src-rn/components/AnalyticsProvider.tsx:122-127). But event signals from
other code paths (e.g. `screen.viewed` on first tab focus, `auth.loginSuccess` from the
startup auto-login) can fire **before** the async accessibility collection resolves; such
signals carry only their event payload and none of the default parameters. This race is
accepted v1 behavior, not a requirement to preserve — v2 may initialize defaults
synchronously before any signal.

---

## 4. Events

Complete list — v1 emits exactly these 13 signal types (16 table rows below; rows 8–10
each cover two service payload variants — `gourmet` and `ventopay` — for 19 distinct
type/payload combinations across 21 `trackSignal` call sites). All payload values are
strings.

| # | Event `type` | Payload | Trigger | Provenance |
|---|---|---|---|---|
| 1 | `app.launched` | `startType: "cold"` | Once per cold start, immediately after the default payload is committed. Release builds only (see §2). | (v1: src/app/src-rn/components/AnalyticsProvider.tsx:126) |
| 2 | `app.foregrounded` | — | App state transition from `inactive` or `background` → `active` (previous state matched against regex `/inactive|background/`). | (v1: src/app/src-rn/components/AnalyticsProvider.tsx:130-132) |
| 3 | `app.backgrounded` | — | App state transition from `active` → `inactive` or `background`. | (v1: src/app/src-rn/components/AnalyticsProvider.tsx:133-134) |
| 4 | `screen.viewed` | `screen: "menus"` | Menus tab gains focus (fires on **every** focus, not just the first). | (v1: src/app/app/(tabs)/index.tsx:128) |
| 5 | `screen.viewed` | `screen: "orders"` | Orders tab gains focus. | (v1: src/app/app/(tabs)/orders.tsx:54) |
| 6 | `screen.viewed` | `screen: "billing"` | Billing tab gains focus. | (v1: src/app/app/(tabs)/billing.tsx:112) |
| 7 | `screen.viewed` | `screen: "settings"` | Settings tab gains focus. | (v1: src/app/app/(tabs)/settings.tsx:52) |
| 8 | `auth.loginSuccess` | `service: "gourmet"` or `"ventopay"` | Successful login for the respective service. Fires for demo-mode logins too, and for every automatic re-login with saved credentials (auto-login delegates to the same login function), i.e. typically once per service per app start. | (v1: src/app/src-rn/store/authStore.ts:40,45; src/app/src-rn/store/ventopayAuthStore.ts:38,43) |
| 9 | `auth.loginFailed` | `service: "gourmet"` or `"ventopay"` | Login attempt threw an error for the respective service. | (v1: src/app/src-rn/store/authStore.ts:50; src/app/src-rn/store/ventopayAuthStore.ts:48) |
| 10 | `auth.logout` | `service: "gourmet"` or `"ventopay"` | User-initiated logout. Fires **before** the logout HTTP request, i.e. unconditionally even if the request then fails. | (v1: src/app/src-rn/store/authStore.ts:65; src/app/src-rn/store/ventopayAuthStore.ts:63) |
| 11 | `order.submitted` | `orderedCount: "<n>"`, `cancelledCount: "<n>"` | Batch order submission from the Menus tab (`submitOrders` flow: cancel + add-to-cart + confirm + refresh) completed successfully. Counts are the number of newly ordered menu selections and the number of cancelled positions, stringified. Not fired on failure. | (v1: src/app/src-rn/store/menuStore.ts:277-280) |
| 12 | `order.confirmed` | — | Standalone "confirm orders" action succeeded (Orders tab). | (v1: src/app/src-rn/store/orderStore.ts:98) |
| 13 | `order.cancelled` | — | Single-order cancellation succeeded (Orders tab; one signal per cancelled position). | (v1: src/app/src-rn/store/orderStore.ts:114) |
| 14 | `notification.reminderToggled` | `enabled: "true"/"false"`, `hour: "<0-23>"`, `minute: "<0-59>"` | Daily-reminder switch toggled in the Notifications screen (fires for both enable and disable; hour/minute are the reminder time configured at toggle time). | (v1: src/app/app/notifications.tsx:128-132) |
| 15 | `notification.locationSet` | — | Company location successfully saved for location-based notifications (after permissions granted and position obtained). | (v1: src/app/app/notifications.tsx:172) |
| 16 | `menu.newDetected` | — | Background menu check detected new menus and fired the local "Neue Menüs verfügbar" notification. | (v1: src/app/src-rn/utils/backgroundMenuCheck.ts:84) |

Notes:
- Rows 4-7 are the only `screen.viewed` emissions. Sub-screens (Notifications, Appearance,
  Kantine login, Automaten login) do **not** emit `screen.viewed`.
- Rows 8-10: demo-mode logins are indistinguishable from real logins in analytics (no
  demo flag in the payload).
- `menu.newDetected` (row 16) can fire from a background task while the app UI never ran;
  in that case the default payload may be empty for the same reason as §3.2 (whatever the
  process state provides).

---

## 5. PII handling

- **No user identity.** `clientUser` is the constant `anonymous` for every install — all
  users of the app are indistinguishable on the TelemetryDeck dashboard. There is no
  device ID, advertising ID, or install ID (v1: src/app/src-rn/utils/analytics.ts:8).
- **No credentials or account data** ever appear in any payload: no usernames, no eater
  IDs, no shop-model IDs.
- **No content data**: no menu titles/choices, no billing amounts, no transaction data.
  Order events carry only counts.
- Most granular data sent: device model/brand, OS version, screen metrics, locale,
  region, time zone, accessibility flags (§3).
- The session ID is a low-entropy random string regenerated per app launch; it groups
  signals within one process lifetime only and cannot link across launches.
- These properties are load-bearing for the published privacy policy (§2.1) — v2 must
  not add payloads that violate them.

---

## 6. Dropped in v2

- `src-rn/components/AnalyticsProvider.web.tsx` — web/desktop variant of the provider.
  Differences from mobile: `app.launched` used `startType: "desktop"` (Tauri) or
  `"web"` instead of `"cold"`, defaults were derived from browser APIs, and no
  foreground/background lifecycle signals were emitted
  (v1: src/app/src-rn/components/AnalyticsProvider.web.tsx:82). Web and desktop targets
  are dropped in v2; only `startType: "cold"` remains.
- `src-rn/utils/cryptoPolyfill.ts` / `cryptoPolyfill.web.ts` — RN-runtime SHA-256 shim
  (v1 mechanism for the JS SDK's `clientUser` hashing). Not needed on native platforms.

---

## 7. Testability requirement

In v1 the analytics module is globally replaced in the test suite: Jest maps any import
of `utils/analytics` to a stub exporting no-op `td.signal`, `setDefaultPayload`, and
`trackSignal` (v1: src/app/jest.config.js:11; src/app/src-rn/utils/__mocks__/analytics.ts:1-3).
The v2 equivalent requirement: the analytics facade must be injectable/stubable so that
unit tests of stores and flows never perform network sends, and importing/initializing
app logic under test must not require live analytics (v1's module performs SDK
construction as an import side effect, which is why the whole module is mocked).

---

## 8. Discrepancies and code-wins notes

- `CLAUDE.md` does not mention analytics at all; this doc is derived purely from code.
- The settings disclosure says "keine persönlichen Daten ... erfasst" — consistent with
  the actual payloads (§5); no discrepancy found.
- Dev-build behavior (§2): signal call sites are not gated by the `__DEV__` check that
  gates the provider, so debug builds still emit non-lifecycle signals without default
  parameters. Code wins: documented as-is.
