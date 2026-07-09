# Daily order reminder

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

A local (on-device) notification that reminds the user of what they ordered for today, delivered
once per day at a user-configured time. Purely client-side: no HTTP requests of its own — it reads
order data already held by the orders feature (see `03-features/orders.md`). Mobile-only in v1
(iOS + Android); web/desktop had stub implementations, which are dropped in v2.

Related docs (do not duplicate their content):

- `03-features/orders.md` — order model (`date`, `title`, `subtitle` fields) and order fetching.
- `03-features/notifications-location.md` — geofence "no order" notification; shares the Android
  channel and background-sync task.
- `03-features/notifications-cancel-reminder.md` — cancel reminder; shares the settings storage
  module and background-sync task.
- `03-features/notification-log.md` — the diagnostic log this feature writes into.
- `05-platform-services.md` — notification permissions, foreground presentation handler,
  Android channel creation, background task registration mechanics.

---

## 1. Model: pre-scheduled local notification, not fire-on-check

The reminder is implemented by **scheduling an OS local notification for the target time** whenever
a "check" runs *before* that time on the same day, rather than firing immediately when a background
task happens to run near the target time (v1: src/app/src-rn/utils/dailyReminderCheck.ts:14-27).
Rationale recorded in code comments: iOS 26 `BGAppRefreshTask` can run hours late, so
immediate-fire from a background task would deliver the reminder at the wrong time of day; a
pre-scheduled local notification is delivered reliably at the configured time
(v1: src/app/src-rn/utils/dailyReminderCheck.ts:48-51, src/app/src-rn/utils/notificationService.ts:202-203).

Consequence an implementer must preserve: **if no check runs between local (Vienna) midnight and
the configured time on a given day, no reminder fires that day** — nothing is ever scheduled for
future days, only for later the same day.

## 2. Triggers — when the check runs

`checkDailyReminder()` is invoked from two places:

1. **Background order-sync task** (`BACKGROUND_ORDER_SYNC_TASK`, task name string
   `'BACKGROUND_ORDER_SYNC_TASK'`, v1: src/app/src-rn/utils/constants.ts:32): the task first loads
   cached orders from local storage (no network — deliberately avoids concurrent scraping), then
   runs the daily-reminder check; errors from the check are caught and logged, not fatal to the
   task (v1: src/app/src-rn/utils/notificationTasks.ts:82-112). The task is registered with a
   minimum interval of **15 minutes** and is skipped if the OS reports background tasks as
   Restricted (v1: src/app/src-rn/utils/notificationService.ts:77-87). v1 mechanism:
   `expo-background-task` (BGAppRefreshTask on iOS, WorkManager on Android) — actual run cadence is
   OS-controlled and best-effort.
2. **After every successful order fetch**: `orderStore.fetchOrders()` calls `checkDailyReminder()`
   (on non-web platforms) right after storing fresh orders, so the scheduled reminder content is
   updated whenever the user refreshes/changes orders in the foreground
   (v1: src/app/src-rn/store/orderStore.ts:69-86). Failures are silently swallowed there.

The background task is registered (any of):

- at app start, when the daily reminder is enabled — even without a company location
  (v1: src/app/app/_layout.tsx:80-93);
- when the user toggles the reminder ON in settings (v1: src/app/app/notifications.tsx:124);
- via `enableNotifications()` when a company location is set (shared with the location feature,
  v1: src/app/src-rn/utils/notificationService.ts:293-296).

## 3. Decision logic — guards, in exact order

`checkDailyReminder()` (v1: src/app/src-rn/utils/dailyReminderCheck.ts:28-81). All time values are
**Europe/Vienna** wall-clock (see §7). Each step also writes a diagnostic log entry (§8).

1. Read the enabled flag; if not enabled → return. (Does **not** cancel a previously scheduled
   notification — see §9.)
2. Read the configured time; if none stored/invalid → return.
3. Compute `currentMin` = current Vienna minutes-since-midnight, `targetMin = hour * 60 + minute`.
   If `currentMin >= targetMin` (at or past target time) → return. This is the "late background
   task" guard (v1: src/app/src-rn/utils/dailyReminderCheck.ts:52-56).
4. Filter the in-memory order list to orders whose `date` is the same calendar day as Vienna-today.
   There is **no filter on approval status** — unapproved orders count. If zero orders today →
   **cancel any previously scheduled reminder notification** (errors ignored) and return
   (v1: src/app/src-rn/utils/dailyReminderCheck.ts:58-68).
5. Build the notification body (§6) and schedule the notification for the target time. The
   scheduling function independently re-checks `currentMin >= targetMin` and returns `false`
   without scheduling in that case (v1: src/app/src-rn/utils/notificationService.ts:205-232).
