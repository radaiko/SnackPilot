# Spec Delta

## Purpose

Defines how SnackPilot decides, schedules, executes in the background and delivers the order-driven reminders (daily order reminder, cancel reminder) and the new-menu notification on iOS and Android. `openspec/specs/` is empty today, so these are initial requirements. They restate behaviour that `docs/requirements/03-features/notifications-*.md`, `orders.md`, `settings.md` and `05-platform-services.md` already specify; each requirement names its source section, and where shipped code violated the docs (reminder default and picker window, channel names, vibration pattern, Doze behaviour) the scenarios pin the documented behaviour.

## ADDED Requirements

### Requirement: REQ-NTF-01 Daily-reminder settings live in the core KV
The core SHALL persist the daily-reminder enabled flag and time in its key-value store under the v1 keys and formats, and expose accessors so a headless caller needs no shell state. Source: `notifications-daily-reminder.md` §9, `05-platform-services.md` §2.

#### Scenario: Defaults and v1-exact format
- **WHEN** a fresh store is read, then time 11:30 and enabled true are written
- **THEN** the first read is disabled with no hour or minute
- **AND** the stored values are `daily_reminder_time` = `{"hour":11,"minute":30}` and `daily_reminder_enabled` = `true`

#### Scenario: Garbage time is ignored
- **WHEN** the stored time is `not json`, `null`, `{"hour":"x","minute":0}` or `[1,2]`
- **THEN** the settings report no hour and no minute

#### Scenario: Enabled without a time
- **WHEN** the enabled flag is true and no valid time is stored
- **THEN** the settings report enabled with no hour and no minute, so the daily-reminder check's `no_time_configured` guard applies

### Requirement: REQ-NTF-02 Post-fetch notification checks return commands
After a successful orders fetch the core SHALL run the geofence-notice cancel, the daily-reminder check and the cancel-reminder check and return their commands to the caller; a failed fetch SHALL return none. Source: `orders.md` §8, `notifications-daily-reminder.md` §2, `notifications-cancel-reminder.md` §3.

#### Scenario: Reminders scheduled after a fetch
- **WHEN** the clock is 08:00 Vienna, the reminder is enabled for 11:00 and the fetched orders include one for today
- **THEN** the commands contain a `ScheduleAt` for `daily-order-reminder` and a `ScheduleAt` for `cancel-order-reminder`

#### Scenario: Geofence notice cancelled
- **WHEN** the fetched orders include an order for today
- **THEN** the commands contain `CancelPending` for `geofence-no-order-reminder`, ahead of the two reminder commands

#### Scenario: No order today
- **WHEN** the fetched orders contain none for today
- **THEN** the commands contain no `CancelPending` for `geofence-no-order-reminder`

#### Scenario: Failed fetch
- **WHEN** the orders fetch fails
- **THEN** the returned commands are empty, the orders error is set and the previous orders are kept

#### Scenario: Checks never affect orders
- **WHEN** a notification check yields nothing or fails
- **THEN** the orders and the orders error are unchanged

### Requirement: REQ-NTF-03 One headless order-sync check
The core SHALL expose one headless order-sync check that reads the cached orders without any network request and returns the daily-reminder and cancel-reminder commands. The FFI entry SHALL delegate to the orders store's implementation. Source: `notifications-daily-reminder.md` §2, `notifications-cancel-reminder.md` §3, `05-platform-services.md` §4.2.

#### Scenario: Empty cache retracts the cancel reminder
- **WHEN** the cache holds no orders, the user is not at the company and the check runs
- **THEN** the only command is `CancelPending` for `cancel-order-reminder`

#### Scenario: Geofence cancel excluded
- **WHEN** the cache holds an order for today and the check runs
- **THEN** the commands contain no `CancelPending` for `geofence-no-order-reminder`

#### Scenario: Same reminders as the post-fetch checks
- **WHEN** orders are fetched and the headless check then runs from the cache
- **THEN** it returns the same daily-reminder `ScheduleAt` and issues no request

### Requirement: REQ-NTF-04 Daily-reminder window and defaults on the shells
Both shells SHALL default the daily reminder to off at 11:00, hydrate it from the core KV and clamp the time to the window 11:00-13:45 before storing it. Source: `notifications-daily-reminder.md` §9, `settings.md` §7.1; the shipped default of 08:00 made the core refuse to fire.

#### Scenario: Default on first launch
- **WHEN** the app starts with an empty store
- **THEN** the reminder is off and the time shown is 11:00

#### Scenario: Clamp to the nearest bound
- **WHEN** a picked time is 08:30, 13:50 or 14:30
- **THEN** the stored time is 11:00, 13:45 and 13:45 respectively
- **AND** a picked 12:20 stays 12:20

#### Scenario: Write order
- **WHEN** the reminder is switched on at 12:00
- **THEN** the time is written to the core before the enabled flag

