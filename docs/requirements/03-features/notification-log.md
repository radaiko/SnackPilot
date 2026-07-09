# Notification debug log

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

A user-activatable, time-boxed diagnostic log for the notification subsystems. When the
user activates it (12 h or 24 h window), notification subsystem code — running in
background tasks AND after foreground order fetches (the reminder checks run on both
paths, v1: src/app/src-rn/store/orderStore.ts:80-84) — appends structured entries to a
persistent key-value store. After the window expires, the user can export the
log via a pre-filled e-mail to the developer, or discard it. The log is off by default and
records nothing unless explicitly activated.

Related docs (do not duplicate their content here):

- `03-features/notifications-daily-reminder.md` — semantics of the `daily-reminder` events
- `03-features/notifications-new-menu.md` — semantics of the `menu-check` events
- `03-features/notifications-location.md` — semantics of the `geofence` events
- `03-features/notifications-cancel-reminder.md` — the cancel-reminder check (wrapped by
  `order-sync` events; it emits no log entries of its own)
- `04-ui-ux.md` — shared styling/theming of the Notifications screen; `06-testing.md` —
  test strategy (the storage module has a dedicated unit-test suite)

---

## 1. Data model

(v1: src/app/src-rn/utils/notificationLogStorage.ts:8-20)

A log entry is a JSON object:

| Field | Type | Required | Meaning |
|---|---|---|---|
| `ts` | string | yes | ISO 8601 timestamp, UTC, produced at append time via `new Date().toISOString()` (v1: notificationLogStorage.ts:92) |
| `subsystem` | string enum | yes | One of `geofence`, `order-sync`, `daily-reminder`, `menu-check` (v1: notificationLogStorage.ts:8) |
| `level` | string enum | yes | One of `info`, `guard`, `error`, `notification` (v1: notificationLogStorage.ts:9) |
| `event` | string | yes | Short machine-readable tag, e.g. `time_guard_fail` |
| `detail` | string | no | Human-readable context, e.g. `currentMin=510 targetMin=525 delta=15`. When no detail is passed, the key must be **absent** from the stored object, not `null`/`undefined` (v1: notificationLogStorage.ts:96; test: src/app/src-rn/__tests__/utils/notificationLogStorage.test.ts:148-155) |

Level semantics as used by callers: `info` = progress checkpoint, `guard` = a check that
prevented a notification, `error` = caught failure, `notification` = a notification was
actually scheduled/fired.

## 2. Storage

(v1: src/app/src-rn/utils/notificationLogStorage.ts)

**v1 mechanism:** React Native AsyncStorage (`@react-native-async-storage/async-storage`)
— an unencrypted, app-private, persistent string key-value store. v2 needs any equivalent
app-private persistent KV store; the two keys and their string formats below define the
contract.

Two keys (v1: notificationLogStorage.ts:4-5):

| Key | Value format |
|---|---|
| `notification_debug_log_entries` | JSON array of log-entry objects (Section 1), oldest first |
| `notification_debug_log_activated_until` | Base-10 string of a Unix epoch timestamp in **milliseconds**; the instant the recording window ends |

### 2.1 Activation window

- `activateLog(hours)` (default `hours = 24`): set
  `notification_debug_log_activated_until` to `now + hours * 60 * 60 * 1000` (epoch ms as
  string) **and delete the entries key** — activation always starts a fresh, empty log
  (v1: notificationLogStorage.ts:32-36; test: notificationLogStorage.test.ts:50-68).
- `clearLog()`: delete **both** keys (v1: notificationLogStorage.ts:38-41).
- Reading the activation timestamp: missing key → `null`; a value that does not parse as a
  number (`Number(raw)` is `NaN`) → `null` (v1: notificationLogStorage.ts:25-30; test:
  notificationLogStorage.test.ts:34-47).
- The log counts as active iff `now < until` (strict); at or after `until`, and whenever
  the timestamp is missing or corrupt, it is inactive (v1: notificationLogStorage.ts:43-47,
  83).

