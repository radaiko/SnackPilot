# Cancel reminder

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

A local (on-device) notification that warns the user: *you have an order for today but you are not
at the office — do you want to cancel it?* It targets 08:45 Europe/Vienna and is hard-gated by
Gourmet's 09:00 cancellation deadline. Purely client-side: no HTTP requests of its own — it reads
order data already held by the orders feature and the geofence-derived "at company" flag from the
location feature. It is active only while a company location is saved; without one the app cannot
know whether the user is in the office, so it schedules nothing and retracts any pending reminder.
Mobile-only in v1 (iOS + Android).

There is **no design doc in `docs/plans/` for this feature**; the code is the only specification.

Related docs (do not duplicate their content):

- `03-features/orders.md` — order model (`date` field, cached order list) and the 09:00
  cancellation cutoff in the UI.
- `03-features/notifications-location.md` — company location setup, geofencing, the `isAtCompany`
  flag, and the geofence "no order" notification (which shares the 08:45 target time).
- `03-features/notifications-daily-reminder.md` — daily reminder; shares the background-sync task,
  the Android channel, Vienna time semantics (§7 there), and the foreground presentation handler.
- `03-features/notification-log.md` — the diagnostic log written by the surrounding tasks.
- `05-platform-services.md` — notification permissions, foreground presentation handler, Android
  channel creation, background task registration mechanics.

---

## 1. Model

Like the daily reminder, the cancel reminder is a **pre-scheduled local notification**: whenever a
"check" runs before 08:45 Vienna time with the trigger conditions met, an OS local notification is
scheduled for 08:45 today; whenever a check runs with the conditions *not* met, any pending
notification is cancelled. Between 08:45 and 09:00 the notification fires immediately instead; at
or after 09:00 nothing is scheduled at all
(v1: src/app/src-rn/utils/cancelReminderCheck.ts:9-36,
src/app/src-rn/utils/notificationService.ts:147-187).

Consequence to preserve: **if no check runs between Vienna midnight and 09:00 on a given day, no
cancel reminder fires that day.** Nothing is ever scheduled for future days.

## 2. Decision logic — `checkCancelReminder()`

Exact algorithm (v1: src/app/src-rn/utils/cancelReminderCheck.ts:22-36):

1. Read the saved company location, `isAtCompany` and the in-memory `orders` array from the
   location/order stores (orders state as-is; this function performs no loading itself — callers
   are responsible for populating the order store first, see §3).
2. `hasOrderToday` = any order whose `date` is the same calendar day as `viennaToday()`
   (component-wise year/month/day comparison via `isSameDay`,
   v1: src/app/src-rn/utils/dateUtils.ts:55-61, 80-86). There is **no filter on approval status** —
   unapproved orders count.