6. If scheduling succeeded, write today's date key (format `YYYY-MM-DD`) to the "sent date" storage
   key (v1: src/app/src-rn/utils/dailyReminderCheck.ts:76-80).

**No weekday condition exists.** The reminder can fire on any day of the week; in practice it is
gated only by "has an order for today".

## 4. Scheduling semantics

`scheduleDailyReminderNotification(targetHour, targetMinute, body)`
(v1: src/app/src-rn/utils/notificationService.ts:205-232):

- Returns `false` (no scheduling, caller must not mark sent) if Vienna
  `currentMin >= targetHour*60 + targetMinute`.
- Otherwise schedules a **date-triggered** local notification at
  `now + (targetMin − currentMin) minutes`. Because `currentMin` has minute granularity
  (v1: src/app/src-rn/utils/dateUtils.ts:66-77), the actual fire instant inherits the current
  seconds-of-minute (fires at `HH:MM:ss` where `ss` is the seconds at scheduling time).
- Uses the **fixed notification identifier `daily-order-reminder`**
  (`DAILY_REMINDER_NOTIFICATION_ID`, v1: src/app/src-rn/utils/constants.ts:38). Re-scheduling with
  the same identifier **replaces** any previously pending reminder — this is the primary dedupe
  mechanism (v1: src/app/src-rn/utils/notificationService.ts:197-199).
- Android: delivered on channel `order-reminders` (`NOTIFICATION_CHANNEL_ID`,
  v1: src/app/src-rn/utils/constants.ts:36); channel display name `Bestellungs-Erinnerungen`,
  importance HIGH, vibration pattern `[0, 250, 250, 250]`
  (v1: src/app/src-rn/utils/notificationService.ts:273-281).

`cancelDailyReminderNotification()` cancels the pending scheduled notification by that same
identifier (v1: src/app/src-rn/utils/notificationService.ts:237-239).

## 5. Once-per-day / dedupe behavior (actual, code wins)

- **At most one pending reminder** exists at any time, because every schedule call reuses the
  identifier `daily-order-reminder` and replaces the previous one.
- **Before the target time**, every check re-schedules (replacing), so the body always reflects the
  most recently known orders. A check that finds zero orders for today cancels the pending
  notification (order cancelled before target time → no reminder).
- **At/after the target time**, every check returns early (guard 3), so nothing is re-scheduled and
  the already-fired notification cannot fire again that day. Checks after Vienna midnight target the
  new day.
- The storage key `daily_reminder_sent_date` is **written after successful scheduling but never
  read by any production code** — `getReminderSentDate()` exists in the storage module but has no
  production caller (v1: src/app/src-rn/utils/reminderStorage.ts:39-45,
  src/app/src-rn/utils/dailyReminderCheck.ts:3-7). It is vestigial from the earlier fire-on-check
  design (§10). v2 need not persist it to reproduce behavior; if dropped, note it in the data
  migration story.
- Test-encoded expectations (v1: src/app/src-rn/__tests__/utils/dailyReminderCheck.test.ts):
  - Re-schedules before target time **even if the sent-date equals today** ("reschedules before
    notification time even if sentDate matches today", lines 97-110).
  - Skips when past target time even if never sent ("iOS 26 late-BG-task fix", lines 158-170).
  - Schedules regardless of how far before the target time the check runs — there is **no ±15-min
    window** ("schedules regardless of current time (no time window)", lines 208-221).
  - Two checks before target time with changed orders → two schedule calls, last one with the new
    content (lines 172-192).

## 6. Notification content

(v1: src/app/src-rn/utils/notificationService.ts:217-230,
src/app/src-rn/utils/dailyReminderCheck.ts:70-72)

| Field | Exact value |
|---|---|
| Identifier | `daily-order-reminder` |
| Title | `Deine Bestellung heute` |
| Body | One line per today's order, joined with `\n`. Per order: `` `${title} — ${subtitle}` `` — i.e. title, space, **em dash U+2014**, space, subtitle. If `subtitle` is falsy (empty string or absent), the line is the title alone with no separator. |
| Sound | `default` |
| Data payload | `{ screen: '/(tabs)/orders' }` |
| Android channel | `order-reminders` |

Body examples (from tests, v1: src/app/src-rn/__tests__/utils/dailyReminderCheck.test.ts:137-156,
223-247):

- Single order: `MENÜ II — Wiener Schnitzel`
- Multiple orders: `MENÜ II — Wiener Schnitzel\nSUPPE & SALAT — Tomatensuppe`
- Order without subtitle: `MENÜ I`

Order of lines = order of the store's order array filtered to today (no re-sorting,
v1: src/app/src-rn/utils/dailyReminderCheck.ts:59, 70-72).