### 2.2 Appending entries

`appendLogEntry(subsystem, level, event, detail?)` (v1: notificationLogStorage.ts:70-104):

1. Read both keys (v1 batches them in a single `AsyncStorage.multiGet` to narrow — not
   eliminate — the read-modify-write race between concurrent appends; documented as
   acceptable entry loss for a diagnostic log, v1: notificationLogStorage.ts:61-68, 77).
2. **Activation guard:** if the activation timestamp is missing, `0`, non-numeric, or
   `now >= until`, return without writing anything (v1: notificationLogStorage.ts:82-83;
   tests: notificationLogStorage.test.ts:117-132).
3. Parse the stored entries array; if the JSON is corrupt, start from an empty array
   (silently discarding the corrupt data) (v1: notificationLogStorage.ts:86-89; test:
   notificationLogStorage.test.ts:157-166).
4. Build the entry (`ts` = current time as ISO 8601; omit `detail` key when not provided),
   append it, and keep only the **last 200** entries — i.e. `[...entries, entry].slice(-200)`
   with `MAX_ENTRIES = 200`; the oldest entries are dropped (v1: notificationLogStorage.ts:22,
   91-100; test: notificationLogStorage.test.ts:168-181).
5. Persist the array as JSON under the entries key.

**Never throws.** Any storage or parse error is swallowed; callers (background tasks) must
be able to fire-and-forget (v1: notificationLogStorage.ts:76, 101-103; test:
notificationLogStorage.test.ts:183-186).

There is no rotation beyond the 200-entry cap and no automatic deletion at expiry: when
the window lapses, appends simply become no-ops and the recorded entries **remain stored**
until the user discards the log or re-activates it (re-activation wipes them, Section 2.1).

### 2.3 Reading entries

`getLogEntries()`: missing key → `[]`; corrupt JSON → `[]`; otherwise the parsed array
(v1: notificationLogStorage.ts:51-59; tests: notificationLogStorage.test.ts:99-114).

Note: v1 also exports `isLogActive()` (v1: notificationLogStorage.ts:43-47), but no
production code outside the module calls it — the UI recomputes activity from the raw
timestamp and `appendLogEntry` checks the window internally. v2 may omit it.

## 3. What gets logged

Log calls only produce entries while the window is active (Section 2.2); the call sites
run unconditionally and rely on the append guard. All call sites in v1, with exact event
tags and detail formats (behavioral semantics live in the owning docs listed at the top):

### `daily-reminder` (v1: src/app/src-rn/utils/dailyReminderCheck.ts)

| Level | Event | Detail (exact template) | When |
|---|---|---|---|
| info | `check_start` | — | check entered (dailyReminderCheck.ts:29) |
| guard | `disabled` | — | reminder toggle off (:33) |
| guard | `no_time_configured` | — | no reminder time stored (:39) |
| guard | `past_target_time` | `currentMin=${currentMin} targetMin=${targetMin}` | current Vienna minute-of-day ≥ target (:53-54) |
| guard | `no_orders_today` | `date=${todayKey} totalOrders=${orders.length}` | no order for today (:65-66) |
| notification | `scheduled` | `date=${todayKey} orderCount=${todayOrders.length} targetTime=${time.hour}:${time.minute}` | local notification scheduled (:78-79) |

`targetTime` interpolates the raw hour/minute numbers with **no zero-padding** — a reminder
at 11:00 logs `targetTime=11:0` (v1: dailyReminderCheck.ts:79).

### `geofence` (v1: src/app/src-rn/utils/notificationTasks.ts, geofence task)