3. If **no company location is saved**, `isAtCompany === true` **or** `hasOrderToday === false` →
   cancel any pending cancel-reminder notification (cancellation errors are swallowed — "may not
   exist") and return.
4. Otherwise (location saved, not at company **and** has an order today) → call
   `scheduleCancelReminderNotification()` (§4). Errors from scheduling propagate to the caller
   (each caller catches them, §3).

The check reads **no settings and no notification permission**: no enabled flag, no configured
time, no notification-permission check. Gating is structural — a saved company location (§7) and
the time guards in the scheduler (§4).

`isAtCompany` semantics (owned by `03-features/notifications-location.md`, summarized here because
it is this feature's core input): initial value `false`; set `true`/`false` only by geofence
Enter/Exit events and reset to `false` when the company location is cleared; **persisted** together
with the company location under the storage key `company-location` (v1 mechanism: Zustand
`persist` over AsyncStorage, whole-state, no partialize), so it survives app restarts as the "last
known location" (v1: src/app/src-rn/store/locationStore.ts:20-43,
src/app/src-rn/utils/notificationTasks.ts:35-37, 62-64). If geofencing never ran (no company
location configured), `isAtCompany` stays `false` — see §10.1.

## 3. Triggers — when the check runs

`checkCancelReminder()` is invoked from four places
(v1: src/app/src-rn/utils/cancelReminderCheck.ts:17-21 lists them in a comment; call sites below):

1. **After every successful order fetch** (foreground refresh): `orderStore.fetchOrders()` calls
   it (non-web platforms only) right after storing fresh orders — after `cancelGeofenceNotification`
   (when an order for today exists) and after `checkDailyReminder`. **All three calls run inside
   ONE shared try/catch** (v1: orderStore.ts:73-85): a throw from the geofence cancel or the
   daily-reminder check silently skips the cancel-reminder check for that fetch (the next trigger
   retries); only the combined failure is swallowed ("notification service may not be available")
   (v1: src/app/src-rn/store/orderStore.ts:59-91).
2. **Background order-sync task** (`BACKGROUND_ORDER_SYNC_TASK`, task name string
   `'BACKGROUND_ORDER_SYNC_TASK'`, v1: src/app/src-rn/utils/constants.ts:32): the task loads cached
   orders from local storage (deliberately no network, to avoid concurrent scraping), runs
   `checkDailyReminder()`, then `checkCancelReminder()`; an error from the cancel-reminder check is
   caught and logged as (`order-sync`, `error`, `cancel_reminder_check_error`, message) and is not
   fatal to the task (v1: src/app/src-rn/utils/notificationTasks.ts:82-112). Task registration:
   minimum interval **15 minutes**, skipped when the OS reports background tasks Restricted
   (v1: src/app/src-rn/utils/notificationService.ts:77-87). v1 mechanism: `expo-background-task`
   (BGAppRefreshTask on iOS, WorkManager on Android) — run cadence is OS-controlled, best-effort.
3. **Geofence Enter** (user arrived at office): the handler sets `isAtCompany = true`, loads cached
   orders (the Zustand store is empty on cold start), then calls `checkCancelReminder()` — which,
   because `isAtCompany` is now true, always takes the cancel path, retracting a pending reminder
   (v1: src/app/src-rn/utils/notificationTasks.ts:35-61). Errors are logged as
   (`geofence`, `error`, `enter_notify_error`, message).
4. **Geofence Exit** (user left office): the handler sets `isAtCompany = false`, cancels the
   geofence "no order" notification, loads cached orders, then calls `checkCancelReminder()` —
   scheduling the reminder if an order for today exists and it is before 09:00
   (v1: src/app/src-rn/utils/notificationTasks.ts:62-75). Errors are logged as
   (`geofence`, `error`, `exit_cancel_error`, message).

## 4. Timing rules — `scheduleCancelReminderNotification()`

(v1: src/app/src-rn/utils/notificationService.ts:147-187, constants in
src/app/src-rn/utils/constants.ts:34-42)

Constants:

| Constant | Value | Meaning |
|---|---|---|
| `NOTIFICATION_HOUR` / `NOTIFICATION_MINUTE` | `8` / `45` | Target fire time 08:45 (shared with the geofence "no order" notification) |
| `CANCEL_REMINDER_DEADLINE_HOUR` / `CANCEL_REMINDER_DEADLINE_MINUTE` | `9` / `0` | Deadline 09:00; code comment: "Gourmet's cancellation deadline is 09:00 — past this, cancelling is no longer possible" |
| `CANCEL_REMINDER_NOTIFICATION_ID` | `cancel-order-reminder` | Fixed notification identifier |
| `NOTIFICATION_CHANNEL_ID` | `order-reminders` | Android channel |

Algorithm, with `currentMin` = current Vienna minutes-since-midnight (`viennaMinutes()`,
minute granularity, v1: src/app/src-rn/utils/dateUtils.ts:66-77), `targetMin = 8*60+45 = 525`,
`deadlineMin = 9*60+0 = 540`:

1. If `currentMin >= deadlineMin` (**at or past 09:00**) → return `false`, schedule nothing. Too
   late to cancel the order, so a reminder would be useless.
2. If `currentMin < targetMin` (**before 08:45**) → schedule a **date-triggered** local
   notification at `now + (targetMin − currentMin) minutes` (fire instant inherits the
   seconds-of-minute at scheduling time), identifier `cancel-order-reminder`, Android channel
   `order-reminders`. Return `true`.
3. Otherwise (**08:45 ≤ now < 09:00**) → fire the notification **immediately** (null trigger), same
   identifier and content. Return `true`.

Note the contrast with the daily reminder, which never fires immediately when past its target time:
the cancel reminder *does* have an immediate-fire window (08:45–08:59 Vienna), bounded by the 09:00
deadline. (Compare v1: src/app/src-rn/utils/notificationService.ts:158, 179-185 vs. 213.)

The 09:00 deadline mirrors the app's own cancellation cutoff: today's order cannot be cancelled at
or after 09:00 Vienna time (`isCancellationCutoff`, v1: src/app/src-rn/utils/dateUtils.ts:107-112;
UI behavior in `03-features/orders.md`).

`cancelCancelReminderNotification()` cancels the *pending scheduled* notification by identifier
(v1: src/app/src-rn/utils/notificationService.ts:192-194; v1 mechanism:
`Notifications.cancelScheduledNotificationAsync` — it does **not** retract an already-delivered
notification, see §10.3).

## 5. Dedupe logic

- **At most one pending reminder** at a time: every schedule call reuses the fixed identifier
  `cancel-order-reminder`, which replaces any previously pending notification with that identifier
  (v1 mechanism: expo-notifications/UNUserNotificationCenter identifier semantics; same pattern as
  the daily reminder, v1: src/app/src-rn/utils/notificationService.ts:171, 181).
- **There is no persisted once-per-day dedupe.** Unlike the daily reminder (which has a vestigial
  sent-date key), the cancel reminder stores nothing at all. Dedupe relies solely on (a) identifier
  reuse and (b) the guards: before 08:45 repeated checks just replace the pending schedule; at or
  after 09:00 nothing fires.
- **Immediate-fire window (08:45–08:59):** every check that passes the condition in §2 fires
  immediately. If multiple checks run inside this 15-minute window (e.g. a background-sync run plus
  a foreground order fetch), each fires a notification with the same identifier; app code does
  nothing to prevent this (v1: src/app/src-rn/utils/notificationService.ts:179-185). Whether the OS
  re-alerts or silently replaces on repeated same-identifier delivery is OS behavior, not
  determined by v1 source.
- **Retraction paths** (cancel pending notification): user enters the company geofence; an order
  check finds no order for today (e.g. the user cancelled the order); the company location is
  removed (cancels *all* scheduled notifications, §7).

## 6. Notification content

(v1: src/app/src-rn/utils/notificationService.ts:160-165)

| Field | Exact value |
|---|---|
| Identifier | `cancel-order-reminder` |
| Title | `SnackPilot` |
| Body | `Du hast heute bestellt, bist aber nicht im Büro. Stornieren?` |
| Sound | `default` |
| Data payload | `{ screen: '/(tabs)/orders' }` |
| Android channel | `order-reminders` on the pre-scheduled (date-trigger, before 08:45) branch only (v1: src/app/src-rn/utils/notificationService.ts:173-177); the **immediate-fire branch passes no channelId** (v1: notificationService.ts:180-184), so which channel an immediate delivery uses is expo default behavior — v2 must decide whether to attach the channel in both branches. (Channel: display name `Bestellungs-Erinnerungen`, importance HIGH, vibration pattern `[0, 250, 250, 250]`; v1: notificationService.ts:273-281) |

The body is static — it does not include the order title or count.

**Tap behavior (code wins):** the `data.screen` payload suggests deep-linking to the Orders tab,
but **no code anywhere in v1 registers a notification-response listener or reads `data.screen`**
(verified by search over `src/app/**`) — tapping merely opens the app via OS default behavior.
Same discrepancy as the daily reminder (`03-features/notifications-daily-reminder.md` §6).

Foreground presentation (v1 mechanism, shared across all notifications): banner + list + sound,
no badge, while the app is foregrounded
(v1: src/app/src-rn/utils/notificationService.ts:262-271).

## 7. Settings gating

**There is no dedicated on/off setting, no configurable time, and no persisted state for the
cancel reminder.** It does not read `reminderStorage` at all — the shared-module listing in the
source map exists only because the daily reminder's enabled flag gates one of the *trigger paths*
below. Activation is entirely indirect, via which trigger paths (§3) are live:

1. **Company location set** ("Standort-Benachrichtigungen" section of the Benachrichtigungen
   screen): setting the current position as company location requests location permissions
   (foreground + background/"Always") and notification permission, stores the location, and calls
   `enableNotifications()` → starts geofencing + registers the background-sync task
   (v1: src/app/app/notifications.tsx:145-177,
   src/app/src-rn/utils/notificationService.ts:293-296). This activates trigger paths 2–4 of §3.
   On every app start with a company location present, `enableNotifications()` runs again
   (v1: src/app/app/_layout.tsx:75-78).
2. **Daily reminder enabled**: toggling the daily reminder ON registers the background-sync task
   even without a company location (v1: src/app/app/notifications.tsx:116-133,
   src/app/app/_layout.tsx:80-93), which activates trigger path 2 of §3. The cancel reminder itself
   still does nothing without a saved company location (§2).
3. **Foreground order fetch**: trigger path 1 of §3 runs after every successful fetch, but the
   cancel reminder only schedules once a company location is saved (§2).

Deactivation: removing the company location (`Standort entfernen`) clears the location, resets
`isAtCompany` to `false`, and calls `disableNotifications()` → stops geofencing, unregisters the
background-sync task, and calls `cancelAllScheduledNotificationsAsync()` — cancelling **all**
pending notifications, including a pending cancel reminder
(v1: src/app/app/notifications.tsx:179-182,
src/app/src-rn/utils/notificationService.ts:298-302, 258-260). v2 retracts the pending geofence
and cancel-reminder notifications instead of cancelling every app notification. Trigger path 1
(foreground fetch) still runs after removal, but §2's location gate keeps it from scheduling
anything — see §10.1.

Permissions: the check/schedule path never requests or verifies notification permission;
permission is only requested by the location-setup and daily-reminder-toggle flows (and by the
new-menu feature at app start when authenticated, v1: src/app/app/_layout.tsx:49-59). What happens
if scheduling runs without granted permission is v1-mechanism/OS behavior, not determined by
source.

## 8. Time and date semantics

All times are **Europe/Vienna** wall-clock regardless of device timezone, using the same helpers as
the daily reminder — `viennaMinutes()`, `viennaToday()`, `isSameDay()`
(v1: src/app/src-rn/utils/dateUtils.ts:55-86). See
`03-features/notifications-daily-reminder.md` §7 for the exact definitions.

## 9. Diagnostic log events

`checkCancelReminder()` itself writes **no log entries** (unlike `checkDailyReminder`). The only
cancel-reminder-specific event is the background task's error wrapper:

| Subsystem | Level | Event | Detail |
|---|---|---|---|
| `order-sync` | `error` | `cancel_reminder_check_error` | error message |

(v1: src/app/src-rn/utils/notificationTasks.ts:97-103.) Errors thrown from the check inside the
geofence handlers surface as the location feature's `enter_notify_error` / `exit_cancel_error`
events (v1: src/app/src-rn/utils/notificationTasks.ts:58-61, 71-74; log mechanism in
`03-features/notification-log.md`).

## 10. Behavior notes and edge cases

Test-encoded expectations (v1: src/app/src-rn/__tests__/utils/cancelReminderCheck.test.ts:52-88):
at company + order today → cancel, not schedule; not at company + order only for tomorrow →
cancel, not schedule; not at company + order today → schedule, not cancel.

Behavior an implementer must reproduce (or consciously change with a note):

1. **Does not fire without location setup.** v2 deliberately diverges from v1 here: v1 scheduled
   the cancel reminder even when no company location was configured, because `isAtCompany`
   defaults to `false` (v1: src/app/src-rn/store/orderStore.ts:70-86,
   src/app/src-rn/utils/cancelReminderCheck.ts:22-35). v2 gates `checkCancelReminder()` on a saved
   company location, so an order fetched without one never produces a "not in the office" reminder;
   any pending reminder is retracted instead. This applies to foreground fetches and the
   background-sync task alike (`docs/requirements/03-features/notifications-location.md` §1, §5).
2. **Stale `isAtCompany`.** The flag only changes on geofence events (and location removal) and is
   persisted across restarts. If the OS misses an Exit event, the reminder is suppressed; if it
   misses an Enter event, the reminder fires despite the user being at the office. The 500 m
   geofence radius (`COMPANY_GEOFENCE_RADIUS_M = 500`, v1: src/app/src-rn/utils/constants.ts:33)
   is owned by `03-features/notifications-location.md`.
3. **Delivered notifications are not retracted.** The cancel path removes only the *pending
   scheduled* notification. Arriving at the office at 08:50, after the 08:45 notification already
   fired, does not remove it from the notification tray (v1 mechanism:
   `cancelScheduledNotificationAsync`, v1: src/app/src-rn/utils/notificationService.ts:192-194).
4. **No approval filter** — an unconfirmed order for today still triggers the reminder
   (v1: src/app/src-rn/utils/cancelReminderCheck.ts:26).
5. **Error asymmetry**: cancellation failures are swallowed inside `checkCancelReminder`;
   scheduling failures propagate and are handled per-caller (§3)
   (v1: src/app/src-rn/utils/cancelReminderCheck.ts:29-31, 35).
6. **Cross-feature cancellation hazard** (shared with the daily reminder): removing the company
   location cancels all pending notifications of the app, and unregisters the background-sync task
   until the next app launch (if the daily reminder is enabled) or reminder toggle (§7).
7. The user-facing description of location notifications ("Erinnerung um 8:45 basierend auf deinem
   Standort", and the save confirmation mentioning 8:45) covers both the geofence notification and
   this feature; UI text ownership is `03-features/notifications-location.md` / `04-ui-ux.md`.

## 11. Dropped in v2

- Web/desktop: v1 shipped no-op web stubs (`notificationService.web.ts`,
  `notificationTasks.web.ts`), gated task definitions behind `Platform.OS !== 'web'`
  (v1: src/app/src-rn/utils/notificationTasks.ts:20) and the fetch-time check behind
  `Platform.OS !== 'web'` (v1: src/app/src-rn/store/orderStore.ts:70). v2 is mobile-only;
  implement natively preserving the semantics above.
- The `expo-notifications` / `expo-background-task` / `expo-location` machinery is a v1 mechanism;
  v2 must preserve the observable contract: the four trigger paths in §3, the 08:45 target /
  08:45–09:00 immediate-fire window / 09:00 hard deadline in §4, identifier-replacement dedupe
  (§5), and the exact content in §6.