#### Scenario: Shared source of truth
- **WHEN** the reminder is set in the app and the background worker later reads the settings
- **THEN** the worker sees the same enabled flag and time

### Requirement: REQ-NTF-05 Enabling the reminder requires notification permission
Turning the daily reminder on SHALL first request notification permission and, when denied, show the `Berechtigung fehlt` alert and change nothing. Source: `settings.md` §7.1, `notifications-daily-reminder.md` §9 step 1.

#### Scenario: Permission denied
- **WHEN** the user turns the reminder on and the system permission is denied
- **THEN** an alert titled `Berechtigung fehlt` with the message `Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.` is shown
- **AND** the switch stays off and the core KV keeps its previous values

#### Scenario: Permission granted
- **WHEN** the user turns the reminder on and permission is granted or already granted
- **THEN** the time is persisted, then the enabled flag, and the reminder is applied

#### Scenario: Turning off
- **WHEN** the user turns the reminder off
- **THEN** no permission request is made and the enabled flag is false

### Requirement: REQ-NTF-06 iOS background refresh task
The iOS app SHALL register one `BGAppRefreshTask` with identifier `dev.radaiko.gourmetclient.menu-refresh` before launch finishes, keep one request pending, and run its work without blocking the main queue. Source: `05-platform-services.md` §4.2, §4.3, §4.5, `notifications-new-menu.md` §3.1.

#### Scenario: Registered and requeued
- **WHEN** the app launches and becomes active on a device
- **THEN** the launch handler is registered and one refresh request is pending with an earliest start 15 minutes ahead
- **AND** a failed submission appears in the unified log

#### Scenario: Requeue at run start
- **WHEN** the task handler is invoked
- **THEN** the next request is submitted before the work starts

#### Scenario: Expiry reports failure
- **WHEN** iOS expires the task before the run finishes
- **THEN** the work is cancelled and the task completes exactly once with success false

#### Scenario: Completion reports success
- **WHEN** the run finishes without being cancelled
- **THEN** the task completes exactly once with success true

#### Scenario: Identifier matches configuration
- **WHEN** the packaged Info.plist is read
- **THEN** its permitted identifiers equal the identifier in code

### Requirement: REQ-NTF-07 Packaged iOS Info.plist carries the background configuration
The iOS app SHALL be built with an `Info.plist` generated by xcodegen from the `info:` block of `project.yml`, because `INFOPLIST_KEY_*` settings cannot carry the background keys. Source: `05-platform-services.md` §4.5, §6, §7.2.

#### Scenario: Background keys present
- **WHEN** a test inside the host app reads the main bundle's info dictionary
- **THEN** `UIBackgroundModes` equals `["fetch"]`
- **AND** `BGTaskSchedulerPermittedIdentifiers` equals `["dev.radaiko.gourmetclient.menu-refresh"]`

#### Scenario: Version keys resolve
- **WHEN** the app is built
- **THEN** `CFBundleShortVersionString` equals `MARKETING_VERSION` and `CFBundleVersion` equals `CURRENT_PROJECT_VERSION` from `project.yml`

#### Scenario: Nothing else regressed
- **WHEN** the built app's Info.plist is inspected
- **THEN** the bundle id is `dev.radaiko.gourmetclient`, the display name is `SnackPilot` and both location purpose strings are unchanged
- **AND** `ITSAppUsesNonExemptEncryption` is false and the scene manifest and launch screen are present

#### Scenario: Generated file is ignored
- **WHEN** the project is generated
- **THEN** `src/ios/Info.plist` exists and is git-ignored

### Requirement: REQ-NTF-08 The new-menu notification and post-fetch commands are delivered
Both shells SHALL deliver the `FireNow` command returned by the menu check and the commands returned by an orders fetch, in the foreground and the background, instead of discarding them. Source: `notifications-new-menu.md` §3.3, §3.4, `orders.md` §8.

#### Scenario: New menu detected
- **WHEN** the menu check returns a `FireNow` command
- **THEN** the shell delivers it

#### Scenario: Orders fetch commands
- **WHEN** an orders fetch returns commands
- **THEN** the shell executes each one in order

#### Scenario: Demo credentials stay local
- **WHEN** the menu check runs with the demo credentials
- **THEN** no host is contacted

### Requirement: REQ-NTF-09 iOS FireNow shows immediately
The iOS notification service SHALL present a `FireNow` command immediately with no trigger and schedule a `ScheduleAt` command with a time-interval trigger. Source: `notifications-new-menu.md` §3.4, `notifications-cancel-reminder.md` §4, `05-platform-services.md` §5.2.

#### Scenario: Immediate request
- **WHEN** a `FireNow` command is executed
- **THEN** the notification request has no trigger

#### Scenario: Scheduled request
- **WHEN** a `ScheduleAt` command with a time 600 seconds ahead is executed
- **THEN** the request has a non-repeating time-interval trigger of about 600 seconds

