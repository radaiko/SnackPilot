# Settings & login screens

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

Covers the Settings tab (root screen), the two credential/login subpages (Kantine =
Gourmet, Automaten = Ventopay), the Appearance subpage, the Notifications subpage
(screen-level inventory only), the auth state machines behind the login flows, and
credential storage/validation UX.

**What this doc does NOT restate** (reference instead):

- HTTP login/logout request specs for Gourmet → `01-gourmet-scraping.md`
- HTTP login/logout request specs for Ventopay → `02-ventopay-scraping.md`
- Theme/accent persistence, color values, alternate app icons → `03-features/themes.md`
- Daily-reminder scheduling internals → `03-features/notifications-daily-reminder.md`
- Location-notification internals → `03-features/notifications-location.md`
- Notification-log storage/format internals → `03-features/notification-log.md`
- Demo-mode APIs and demo data → `03-features/demo-mode.md`
- Analytics (`trackSignal`) transport and semantics → `03-features/analytics.md`
- Secure credential storage mechanism and keychain migration → `05-platform-services.md`
- Dialog component (`alert`/`confirm`), navigation shell, styling system → `04-ui-ux.md`

All user-facing strings below are German and must be reproduced byte-for-byte
(including typographic quotes and `...` sequences).

---

## 1. Navigation structure

The Settings tab is one of the four bottom tabs. Four subpages are pushed onto the
root stack as full-screen cards with the native header hidden
(v1: src/app/app/_layout.tsx:99-102):

| Route | Screen | Registration |
|---|---|---|
| `/kantine-login` | Kantine credentials | `headerShown: false, presentation: 'card'` |
| `/automaten-login` | Automaten credentials | `headerShown: false, presentation: 'card'` |
| `/notifications` | Notifications | `headerShown: false, presentation: 'card'` |
| `/appearance` | Appearance | `headerShown: false, presentation: 'card'` |

Because the native header is hidden, every subpage renders its own back control at the
top: a back-chevron icon (Ionicons `chevron-back`, size 24, tinted with the accent
`primary` color) followed by the text `Einstellungen` (font size 17, accent color);
tapping it performs a navigation "back" (v1: src/app/app/kantine-login.tsx:83-86, and
identically on the other three subpages).

---

## 2. Settings tab (root screen)

(v1: src/app/app/(tabs)/settings.tsx)

A vertically scrolling list. On every screen focus it emits the analytics signal
`screen.viewed` with payload `{ screen: 'settings' }`
(v1: src/app/app/(tabs)/settings.tsx:50-54).

Row inventory, in mobile order (v1: src/app/app/(tabs)/settings.tsx:166-175). Each of
the first four rows is a pressable navigation row with a title, a status hint line
beneath it, and a trailing chevron icon (Ionicons `chevron-forward`, size 20, tertiary
text color). Rows are separated by 1-px dividers.

### 2.1 Kantine-Zugangsdaten row

- Title: `Kantine-Zugangsdaten`
- Hint: if Gourmet auth status is `authenticated`: `Angemeldet als {username}` where
  `{username}` is the logged-in user's display name from the Gourmet user info
  (`userInfo.username`); otherwise `Nicht angemeldet`
  (v1: src/app/app/(tabs)/settings.tsx:60-65)
- Tap → navigate to `/kantine-login`

### 2.2 Automaten-Zugangsdaten row

- Title: `Automaten-Zugangsdaten`
- Hint: if Ventopay auth status is `authenticated`: `Sitzung aktiv`; otherwise
  `Nicht angemeldet` (v1: src/app/app/(tabs)/settings.tsx:77-82). Note: unlike
  Gourmet, no username is shown (Ventopay auth state carries no user info).
- Tap → navigate to `/automaten-login`

### 2.3 Darstellung row

- Title: `Darstellung`
- Hint: the current theme preference mapped through
  `{ system: 'System', light: 'Hell', dark: 'Dunkel' }`, falling back to the raw
  preference string if unmapped (v1: src/app/app/(tabs)/settings.tsx:29-33,94-97)
