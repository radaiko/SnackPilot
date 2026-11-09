# platform-bridge Specification

## Purpose
Defines the contract between the Rust UI and the operating-system services that stay native: secure credentials, notifications, location and geofence, analytics and app-icon switching.

## Requirements

### Requirement: REQ-PLT-01 Credentials in the platform secure store
The app SHALL read and write the four credentials `gourmet_username`, `gourmet_password`, `ventopay_username` and `ventopay_password` through the native secure store at the exact coordinates in `docs/requirements/05-platform-services.md` §1, so an in-place update keeps working credentials. Credentials SHALL be saved before a login attempt.

#### Scenario: iOS coordinates
- **WHEN** the app saves a Gourmet login on iOS
- **THEN** a generic-password item with service `app:no-auth`, the key as account and AfterFirstUnlock accessibility exists

#### Scenario: Android coordinates
- **WHEN** the app saves a Gourmet login on Android
- **THEN** the `SecureStore` preferences entry `key_v1-gourmet_username` holds the AES-GCM JSON of §1.5

#### Scenario: Save before validate
- **WHEN** the user submits wrong credentials
- **THEN** they are stored before the login fails

### Requirement: REQ-PLT-02 Notification delivery
The bridge SHALL execute the core's `FireNow`, `ScheduleAt` and `CancelPending` commands one to one and re-issuing an id SHALL replace the pending notification. On Android the channels `order-reminders` and `menu-updates` SHALL exist with the ids of `05-platform-services.md` §5.3. Notifications SHALL show while the app is in the foreground.

#### Scenario: Schedule then cancel
- **WHEN** the core returns `ScheduleAt` for id `daily-order-reminder` and later `CancelPending` for the same id
- **THEN** the OS has no pending notification with that id

#### Scenario: Foreground presentation
- **WHEN** a notification fires while the app is open
- **THEN** a banner is still shown

### Requirement: REQ-PLT-03 Company geofence
The bridge SHALL monitor one 500 m region with identifier `company` once a company location is saved, start it idempotently, and deliver Enter and Exit events to the model even when the OS relaunches the app in the background. The "Always" location permission flow and its German alerts SHALL match `04-ui-ux.md` §3.8.

#### Scenario: Idempotent start
- **WHEN** the app starts twice with a saved company location
- **THEN** monitoring is registered once and no extra Enter event fires

#### Scenario: Background relaunch
- **WHEN** the OS relaunches the app for a region Exit
- **THEN** the model records is-at-company false and runs the core's decision functions

#### Scenario: Permission refused
- **WHEN** only When-In-Use location is granted
- **THEN** the alert "Standort „Immer" erforderlich" is shown and nothing is saved

### Requirement: REQ-PLT-04 Analytics forwarding
The bridge SHALL forward every event the core emits and the per-launch event `app.launched` to the self-hosted Aptabase endpoint without user ids, device tokens or IPs. A send failure SHALL NOT block or crash the app.

#### Scenario: Core event reaches the sink
- **WHEN** the core emits `order.submitted`
- **THEN** the platform analytics shim receives that event name and its properties

#### Scenario: Offline analytics
- **WHEN** the analytics endpoint is unreachable
- **THEN** the UI keeps working and no error is shown

### Requirement: REQ-PLT-05 Alternate app icon
Changing the accent colour SHALL switch the app icon to the matching alternate (iOS alternate icon, Android launcher alias) as in the themes spec, and only on explicit user change, not at startup.

#### Scenario: Accent switch
- **WHEN** the user picks Smaragd
- **THEN** the platform icon switches to the emerald variant

### Requirement: REQ-PLT-06 Bridge failures are contained
A failing, denied or missing platform call SHALL surface as a typed result and never panic or abort the process. A denied permission SHALL show the documented German alert and leave the related setting unchanged.

#### Scenario: Notification permission denied
- **WHEN** the user turns on the daily reminder and denies the permission
- **THEN** the alert "Berechtigung fehlt" appears and the switch stays off

#### Scenario: Missing bridge function
- **WHEN** the platform returns an error for a call
- **THEN** the model returns a typed error and the UI stays responsive

### Requirement: REQ-PLT-07 Desktop fallbacks
On desktop, credentials SHALL be stored in the OS keychain, analytics MAY be a no-op, and notifications, geofence and alarms SHALL be unavailable with the Benachrichtigungen row hidden.

#### Scenario: Desktop credentials
- **WHEN** a desktop user saves a Kantine login
- **THEN** it is stored in the OS keychain and read back on the next start

#### Scenario: Notifications row on desktop
- **WHEN** Einstellungen is shown on desktop
- **THEN** the Benachrichtigungen row is absent