| Level | Event | Detail (exact) | When |
|---|---|---|---|
| error | `task_error` | `error.message` | geofence task invoked with an error (notificationTasks.ts:31) |
| info | `region_enter` | `isAtCompany=true` | region enter event (:37) |
| guard | `has_order_today` | `ordersLoaded=${orders.length}` | enter, but order already exists (:49-50) |
| info | `notification_scheduled` | `scheduled=${scheduled} ordersLoaded=${orders.length}` | enter notification scheduled (:56-57) |
| error | `enter_notify_error` | error message (`e instanceof Error ? e.message : String(e)`) | enter handling threw (:59-60) |
| info | `region_exit` | `isAtCompany=false` | region exit event (:64) |
| info | `notification_cancelled` | `region_exit` | pending notification cancelled on exit (:67) |
| error | `exit_cancel_error` | error message | exit handling threw (:72-73) |

### `order-sync` (v1: src/app/src-rn/utils/notificationTasks.ts, background order-sync task)

| Level | Event | Detail | When |
|---|---|---|---|
| info | `task_start` | — | task begins (notificationTasks.ts:84) |
| error | `reminder_check_error` | error message | `checkDailyReminder()` threw (:93-94) |
| error | `cancel_reminder_check_error` | error message | `checkCancelReminder()` threw (:101-102) |
| info | `task_complete` | — | task finished (:105) |
| error | `task_fatal` | error message | outer catch (:108-109) |

### `menu-check` (v1: src/app/src-rn/utils/backgroundMenuCheck.ts)

| Level | Event | Detail (exact) | When |
|---|---|---|---|
| info | `task_start` | — | task begins (backgroundMenuCheck.ts:34) |
| guard | `no_credentials` | — | no stored Gourmet credentials (:40) |
| guard | `demo_credentials_skip` | — | demo credentials, live check skipped (:47) |
| info | `login_start` | — | before Gourmet login (:52) |
| info | `login_success` | — | after login (:55) |
| info | `menus_fetched` | `count=${items.length}` | after menu fetch (:58-59) |
| info | `comparison_result` | `hasNew=${hasNew} alreadySent=${alreadySent} currentCount=${currentFingerprints.size} knownCount=${knownMenus.size}` | fingerprint comparison done (:69-70) |
| notification | `fired` | `new menus detected` | new-menu notification fired (:85-86) |
| info | `notification_flag_reset` | `menus unchanged, reset for next batch` | sent-flag reset (:95-96) |
| guard | `no_notification` | `hasNew=${hasNew} alreadySent=${alreadySent}` | no notification warranted (:99-100) |
| error | `task_error` | error message | outer catch (:103-104) |

## 4. E-mail export format

`formatLogForEmail(entries)` (v1: notificationLogStorage.ts:108-117; test:
notificationLogStorage.test.ts:189-205):

- Empty log → the exact string `(keine Einträge aufgezeichnet)`.
- Otherwise one block per entry, blocks joined with `\n`:
  - Line 1: `[${ts}] [${subsystem}] [${LEVEL}] ${event}` where `LEVEL` is the level
    uppercased (e.g. `INFO`, `GUARD`, `ERROR`, `NOTIFICATION`).
  - If the entry has a `detail`: a second line consisting of exactly two spaces followed
    by the detail text (`\n  ${detail}`).

Example (asserted verbatim in the test, notificationLogStorage.test.ts:200-203):

```
[2026-01-01T08:00:00Z] [geofence] [INFO] enter
[2026-01-01T08:05:00Z] [order-sync] [ERROR] fail
  status=500
```

## 5. Log screen UX

The debug log lives in the "Benachrichtigungs-Log" section at the bottom of the
Notifications settings screen (route `/notifications`, opened from Settings; the screen's
back affordance is a chevron labelled `Einstellungen` and the page title is
`Benachrichtigungen`). The same screen hosts the daily-reminder and location-notification
sections — those are specified in their owning docs. (v1: src/app/app/notifications.tsx:211-356)

Section header: `Benachrichtigungs-Log`. Hint text below it:
`Zeichnet 24 Stunden lang Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.`
(v1: notifications.tsx:306-309). Note the hint's "24 Stunden" is inaccurate copy — a
12-hour option exists (code wins).

### 5.1 State model

The screen holds the raw activation timestamp (`until`, epoch ms or null) plus the loaded
entries and derives (v1: notifications.tsx:80-84):