- Tap → navigate to `/appearance`

### 2.4 Benachrichtigungen row

- Title: `Benachrichtigungen`
- Hint: `Erinnerungen und Standort-Benachrichtigungen` (static)
- Tap → navigate to `/notifications`
- v1 renders this row only when `isNative()` is true (hidden on web/desktop)
  (v1: src/app/app/(tabs)/settings.tsx:104-115). **Dropped in v2:** v2 is
  mobile-only, so the row is always shown.

### 2.5 Updates card — Dropped in v2

v1 shows an `Updates` card only on desktop (Tauri) when a Velopack update is pending:
text `Version {pendingVersion} ist bereit zur Installation.`, a primary button
`Jetzt aktualisieren`, and hint `Das Update wird auch beim nächsten Neustart
automatisch angewendet.` (v1: src/app/app/(tabs)/settings.tsx:117-136). The desktop
target and Velopack updater are dropped in v2; do not port.

### 2.6 Datenschutz link

A centered tertiary-colored text link `Datenschutz` at the bottom of the screen.
Tapping it shows a single-OK alert dialog with title `Datenschutz` and this exact
message (v1: src/app/app/(tabs)/settings.tsx:138-149):

> Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.

### 2.7 Desktop wide layout — Dropped in v2

On desktop widths v1 rearranges the rows into a two-column card grid (max width 900)
(v1: src/app/app/(tabs)/settings.tsx:153-165). Dropped with the desktop target.
Likewise all `isCompactDesktop` metric variants throughout these screens
(`isCompactDesktop` is true only on desktop; v1: src/app/src-rn/utils/platform.ts:23)
— v2 uses only the non-compact (mobile) values.

---

## 3. Auth state machines

Two independent per-service auth stores with identical shapes
(v1: src/app/src-rn/store/authStore.ts — Gourmet;
src/app/src-rn/store/ventopayAuthStore.ts — Ventopay).

### 3.1 State

```
status:   'idle' | 'loading' | 'authenticated' | 'error' | 'no_credentials'
error:    string | null
userInfo: GourmetUserInfo | null    // Gourmet store only
api:      service API instance      // swappable for demo mode
```

(v1: src/app/src-rn/store/authStore.ts:10-24;
src/app/src-rn/store/ventopayAuthStore.ts:11-24)

Initial state: `status: 'idle'`, `error: null`, `userInfo: null`
(verified by v1: src/app/src-rn/__tests__/store/authStore.test.ts:41-48).

`GourmetUserInfo` = `{ username, shopModelId, eaterId, staffGroupId }` (extraction
spec in `01-gourmet-scraping.md`; test evidence
v1: src/app/src-rn/__tests__/store/authStore.test.ts:51-56).

### 3.2 `login(username, password) → bool`

1. Set `status: 'loading'`, `error: null`.
2. **Demo-credentials check** (before any network I/O):
   `username.toLowerCase() === 'demo' && password === 'demo1234!'`
   (v1: src/app/src-rn/utils/constants.ts:23-27 — `DEMO_USERNAME = 'demo'`,
   `DEMO_PASSWORD = 'demo1234!'`; the username comparison is case-insensitive, the
   password comparison is exact). If matched, swap the store's `api` instance for the
   demo API, "log in" against it, set `status: 'authenticated'`, emit
   `auth.loginSuccess`, and return `true` — the real service is never contacted
   (verified by v1: src/app/src-rn/__tests__/store/authStore.test.ts:204-213 and
   ventopayAuthStore.test.ts:146-154). The Gourmet demo path additionally resets the
   menu cache (`items: []`, `lastFetched: null`) so stale real data is not shown
   (v1: src/app/src-rn/store/authStore.ts:38). Demo API behavior →
   `03-features/demo-mode.md`.
3. Otherwise call the real service login (HTTP spec → `01-gourmet-scraping.md` /
   `02-ventopay-scraping.md`).
