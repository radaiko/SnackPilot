# New-menu notifications (background check + fingerprinting)

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

Notify the user when the canteen imports new or changed menus so they don't miss ordering
(GitHub issue #13). Two detection paths share one fingerprint algorithm and one persisted
state:

1. **Background path** — an OS-scheduled background task logs into Gourmet, scrapes the
   menus, compares fingerprints against the persisted known set, and fires a **local**
   notification if anything is new. No server or remote-push infrastructure is involved.
2. **Foreground path** — after each `triggerRefresh`-initiated menu fetch (Menus tab
   focus or auth becoming `authenticated`, §4; the post-order-submit menu refresh does
   NOT run detection — it lives outside `triggerRefresh`,
   v1: src/app/app/(tabs)/index.tsx:87-138), the same comparison runs; a transient
   in-app toast is shown instead of an OS notification, and the current menus are
   acknowledged as "known".

Mobile only. v1 explicitly no-ops on web (`if (Platform.OS === 'web') return;`,
v1: src/app/src-rn/utils/backgroundMenuCheck.ts:120,135). **Dropped in v2:** web/desktop
targets; only the iOS/Android behavior below is required.

Related docs (do not restate here):

- `01-gourmet-scraping.md` — the exact HTTP login sequence and menu-page scraping used by
  `GourmetApi.login()` / `GourmetApi.getMenus()` (v1: src/app/src-rn/api/gourmetApi.ts).
- `03-features/menus.md` — the `GourmetMenuItem` model whose fields feed the fingerprint.
- `03-features/demo-mode.md` — demo credentials that must never reach the live server.
- `03-features/notification-log.md` — the opt-in diagnostic log written via
  `appendLogEntry` (subsystem `'menu-check'`).
- `03-features/analytics.md` — `trackSignal` semantics.
- `05-platform-services.md` — credential storage, notification permissions, background-task
  platform configuration (app.json / Info.plist / Android permissions).

---

## 1. Fingerprint algorithm (exact)

Source: v1: src/app/src-rn/utils/menuFingerprint.ts

### 1.1 Fingerprint map

`computeFingerprints(items)` maps a list of `GourmetMenuItem` to a `Map<string, string>`
(v1: src/app/src-rn/utils/menuFingerprint.ts:10-17):

- **Key** (composite): `` `${item.id}|${localDateKey(item.day)}` ``
- **Value**: `` `${item.title}|${item.subtitle}|${item.allergens.join(',')}` ``

`localDateKey(date)` formats a `Date` in the **device-local timezone** as
`YYYY-MM-DD` with zero-padded month/day (v1: src/app/src-rn/utils/dateUtils.ts:45-50):

```ts
const y = date.getFullYear();
const m = String(date.getMonth() + 1).padStart(2, '0');
const d = String(date.getDate()).padStart(2, '0');
return `${y}-${m}-${d}`;
```

The key is composite (`id|date`) **on purpose**: Gourmet menu IDs are per-category, not
per-item — the same ID appears on different days with different content
(v1: src/app/src-rn/utils/menuFingerprint.ts:7-8). Fields used from the menu item: `id`,
`day` (a `Date`), `title`, `subtitle`, `allergens` (string array, joined with `,` and no
spaces). Other item fields (`available`, `ordered`, `category`, `price`) do **not**
participate in the fingerprint.

Test-pinned examples (v1: src/app/src-rn/__tests__/utils/menuFingerprint.test.ts:27-64):

- Item `{id:'menu-001', day: 2026-02-10, title:'MENU I', subtitle:'Schnitzel mit Reis', allergens:['A','G']}`
  → entry `'menu-001|2026-02-10'` → `'MENU I|Schnitzel mit Reis|A,G'`.
- Same ID on two days produces two separate entries.
- Same ID on the same day: **last item wins** (map overwrite).

### 1.2 Change detection

`detectNewMenus(current, known)` returns a boolean
(v1: src/app/src-rn/utils/menuFingerprint.ts:23-37):

1. If `current.size === 0` → `false` (an empty scrape never counts as "new").
2. Else if `known.size === 0` → `true` (first ever run: everything is new).
3. Else: `true` iff any key in `current` is absent from `known` **or** present with a
   different fingerprint value. Keys present in `known` but missing from `current`
   (menus removed) do **not** count as a change
   (v1: src/app/src-rn/__tests__/utils/menuFingerprint.test.ts:91-95).

### 1.3 Serialization

The map is persisted as JSON of the entries array
(v1: src/app/src-rn/utils/menuFingerprint.ts:40-52):

- `serializeKnownMenus(map)` = `JSON.stringify(Array.from(map.entries()))`
  (i.e. `[["key","value"],...]`).
- `deserializeKnownMenus(json)`: `null`/empty input → empty map; unparseable JSON → empty
  map (swallow the error); otherwise `new Map(JSON.parse(json))`.

## 2. Persisted state

Source: v1: src/app/src-rn/utils/menuChangeStorage.ts. v1 mechanism: AsyncStorage
(plain key-value store, not the secure credential store). v2 needs an equivalent
unencrypted local KV store shared by the background task and the UI process.

| Key (exact string) | Value | Meaning |
|---|---|---|
| `known_menu_fingerprints` | serialized fingerprint map (§1.3) | Last acknowledged menu set (v1: menuChangeStorage.ts:4) |
| `menu_notification_sent` | string `"true"` / `"false"` (via `String(bool)`; read as `value === 'true'`, missing ⇒ `false`) | One-notification-per-batch latch (v1: menuChangeStorage.ts:5,16-23) |

## 3. Background check task

Source: v1: src/app/src-rn/utils/backgroundMenuCheck.ts

### 3.1 Registration & cadence (v1 mechanism)

- v1 mechanism: `expo-background-task` + `expo-task-manager`. The task is **defined at
  module load time** under the exact name `BACKGROUND_MENU_CHECK`
  (v1: backgroundMenuCheck.ts:18,110-112). The module is loaded (via conditional
  `require`) from the root layout **only on native platforms**
  (v1: src/app/app/_layout.tsx:26-28).
- `registerBackgroundMenuCheck()` (v1: backgroundMenuCheck.ts:119-128):
  - Returns immediately on web (**Dropped in v2** — v2 is mobile-only).
  - No-op if the task is already registered.
  - Otherwise registers with `minimumInterval: 15`. Unit is **minutes** (per
    expo-background-task v55: "Inexact interval in minutes … the system controls the
    execution interval and treats the value as a minimum delay"; iOS often runs such
    tasks only in system-chosen windows, e.g. overnight). The cadence is therefore
    OS-controlled and best-effort — v2 should use the platform-native equivalent
    (iOS `BGTaskScheduler` processing task, Android WorkManager periodic work,
    ≥ 15-minute minimum) with the same "at most as often as the OS allows" semantics.
- **Trigger for registration**: an effect in the root layout runs whenever the Gourmet
  auth status becomes `'authenticated'`; it calls `registerBackgroundMenuCheck()` then
  `requestNotificationPermissions()`, all errors swallowed (best-effort)
  (v1: src/app/app/_layout.tsx:49-59). There is no unregistration path anywhere — once
  registered, the task stays registered even after logout/credential deletion (the task
  itself guards on missing credentials, §3.3 step 2).
- iOS platform config required for the task to run: `UIBackgroundModes: ["processing"]`
  and `BGTaskSchedulerPermittedIdentifiers: ["com.expo.modules.backgroundtask.processing"]`
  (v1 mechanism — expo-background-task's fixed identifier; v2 will define its own);
  Android: `RECEIVE_BOOT_COMPLETED`, `WAKE_LOCK` permissions
  (v1: src/app/app.json — see 05-platform-services).

### 3.2 Notification permission request

`requestNotificationPermissions()` in this module (v1: backgroundMenuCheck.ts:134-142):
web → `false` (**Dropped in v2**); if current permission status is `'granted'` → `true`
without prompting; otherwise request permissions (no iOS options object here, unlike the
variant in notificationService.ts:40-47) and return whether granted. Called after every
auth transition to authenticated; effective re-prompting is limited by OS behavior (iOS
shows the system prompt only once). If permission is denied, detection still runs and
state is still updated — the scheduled local notification simply won't be displayed.
There is deliberately no in-app re-prompt flow.

### 3.3 Task algorithm (exact order)

`backgroundMenuCheckTask()` (v1: backgroundMenuCheck.ts:32-107). Runs headless — no UI.
Every step below that logs uses `appendLogEntry('menu-check', level, event, detail?)`,
which is a no-op unless the diagnostic log is activated (see
03-features/notification-log.md).

1. Log `info/task_start`.
2. Read Gourmet credentials from secure storage, keys `gourmet_username` /
   `gourmet_password` (v1: src/app/src-rn/utils/constants.ts:12-13; storage mechanism in
   05-platform-services). If either is missing → log `guard/no_credentials` → return
   **Success**.
3. Demo guard: if `username.toLowerCase() === 'demo' && password === 'demo1234!'`
   (v1: constants.ts:23-28) → log `guard/demo_credentials_skip` → return **Success**.
   Demo credentials are fake and must never be sent to the live Gourmet server.
4. Log `info/login_start`; construct a fresh `GourmetApi` instance (fresh JS-side state
   only — `new GourmetApi()`, v1: backgroundMenuCheck.ts:53) and perform the full login
   sequence, then log `info/login_success`. **Note:** on native the cookies live in the
   platform's shared app-wide cookie store (gourmetClient.ts:9-11,26), so the background
   login reuses/overwrites the SAME Gourmet session cookies as the foreground app — v1
   has no isolated background session, and a background login can invalidate the
   foreground session server-side (the foreground's `ensureSession` re-login covers
   this). Login + menu scraping are byte-for-byte the flows specified in
   01-gourmet-scraping.md — this feature adds no new HTTP behavior.
5. Fetch all menus (`getMenus()`, pages 0–9 scrape per 01-gourmet-scraping.md); log
   `info/menus_fetched` with detail `` `count=${items.length}` ``.
6. Compute `currentFingerprints` (§1.1), load `knownMenus` (§2), compute
   `hasNew = detectNewMenus(currentFingerprints, knownMenus)` (§1.2), load
   `alreadySent` flag; log `info/comparison_result` with detail
   `` `hasNew=${hasNew} alreadySent=${alreadySent} currentCount=${currentFingerprints.size} knownCount=${knownMenus.size}` ``.
7. Branch (state machine):

   | `hasNew` | `alreadySent` | Action (in this order) |
   |---|---|---|
   | true | false | Schedule the notification (§3.4) → set `menu_notification_sent = true` → set `known_menu_fingerprints = current` → `trackSignal('menu.newDetected')` → log `notification/fired` (detail `new menus detected`) → **Success** (v1: backgroundMenuCheck.ts:72-88) |
   | false | true | Set `menu_notification_sent = false` → set `known_menu_fingerprints = current` → log `info/notification_flag_reset` (detail `menus unchanged, reset for next batch`) — then fall through to the no-notification exit (v1: backgroundMenuCheck.ts:92-97) |
   | true | true | No state change (suppressed: a notification for this batch was already sent; remains frozen until a foreground acknowledgment (§4) or until menus revert to the known set) |
   | false | false | No state change |

   All non-notify paths end with log `guard/no_notification` (detail
   `` `hasNew=${hasNew} alreadySent=${alreadySent}` ``) and return **Success**.
8. Any thrown error (login failure, network, parse) → log `error/task_error` with the
   error message → return **Failed** (v1: backgroundMenuCheck.ts:102-106).

Behavioral consequences to preserve:

- **One notification per batch.** After firing, `known` is updated to the notified set, so
  the next run sees `hasNew=false, alreadySent=true` and re-arms the latch automatically.
- If the menus change *again* before the latch resets (row `true|true`), no second
  notification fires and no state is written; the situation persists until the user opens
  the Menus tab (foreground ack, §4) or the site content reverts.
- Edge case: if a run scrapes zero items while `alreadySent=true`, row `false|true`
  applies and `known_menu_fingerprints` is overwritten with the **empty** map — meaning
  the next non-empty scrape counts as all-new. This is v1 behavior as coded.

### 3.4 OS notification content (exact)

Fired immediately (`trigger: null`) via the local-notifications API
(v1: backgroundMenuCheck.ts:73-81):

- **title**: `Neue Menüs verfügbar`
- **body**: `Es gibt neue Menüs. Öffne SnackPilot um sie anzusehen.`
- **data payload**: `{ screen: '/(tabs)' }`
- **Android only**: `channelId: 'menu-updates'`

Android channel: created at module load (i.e. app startup on Android, before any auth)
with id `menu-updates`, name `Neue Menüs`, importance DEFAULT
(v1: backgroundMenuCheck.ts:21-26). This is a separate channel from the order-reminder
channel (`order-reminders`, see the other notifications-* docs).

**Discrepancy (code wins):** the design doc claims "Tapping the OS notification
deep-links to the Menus tab" (v1: docs/plans/2026-02-24-new-menu-notifications-design.md:31),
and the `data.screen` payload is set accordingly — but v1 contains **no**
notification-response listener anywhere (no `addNotificationResponseReceivedListener` /
`useLastNotificationResponse` in the codebase). Tapping the notification just opens the
app; the `data.screen` payload is dead. v2 must not invent deep-linking; carrying the
payload is optional.

Foreground presentation (applies while the app is open): the app-wide notification
handler shows banner + list, plays sound, does not set badge
(v1: src/app/src-rn/utils/notificationService.ts:262-271, installed at app start on
native, v1: src/app/app/_layout.tsx:69-73).

## 4. Foreground detection + acknowledgment (Menus screen)

Source: v1: src/app/app/(tabs)/index.tsx:85-138

Triggers — `triggerRefresh()` runs:

- on every focus of the Menus tab (navigation `focus` listener; also emits
  `trackSignal('screen.viewed', { screen: 'menus' })`) (v1: index.tsx:126-132), and
- whenever Gourmet auth status becomes `'authenticated'` (v1: index.tsx:134-138).

`triggerRefresh()` sequence (v1: index.tsx:87-124):

1. Abort unless auth status is `'authenticated'`.
2. Load cached menus + cached orders (see 03-features/caching.md), errors swallowed.
3. If the store now has items → `refreshAvailability()`, else → full `fetchMenus()`
   (see 03-features/menus.md).
4. **After that fetch promise resolves**, run detection (all errors swallowed so a
   fingerprint failure never breaks menu loading):
   1. Read `items` from the menu store; if empty → stop (no state change).
   2. `currentFingerprints = computeFingerprints(items)`; load `knownMenus` and
      `notificationSent` from storage.
   3. If `detectNewMenus(currentFingerprints, knownMenus) && !notificationSent`:
      show the toast (§5) and write `menu_notification_sent = true`.
   4. **Acknowledge unconditionally**: write `known_menu_fingerprints =
      currentFingerprints`, then write `menu_notification_sent = false`
      (v1: index.tsx:114-116; note the exact write order — the `true` written in step 3
      is transient and immediately overwritten; final state after any successful
      foreground pass is always `sent=false`, `known=current`).
5. In parallel, `fetchOrders()` is kicked off (unrelated to this feature).

So opening the Menus tab is the acknowledgment mechanism: it marks everything currently
on the site as known and re-arms the OS-notification latch, exactly as the design doc
states (v1: docs/plans/2026-02-24-new-menu-notifications-design.md:30).

Observable first-run behavior (pinned by the v1 implementation plan's manual test,
v1: docs/plans/2026-02-24-new-menu-notifications.md:816-822): on the very first load
after install (empty `known` map), the toast appears once; navigating away and back
does not show it again.

## 5. In-app toast (NewMenuToast)

Source: v1: src/app/src-rn/components/NewMenuToast.tsx

- **Text**: `Neue Menüs verfügbar!` (v1: NewMenuToast.tsx:62)
- **Placement**: absolutely positioned overlay at the top of the Menus screen content
  area; `top = safe-area top inset + 8`, `left: 16`, `right: 16`, above all content
  (`zIndex: 100`), content centered, `padding: 12` (v1: NewMenuToast.tsx:61,69-77).
  Rendered in both the phone layout and the wide layout of the Menus screen
  (v1: src/app/app/(tabs)/index.tsx:349,375; wide/desktop layout **dropped in v2** —
  only the phone placement is required).
- **Style**: theme "tinted banner" surface using the theme's `glassPrimary` tint; text
  14 pt, weight 600, in the theme primary color (v1: NewMenuToast.tsx:77-83; tokens in
  03-features/themes.md / 04-ui-ux.md).
- **Animation**: slide-in from above (translateY −100 → 0) combined with fade
  (opacity 0 → 1), duration **300 ms**; stays for **4000 ms**; then the reverse
  animation (300 ms) runs and the toast dismisses itself via the `onDismiss` callback
  (v1: NewMenuToast.tsx:13-14,22-54). Not tappable; no action; auto-dismiss only.
- Controlled by a `visible` boolean; the Menus screen sets it true on detection and
  false in `onDismiss` (v1: index.tsx:85,349).

## 6. Settings gating

**There is no user-facing setting for this feature.** The notifications settings screen
(v1: src/app/app/notifications.tsx) contains toggles only for the daily order reminder,
the location-based reminder, and the diagnostic log — nothing for new-menu notifications.
This matches the design decision "No user-facing settings toggle (YAGNI)"
(v1: docs/plans/2026-02-24-new-menu-notifications-design.md:44). The feature is always
active once the effective gates pass:

1. Native platform (iOS/Android) — web no-ops (**dropped in v2**).
2. Gourmet auth: registration + permission prompt happen only after auth status becomes
   `'authenticated'` (v1: src/app/app/_layout.tsx:49-59); foreground detection only runs
   while authenticated (v1: index.tsx:88-89).
3. Stored credentials must exist and must not be the demo credentials (background task
   guards, §3.3 steps 2-3).
4. OS notification permission — gates only the visible OS notification, not detection,
   state updates, or the in-app toast.

## 7. Discrepancies (code wins)

| Claim in docs/plans | Actual v1 code |
|---|---|
| Fingerprint key is the menu ID alone (design.md:16, plan Task 2) | Composite key `id\|localDateKey(day)` (menuFingerprint.ts:13); tests pin the composite form |
| Background stack is `expo-background-fetch`, `minimumInterval: 15 * 60` seconds, `stopOnTerminate: false`, `startOnBoot: true` (plan Tasks 1, 4) | `expo-background-task` with `minimumInterval: 15` (minutes) and no other options (backgroundMenuCheck.ts:125-127) |
| Background task exits early when `notificationSent` is already true, before any network call (plan Task 4, 2026-02-24-new-menu-notifications.md:447-451; design.md's data flow actually matches the code's check-after-fetch ordering) | The task always logs in and scrapes, then consults the flag; it also has the `false\|true` reset branch that re-arms the latch (backgroundMenuCheck.ts:64-97) — neither plan nor design doc had the reset |
| Registration/permissions requested once on app start (plan Task 6: effect with `[]` deps) | Effect keyed on Gourmet auth status; runs on every transition to `'authenticated'` (_layout.tsx:49-59) |
| Tapping the OS notification deep-links to the Menus tab (design.md:31) | No notification-response handler exists; `data.screen` is unused (§3.4) |
| Toast `top: 0` (plan Task 5) | `top: insets.top + 8` at render time (NewMenuToast.tsx:61) |
| Background task returns `NoData`/`NewData` fetch results (plan Task 4) | expo-background-task has only `Success`/`Failed`; all guard paths return `Success` (backgroundMenuCheck.ts:41-105) |

## 8. v2 rebuild checklist

- Pure fingerprint module (§1) — port exactly, including empty/invalid-JSON handling;
  the Rust core is the natural home (same code serves background task and UI).
- KV persistence with the exact keys/formats of §2 (needed if v2 wants to migrate v1
  state in place; otherwise keys may change but the doc's semantics must hold).
- Background execution: iOS `BGTaskScheduler` (processing/app-refresh) and Android
  WorkManager periodic work, minimum interval 15 min, OS-controlled, registered after
  login, never unregistered, guards per §3.3.
- Local notification per §3.4 with German strings copied byte-for-byte; Android channel
  `menu-updates` / `Neue Menüs` / default importance.
- Foreground detect-and-acknowledge per §4 wired to the Menus screen's fetch lifecycle;
  toast per §5.
- No settings toggle (§6).