- **Inactive**: `until === null`
- **Active**: `until !== null && now < until`
- **Expired**: `until !== null && now >= until`

State is (re)loaded from storage on mount and again **every time the screen gains focus**,
so a user returning after the window lapsed sees the Expired state without restarting the
app (v1: notifications.tsx:86-114). Entries are only loaded when `until !== null`
(v1: notifications.tsx:89-92).

The raw entries are never rendered on screen — the UI shows only counts and the
expiry time; entry content leaves the device solely through the e-mail export.

### 5.2 Inactive state

Two equal-width primary buttons side by side: `12 Stunden` and `24 Stunden`
(v1: notifications.tsx:340-355). Tapping one calls `activateLog(12)` / `activateLog(24)`
(which also wipes any previous entries, Section 2.1) and reloads the section state
(v1: notifications.tsx:184-187).

### 5.3 Active state

A single status line, no buttons:
`Aufzeichnung läuft bis {expiry}` followed by ` ({n} Einträge)` only when `n > 0`
(v1: notifications.tsx:311-323). `{expiry}` is the activation-until timestamp formatted
with locale `de-AT` and options `{ day: '2-digit', month: '2-digit', hour: '2-digit',
minute: '2-digit' }` (device-local time zone), e.g. `08.07., 14:30`
(v1: notifications.tsx:315-320). There is deliberately no way to stop, extend, or send the
log from the UI while recording is active.

### 5.4 Expired state

(v1: notifications.tsx:324-339)

- Status line: `Aufzeichnung abgeschlossen ({n} Einträge).`
- Primary button with a leading mail icon (v1: Ionicons `mail-outline`, white, on the
  primary button): label `Log per E-Mail senden`.
- Below it a borderless, tertiary-colored text button: `Log verwerfen`.

**Send** (v1: notifications.tsx:189-203): opens the OS mail composer pre-filled with

- recipients: `['aiko@spitzbub.app']`
- subject: `` `SnackPilot Notification Log (bis ${expiryStr})` `` where `expiryStr` is
  `new Date(until).toLocaleString('de-AT')` (full default de-AT date+time formatting —
  unlike the truncated format in the Active status line)
- body: the plain-text export of Section 4

**v1 mechanism:** `expo-mail-composer` `composeAsync`. If composing fails (e.g. no mail
account configured), show an alert dialog titled `Fehler` with message
`E-Mail-App konnte nicht geöffnet werden.` Sending does **not** clear or alter the stored
log; the user can send repeatedly and must discard explicitly.

**Discard** (`Log verwerfen`, v1: notifications.tsx:205-209): calls `clearLog()` (deletes
both keys) and resets the section to the Inactive state. There is **no confirmation
dialog**.

### 5.5 Analytics

The log feature emits **no** analytics signals — none of activate/send/discard call
`trackSignal` (v1: notifications.tsx:184-209; contrast with the reminder/location handlers
on the same screen which do).

## Dropped in v2

- The screen's desktop-compact styling branches (`isCompactDesktop`, `useFlatStyle`
  throughout `createStyles`, v1: notifications.tsx:360-510) target the Tauri desktop
  build — dropped; only the mobile styling path applies.
- On non-native platforms v1 skips the mount-time load via an `isNative()` guard
  (v1: notifications.tsx:95-96) — moot in v2 (mobile only).
- The web variants of the notification tasks (`notificationTasks.web.ts` etc.) never log —
  web target dropped.

## Notes for v2 implementers

- The append path must be callable from headless background-task contexts (geofence
  callbacks, background fetch) with no UI attached, and must never propagate errors into
  those tasks.
- The 200-entry cap is applied on every append (`slice(-200)`), so even a store that
  somehow holds more than 200 entries is trimmed back to exactly 200 by the next append
  (test: notificationLogStorage.test.ts:168-181).
- Persisted `ts` values are UTC ISO 8601; only the UI-facing expiry strings use de-AT
  local formatting.