4. Success: `status: 'authenticated'`, `error: null` (Gourmet also stores the returned
   `userInfo`); emit analytics signal `auth.loginSuccess` with
   `{ service: 'gourmet' }` or `{ service: 'ventopay' }`; return `true`.
5. Failure (thrown error): `status: 'error'`; `error` = the thrown `Error`'s message,
   or the fallback string when the thrown value is not an `Error`: `Login failed`
   (Gourmet, v1: src/app/src-rn/store/authStore.ts:48) / `Ventopay login failed`
   (Ventopay, v1: src/app/src-rn/store/ventopayAuthStore.ts:46); Gourmet also clears
   `userInfo`; emit `auth.loginFailed` with the service payload; return `false`.
   Note the store error strings are English (surfaced verbatim in the failure alert,
   §4.2). The messages thrown by the API layer when the server rejects the
   credentials — i.e. what the user typically sees in that alert — are exactly:
   - Gourmet: `Login failed: invalid credentials or account blocked`
     (v1: src/app/src-rn/api/gourmetApi.ts:95)
   - Ventopay: `Ventopay login failed: invalid credentials or account blocked`
     (v1: src/app/src-rn/api/ventopayApi.ts:57)

### 3.3 `loginWithSaved() → bool`

Read saved credentials (§3.6). If absent → set `status: 'no_credentials'`, return
`false`, **no network request** (verified by
v1: src/app/src-rn/__tests__/store/authStore.test.ts:127-135). Otherwise delegate to
`login(saved.username, saved.password)`.

### 3.4 `logout()`

1. Emit `auth.logout` with the service payload (before the network call).
2. Call the service logout (HTTP spec → 01-/02- docs).
3. **Always** (even if the logout request throws) reset state to `status: 'idle'`,
   `error: null` (Gourmet also `userInfo: null`). A thrown error still propagates to
   the caller after the reset (verified by
   v1: src/app/src-rn/__tests__/store/authStore.test.ts:155-166).

**Logout does NOT delete saved credentials.** `clearCredentials` exists on both
stores but is never invoked from any v1 screen (verified by repo-wide search at
6997c44). Combined with startup auto-login (§3.7), tapping `Abmelden` only ends the
current session; the next app launch silently logs back in with the still-saved
credentials.

### 3.5 Credential storage keys

Stored via the secure-storage service (mechanism, keychain accessibility class, and
migration → `05-platform-services.md`). Exact keys:

| Service | Username key | Password key |
|---|---|---|
| Gourmet | `gourmet_username` | `gourmet_password` |
| Ventopay | `ventopay_username` | `ventopay_password` |

(v1: src/app/src-rn/utils/constants.ts:12-13;
src/app/src-rn/store/ventopayAuthStore.ts:8-9)

### 3.6 `saveCredentials` / `getSavedCredentials` / `clearCredentials`

- `saveCredentials(u, p)`: write both keys.
- `getSavedCredentials()`: read both keys; return `{ username, password }` only when
  **both** are non-empty, otherwise `null`
  (v1: src/app/src-rn/store/authStore.ts:78-83).
- `clearCredentials()`: delete both keys (present in the API surface; unused by UI,
  see §3.4).

### 3.7 Startup auto-login

On app launch (root layout mount), after the keychain-accessibility migration over
the four keys above (native only; details → `05-platform-services.md`), both stores'
`loginWithSaved()` are invoked — Gourmet first, then Ventopay, fire-and-forget without
awaiting either (they run concurrently)
(v1: src/app/app/_layout.tsx:36-47). Users therefore never see a login wall; a user
with no saved credentials simply lands in `no_credentials` state and the Settings
rows read `Nicht angemeldet`.

---

## 4. Kantine login screen (`/kantine-login`)

(v1: src/app/app/kantine-login.tsx)