#### Scenario: Past time keeps a floor
- **WHEN** a `ScheduleAt` command with a time in the past is executed
- **THEN** the trigger interval is one second

#### Scenario: Cancel
- **WHEN** a `CancelPending` command is executed for `daily-order-reminder`
- **THEN** both the pending and the delivered notification with that id are removed

#### Scenario: Foreground presentation
- **WHEN** a notification arrives while the app is open
- **THEN** it is shown as banner, list and sound with no badge

### Requirement: REQ-NTF-10 Android periodic background sync
The Android app SHALL run one unique 15-minute periodic WorkManager job that builds its own core, runs the menu check and the headless order-sync check and delivers the returned commands. Source: `05-platform-services.md` §4.2, §4.3, §4.5, §7.3, `notifications-new-menu.md` §3.1.

#### Scenario: Scheduled once
- **WHEN** the app is launched twice
- **THEN** JobScheduler holds a single job for the unique work `background-sync`
- **AND** its interval is 15 minutes with an initial delay and the keep policy

#### Scenario: Re-armed after boot
- **WHEN** the boot broadcast is received by the non-exported receiver holding `RECEIVE_BOOT_COMPLETED`
- **THEN** the unique work is enqueued again with the keep policy and no duplicate appears

#### Scenario: Headless run delivers commands
- **WHEN** the job runs in a process the UI never started
- **THEN** it constructs the core, runs both checks and posts the returned notifications

#### Scenario: Initialisation failure retries
- **WHEN** the core cannot be built
- **THEN** the job returns retry

### Requirement: REQ-NTF-11 Android delivery is gated on POST_NOTIFICATIONS
On API 33 and later the Android notification service SHALL check `POST_NOTIFICATIONS` before posting and post nothing when it is denied. Source: `05-platform-services.md` §5.1.

#### Scenario: Denied in the background
- **WHEN** the worker or an alarm tries to post and the permission is denied
- **THEN** nothing is posted and nothing crashes

#### Scenario: Denied at launch
- **WHEN** the user denies the launch prompt
- **THEN** the `Berechtigung fehlt` dialog is shown

#### Scenario: Already granted
- **WHEN** the app starts with the permission granted
- **THEN** no prompt is shown

### Requirement: REQ-NTF-12 Android channels and Doze-safe alarms
The Android app SHALL create the two notification channels with the documented ids, names, importance and vibration, and register `ScheduleAt` commands as alarms that fire while the device is idle. Source: `05-platform-services.md` §5.3, `notifications-daily-reminder.md` §4.

#### Scenario: Channel definitions
- **WHEN** the notification service is created
- **THEN** `order-reminders` is named `Bestellungs-Erinnerungen` with importance HIGH and vibration `[0, 250, 250, 250]`
- **AND** `menu-updates` is named `Neue Menüs` with importance DEFAULT

#### Scenario: Exact alarm permitted
- **WHEN** the device is below API 31 or the OS allows exact alarms
- **THEN** the alarm is registered exact-while-idle

#### Scenario: Exact alarm not permitted
- **WHEN** the OS reports that exact alarms cannot be scheduled
- **THEN** the alarm is registered with the allow-while-idle inexact call and may fire minutes late

#### Scenario: Schedule then cancel
- **WHEN** the core returns `ScheduleAt` for an id and later `CancelPending` for it
- **THEN** no alarm remains pending for that id

### Requirement: REQ-NTF-13 Background work adds no extra scraping
Background execution SHALL perform at most one Gourmet menu scrape at a time and SHALL NOT log in or fetch billing as part of the background body. Source: `05-platform-services.md` §4.2, `notifications-new-menu.md` §3.3, `AGENTS.md` threat model.

#### Scenario: Overlapping runs
- **WHEN** a second menu check starts while one is running, from the task, the manual trigger or the foreground
- **THEN** the second returns without any request

#### Scenario: Background body is headless on iOS
- **WHEN** the iOS background body runs with the demo credentials
- **THEN** no login, billing fetch, analytics launch event or permission prompt occurs
- **AND** the only delivered command is the order-sync result

#### Scenario: Order sync reads only the cache
- **WHEN** the order-sync check runs
- **THEN** it issues no network request

### Requirement: REQ-NTF-14 Manual background-sync trigger
Einstellungen → Diagnose on both platforms SHALL offer a control that runs the same body as the background task on demand. The labels are `Hintergrund-Sync ausführen` on iOS and `Hintergrund-Sync` on Android.

#### Scenario: Run on demand
- **WHEN** the user taps the control with the diagnostic log active and the demo credentials saved
- **THEN** the log shows the menu-check entry `demo_credentials_skip`
- **AND** any command returned by the order-sync check is delivered

#### Scenario: Same body
- **WHEN** the control and the background task run
- **THEN** both call the same shared function