**Tap behavior discrepancy (code wins):** the design doc says tapping opens the Orders tab, and the
`data.screen` payload is set accordingly, but **no code registers a notification-response listener
or reads `data.screen`** anywhere in v1 — tapping merely opens the app via OS default behavior.
(Design claim: v1: docs/plans/2026-02-25-daily-order-reminder-design.md:18, 58; no consumer found
in `src/app/**`.) v2 may implement the navigation, but must not assume v1 had it.

Foreground presentation (v1 mechanism, shared across all notifications): the app's notification
handler shows banner + list and plays sound while the app is foregrounded, no badge
(v1: src/app/src-rn/utils/notificationService.ts:262-271).

## 7. Time and date semantics

All in **Europe/Vienna**, regardless of device timezone:

- `viennaMinutes()` — current Vienna time as minutes since midnight, minute granularity
  (v1: src/app/src-rn/utils/dateUtils.ts:66-77).
- `viennaToday()` — the current Vienna calendar date, materialized as a local-midnight `Date`
  (v1: src/app/src-rn/utils/dateUtils.ts:80-86).
- `isSameDay(a, b)` — compares local year/month/day components
  (v1: src/app/src-rn/utils/dateUtils.ts:55-61).
- `localDateKey(date)` — `YYYY-MM-DD` from local date components, zero-padded, explicitly *not*
  UTC-based (v1: src/app/src-rn/utils/dateUtils.ts:40-50).

So the configured reminder time is interpreted as Vienna wall-clock time, and "today" is the
Vienna calendar date.

## 8. Diagnostic log events

Every check writes to the notification log (subsystem `daily-reminder`; mechanism in
`03-features/notification-log.md` — entries are only persisted while the log is activated). Exact
events, in the order they can occur (v1: src/app/src-rn/utils/dailyReminderCheck.ts:29-79):

| Level | Event | Detail (exact template) |
|---|---|---|
| `info` | `check_start` | — |
| `guard` | `disabled` | — |
| `guard` | `no_time_configured` | — |
| `guard` | `past_target_time` | `currentMin=${currentMin} targetMin=${targetMin}` |
| `guard` | `no_orders_today` | `date=${todayKey} totalOrders=${orders.length}` |
| `notification` | `scheduled` | `date=${todayKey} orderCount=${todayOrders.length} targetTime=${time.hour}:${time.minute}` (note: hour/minute **not** zero-padded, e.g. `targetTime=11:0`) |

If `checkDailyReminder()` itself throws inside the background task, the task logs
(`order-sync`, `error`, `reminder_check_error`, error message) and continues
(v1: src/app/src-rn/utils/notificationTasks.ts:90-95).

## 9. Settings and persistence

### Storage (v1 mechanism: AsyncStorage — unencrypted app-local key-value store)

(v1: src/app/src-rn/utils/reminderStorage.ts:1-45; behavior locked by
src/app/src-rn/__tests__/utils/reminderStorage.test.ts)

| Key | Format | Semantics |
|---|---|---|
| `daily_reminder_enabled` | string `"true"` / `"false"` (via `String(enabled)`) | Getter returns `value === 'true'`; default (nothing stored) = **false**. |
| `daily_reminder_time` | JSON object, e.g. `{"hour":11,"minute":30}` | Getter returns `{ hour, minute }` only if the stored value parses to a non-null object with numeric `hour` and numeric `minute`; otherwise (missing, parse error, wrong shape) returns null → treated as "not configured". |
| `daily_reminder_sent_date` | `YYYY-MM-DD` (from `localDateKey`) | Written after successful scheduling; **never read** (see §5). |

There is **no default time**: until the user enables the feature, no time is stored
(v1: docs/plans/2026-02-25-daily-order-reminder-design.md:13, matching code — the
`no_time_configured` guard).

### Settings UI

Lives on the "Benachrichtigungen" screen (pushed from the Settings tab), section
"Bestell-Erinnerung" with hint text `Tägliche Erinnerung an deine Bestellung`
(v1: src/app/app/notifications.tsx:221-272). UI-chrome details are covered by `04-ui-ux.md`;
behaviorally required:

- **Toggle** (label `Aktiviert`). Turning ON (v1: src/app/app/notifications.tsx:116-133):
  1. Request notification permission (iOS options: allowAlert, allowBadge, allowSound;
     v1: src/app/src-rn/utils/notificationService.ts:40-47). If denied → show alert titled
     `Berechtigung fehlt` with message
     `Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.`
     and abort (toggle stays off).
  2. Persist the currently selected picker time (defaults to 11:00 if the user never touched the
     picker) **before** setting the enabled flag.
  3. Register the background sync task.
  4. Set `daily_reminder_enabled` = true.
  5. Emit analytics signal `notification.reminderToggled` with string properties
     `enabled`, `hour`, `minute` (see `03-features/analytics.md`).
  Turning OFF: only sets `daily_reminder_enabled` = false and emits the same analytics signal —
  it does **not** cancel an already-scheduled notification for today, does not unregister the
  background task, and requests no permissions (see §10 behavior notes).