Layout top-to-bottom: back control (§1), page title `Kantine-Zugangsdaten`, two
labeled inputs, a save button, and — only while `status === 'authenticated'` — a
session section. The screen scrolls, keeps taps on inputs while the keyboard is open
(`keyboardShouldPersistTaps="handled"`), and on iOS pads content above the keyboard
(v1 mechanism: `KeyboardAvoidingView` with `behavior='padding'` on iOS only,
v1: src/app/app/kantine-login.tsx:77).

### 4.1 Fields

| Label | Placeholder | Behavior |
|---|---|---|
| `Benutzername` | `Benutzername eingeben` | no auto-capitalize, no autocorrect |
| `Passwort` | `Passwort eingeben` | obscured (secure entry), no auto-capitalize, no autocorrect |

**Prefill:** on mount, saved credentials (if any) are loaded and populate both
fields — including the password into the (obscured) password field
(v1: src/app/app/kantine-login.tsx:45-53).

### 4.2 Save action

Button label `Speichern`; while the save is in flight the label changes to
`Speichern...` and the button is disabled (v1: src/app/app/kantine-login.tsx:117-125).

Flow (v1: src/app/app/kantine-login.tsx:55-70):

1. **Client-side validation:** if either field is empty → alert
   (`Fehler`, `Bitte Benutzername und Passwort eingeben`); stop, no storage write, no
   network.
2. **Persist first:** `saveCredentials(username, password)` — credentials are written
   to secure storage **before** they are validated against the server. A subsequent
   failed login therefore still leaves the (possibly wrong) credentials saved,
   overwriting any previously working ones. Reproduce this ordering exactly.
3. **Validate by logging in:** `login(username, password)` (§3.2 — includes the demo
   branch, so entering `demo` / `demo1234!` here activates demo mode).
4. On success → alert (`Gespeichert`, `Kantine-Zugangsdaten sicher gespeichert`).
5. On failure → alert (`Login fehlgeschlagen`, `{storeError}`), where `{storeError}`
   is the store's `error` string (§3.2); if that is empty/null the fallback message is
   `Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.`
6. In both outcomes the screen stays put — there is no automatic navigation back to
   the Settings tab; the user leaves via the back control.

All alerts here are single-OK dialogs (dialog component → `04-ui-ux.md`).

### 4.3 Session section (visible only when authenticated)

- Text: `Angemeldet als: {userInfo.username}`
- Danger (red) button `Abmelden` → store `logout()` (§3.4). No confirmation dialog,
  no success alert, inputs keep their values, saved credentials remain stored
  (v1: src/app/app/kantine-login.tsx:72-74,127-136). Any error thrown by the logout
  HTTP call is not caught by the screen.

---

## 5. Automaten login screen (`/automaten-login`)

(v1: src/app/app/automaten-login.tsx)

Identical structure and flow to §4 against the Ventopay store, with these
differences:

- Page title: `Automaten-Zugangsdaten`, plus a subtitle directly beneath it:
  `Für Automaten und Kassenabrechnungen` (v1: src/app/app/automaten-login.tsx:87-88).
- Success alert message: (`Gespeichert`, `Automaten-Zugangsdaten sicher gespeichert`).
- Failure alert identical (`Login fehlgeschlagen`, store error or
  `Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.`).
- Session section text: `Automaten-Sitzung aktiv` (no username; the Ventopay store
  has no user info), followed by the `Abmelden` danger button
  (v1: src/app/app/automaten-login.tsx:127-134).
- Same persist-before-validate ordering, same empty-field validation alert
  (`Fehler`, `Bitte Benutzername und Passwort eingeben`), same prefill-on-mount, same
  demo-credentials branch.

---

## 6. Appearance screen (`/appearance`)

(v1: src/app/app/appearance.tsx)

Back control (§1), page title `Darstellung`, then two cards. Every selection applies
and persists immediately; there is no save button. Persistence keys, color values,
and the coupled alternate-app-icon behavior → `03-features/themes.md`.

### 6.1 Card `Design` — theme preference

Three equal-width segments, exactly (v1: src/app/app/appearance.tsx:18-22):

| value | Label | Icon (Ionicons) |
|---|---|---|
| `system` | `System` | `phone-portrait-outline` |
| `light` | `Hell` | `sunny-outline` |
| `dark` | `Dunkel` | `moon-outline` |

The selected segment is highlighted (accent-tinted surface, accent-colored icon and
label; the others use secondary text color).

### 6.2 Card `Akzentfarbe` — accent color

Five options rendered in definition order with exact ids and labels
(v1: src/app/src-rn/theme/colors.ts:162,177-253):

| id | Label |
|---|---|
| `orange` | `Orange` |
| `emerald` | `Smaragd` |
| `berry` | `Beere` |
| `golden` | `Gold` |
| `ocean` | `Ozean` |

Each option is a filled circle in that accent's **light-mode** `primary` color
(regardless of the currently active color scheme; v1:
src/app/app/appearance.tsx:88-97) with the label underneath. The selected option
shows a white checkmark inside the circle and a 3-px border in the same color; its
label switches to the current accent color.

---

## 7. Notifications screen (`/notifications`)

(v1: src/app/app/notifications.tsx)

Screen-level inventory; the underlying scheduling/geofencing/log-storage logic lives
in the referenced feature docs. Back control (§1), page title `Benachrichtigungen`,
then three sections separated by dividers.

On mount, the screen loads the persisted reminder-enabled flag and reminder time, and
the log state; the log state is additionally re-loaded every time the screen regains
focus (so a recording that expired while the user was away is reflected)
(v1: src/app/app/notifications.tsx:95-114).

### 7.1 Section `Bestell-Erinnerung` (daily order reminder)

Hint: `Tägliche Erinnerung an deine Bestellung`.

**Toggle** labeled `Aktiviert` (v1: src/app/app/notifications.tsx:116-133):

- Turning ON: request notification permission. If denied → alert
  (`Berechtigung fehlt`, `Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.`)
  and the toggle stays off. If granted → persist the currently selected reminder time,
  register the background sync task, then persist enabled = true and flip the toggle.
- Turning OFF: persist enabled = false (no permission interaction).
- On completion of either the ON (permission granted) or OFF path, emit
  `notification.reminderToggled` with
  `{ enabled: '{true|false}', hour: '{hour}', minute: '{minute}' }` (all values
  stringified). **The permission-denied abort emits no signal** — the handler returns
  before the `trackSignal` call (v1: src/app/app/notifications.tsx:118-132).

**Time picker** — visible only while the toggle is on, headed by the label `Uhrzeit`
(v1: src/app/app/notifications.tsx:240): a horizontal chip row of times
from 11:00 to 13:45 in 15-minute steps (hours 11–13 inclusive × minutes 0/15/30/45 =
12 chips), each labeled zero-padded `HH:MM` (v1: src/app/app/notifications.tsx:51-60).
Default selection 11:00 (v1: src/app/app/notifications.tsx:76-77). Tapping a chip
selects it and persists the time immediately
(v1: src/app/app/notifications.tsx:135-139). Scheduling semantics →
`03-features/notifications-daily-reminder.md`.

**Android-only hint banner** beneath the picker (v1:
src/app/app/notifications.tsx:262-270): info icon plus text
`Damit Erinnerungen zuverlässig funktionieren, muss die Hintergrundaktivität für diese App erlaubt sein. `
followed by the accent-colored inline link `App-Einstellungen öffnen`; tapping
anywhere on the banner opens the OS app-settings page for the app.

### 7.2 Section `Standort-Benachrichtigungen` (location notifications)

Hint: `Erinnerung um 8:45 basierend auf deinem Standort`.

**No company location set** — primary button
`Aktuellen Standort als Firmenstandort setzen`; while working, label
`Standort wird ermittelt...` and button disabled. Flow
(v1: src/app/app/notifications.tsx:145-177):

1. Request location permissions (foreground + background). If not fully granted and
   background ("Always") permission is missing → alert with title
   `Standort „Immer" erforderlich` and message
   `Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer" gesetzt werden.\n\nBitte öffne die Einstellungen und wähle unter Standort „Immer" aus.`,
   then open the OS app-settings page; abort.