- **Time picker**: horizontal row of chips in 15-minute increments restricted to
  **11:00–13:45** (hours 11, 12, 13 × minutes 0/15/30/45 → 12 options), labels zero-padded `HH:MM`
  (v1: src/app/app/notifications.tsx:51-60). Shown only while the toggle is on. Selecting a chip
  persists the time immediately (v1: src/app/app/notifications.tsx:135-139); the pending
  notification is *not* immediately re-scheduled — it updates on the next check (next order fetch
  or background run).
- **Android-only hint banner** below the picker: text
  `Damit Erinnerungen zuverlässig funktionieren, muss die Hintergrundaktivität für diese App erlaubt sein. App-Einstellungen öffnen`
  — tapping opens the OS app-settings page (v1: src/app/app/notifications.tsx:262-270).
- On screen load, the toggle/picker state is hydrated from storage; picker defaults to 11:00 when
  no time is stored (v1: src/app/app/notifications.tsx:75-77, 95-107).

## 10. Behavior notes, edge cases, and discrepancies

Design-doc vs. code discrepancies — **code wins** in all cases:

1. **Windowed fire-on-check vs. pre-scheduling.** Both plan docs
   (v1: docs/plans/2026-02-25-daily-order-reminder-design.md:23-36,
   docs/plans/2026-02-25-daily-order-reminder.md:435-482) describe firing immediately when a
   background run lands within ±15 min of the configured time, deduped by reading the sent-date.
   Shipped code pre-schedules at the target time, skips when past it, and never reads the
   sent-date (§1, §5).
2. **Storage keys.** Design doc lists `daily_reminder_hour` / `daily_reminder_minute` as separate
   keys (v1: docs/plans/2026-02-25-daily-order-reminder-design.md:44-46); code uses the single
   JSON key `daily_reminder_time` (§9).
3. **Picker range.** Design doc implies full-day 15-min options
   (v1: docs/plans/2026-02-25-daily-order-reminder.md:682-694); code restricts to 11:00–13:45 (§9).
4. **Tap action.** Design doc: opens Orders tab; code: payload set but never consumed (§6).

v1 behavior an implementer should reproduce (or consciously change with a note):

- **Disabling does not retract.** Toggling the reminder off does not cancel a notification already
  scheduled for later today; it will still fire once. Only the `no_orders_today` path cancels a
  pending notification (v1: src/app/src-rn/utils/dailyReminderCheck.ts:32-35, 60-68).
- **Late order cancellation cannot retract.** The `past_target_time` guard runs before the
  no-orders check, so once the target time passes nothing cancels — but by then the notification
  has already fired, so this is moot in practice.
- **Cross-feature cancellation hazard:** removing the company location (location-notifications
  feature) calls `disableNotifications()`, which unregisters the shared background-sync task and
  calls `cancelAllScheduledNotificationsAsync()` — cancelling **all** pending notifications
  including a scheduled daily reminder; the background task is only re-registered on next app
  launch, reminder toggle, or when a company location is set again (setting a location calls
  `enableNotifications()` → `registerBackgroundSync()`,
  v1: src/app/app/notifications.tsx:170, src/app/src-rn/utils/notificationService.ts:293-296)
  (v1: src/app/src-rn/utils/notificationService.ts:298-302,
  src/app/app/notifications.tsx:179-182, src/app/app/_layout.tsx:80-93).
- **Stale content window:** the fired body reflects orders as of the last check before the target
  time (typically ≤15 min stale via the background task, or fresher if the user opened the app).
- Enabled-without-time is unreachable via the UI (time is persisted before the flag) but the
  `no_time_configured` guard still protects against it.

## 11. Dropped in v2

- Web/desktop: v1 shipped no-op web stubs (`notificationService.web.ts`,
  `notificationTasks.web.ts`). The actual native-only gates were: the Settings-tab entry to the
  Benachrichtigungen screen (v1: src/app/app/(tabs)/settings.tsx:104), the screen's
  state-hydration effect (`if (!isNative()) return;`, v1: src/app/app/notifications.tsx:95-107),
  the background task definition (v1: src/app/src-rn/utils/notificationTasks.ts:20), and the web
  permission stub returning `false` so the toggle could never be enabled — the reminder UI itself
  rendered unconditionally if reached. v2 is mobile-only; implement natively (e.g.
  UNUserNotificationCenter on iOS, AlarmManager/WorkManager + NotificationManager on Android)
  preserving the semantics above.
- The `expo-background-task` / `expo-notifications` machinery is a v1 mechanism; v2 must preserve
  the observable contract: a ~15-minute-interval background refresh opportunity that runs the check
  in §3, plus the on-fetch trigger in §2.