2. Request notification permission. If denied → alert (`Berechtigung fehlt`, same
   message as §7.1); abort.
3. Read the current GPS position, store it as the company location, enable the
   location-notification machinery, then alert (`Gespeichert`,
   `Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro bist und nicht bestellt hast.`)
   and emit `notification.locationSet` (no payload).
4. Any thrown error in the flow → alert (`Fehler`,
   `Standort konnte nicht ermittelt werden.`).

**Company location set** — status text `Firmenstandort gesetzt` and a danger button
`Standort entfernen` which clears the stored location and disables the notification
machinery (v1: src/app/app/notifications.tsx:179-182). No confirmation dialog.

Geofence/notification internals → `03-features/notifications-location.md`.

### 7.3 Section `Benachrichtigungs-Log` (diagnostic log + mail feedback)

Hint: `Zeichnet 24 Stunden lang Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.`
(Copy note: the hint says 24 hours although a 12-hour option exists below — the code
wins; keep both options.)

Three mutually exclusive states, derived from a persisted "activated until" timestamp
(v1: src/app/app/notifications.tsx:83-84):

- **Inactive** (no timestamp): two side-by-side primary buttons `12 Stunden` and
  `24 Stunden`; tapping activates recording for that many hours and refreshes the
  section (v1: src/app/app/notifications.tsx:340-355).
- **Active** (`now < activatedUntil`): status text
  `Aufzeichnung läuft bis {timestamp}` where `{timestamp}` is the expiry formatted in
  the `de-AT` locale with 2-digit day, month, hour, minute
  (v1: src/app/app/notifications.tsx:313-322); if there are recorded entries, append
  ` ({count} Einträge)`.
- **Expired** (`now >= activatedUntil`): status text
  `Aufzeichnung abgeschlossen ({count} Einträge).` plus:
  - Primary button with a mail icon (Ionicons `mail-outline`) labeled
    `Log per E-Mail senden` → opens the system **mail composer** (v1 mechanism:
    `expo-mail-composer` `composeAsync`) prefilled with
    (v1: src/app/app/notifications.tsx:189-203):
    - recipient: `aiko@spitzbub.app`
    - subject: `SnackPilot Notification Log (bis {expiry})` where `{expiry}` is the
      activated-until timestamp formatted with the default `de-AT` locale string
      conversion
    - body: the formatted log entries (format spec →
      `03-features/notification-log.md`)
    - The user sends the mail themselves from the composer. If the composer cannot be
      opened (throws) → alert (`Fehler`, `E-Mail-App konnte nicht geöffnet werden.`).
      v1 performs no upfront "mail available" check — just attempt and catch.
  - A plain tertiary-colored text button `Log verwerfen` → clears the persisted log
    and resets the section to Inactive (v1: src/app/app/notifications.tsx:205-209).

This mail composer is the only mail/feedback action in the app.

---

## 8. Analytics signals emitted by these screens

Names and payloads exact (transport → `03-features/analytics.md`):

| Signal | Payload | Trigger |
|---|---|---|
| `screen.viewed` | `{ screen: 'settings' }` | Settings tab focused |
| `auth.loginSuccess` | `{ service: 'gourmet' \| 'ventopay' }` | store login success (incl. demo) |
| `auth.loginFailed` | `{ service: 'gourmet' \| 'ventopay' }` | store login failure |
| `auth.logout` | `{ service: 'gourmet' \| 'ventopay' }` | store logout invoked |
| `notification.reminderToggled` | `{ enabled, hour, minute }` (strings) | reminder toggle changed |
| `notification.locationSet` | — | company location saved |

---

## 9. Dropped in v2 (summary)

- Desktop Updates card / Velopack updater (§2.5).
- Desktop two-column wide layout on the Settings tab (§2.7) and all
  `isCompactDesktop` compact metrics on every screen in this doc.
- Web-only conditional hiding of the Benachrichtigungen row (§2.4) — always show on
  mobile.
