# Spec Delta

## Purpose

Defines what every SnackPilot screen and shared component does in the gpui-kit UI. It reproduces `docs/requirements/04-ui-ux.md` so the rewrite changes the toolkit and not the behaviour.

## ADDED Requirements

### Requirement: REQ-SCR-01 Menus tab
The Menüs tab SHALL behave as `docs/requirements/04-ui-ux.md` §3.1, §4.4 and §4.5 specify: day navigator, menu cards grouped by category, pending order and cancel toggles, the floating submit button with its three label forms, the order-progress banner, and the empty and error states. Ordering past the 09:00 Europe/Vienna cutoff SHALL be blocked using the core's decision.

#### Scenario: Toggle and submit
- **WHEN** the user toggles a menu on a day before the cutoff and taps "Bestellen (1)"
- **THEN** the progress banner shows the phases "Wird in den Warenkorb gelegt..." and "Bestellung wird bestätigt..." and the menu shows as ordered afterwards

#### Scenario: Day navigation
- **WHEN** the user taps the forward arrow or swipes left on the content area
- **THEN** the next available day is shown, and at the last day the arrow is inactive

#### Scenario: Cutoff day
- **WHEN** the selected day is past the cutoff
- **THEN** its menu cards cannot be toggled

### Requirement: REQ-SCR-02 Orders tab
The Bestellungen tab SHALL behave as `04-ui-ux.md` §3.2 and §4.6 specify: Kommende and Vergangene segments with counts, the unconfirmed-orders banner with "Bestätigen", the cancel confirmation dialog, multi-select batch cancellation, and one cancellation in flight at a time. Past-cutoff orders SHALL have their cancel control disabled.

#### Scenario: Cancel one order
- **WHEN** the user taps the cancel control on an order and confirms "Stornieren"
- **THEN** that order's card dims, other cancel controls are hidden, and the order disappears after the core refreshes

#### Scenario: Batch cancel
- **WHEN** the user enters selection mode, selects three orders and confirms
- **THEN** the core receives one batch cancel for all three ids

#### Scenario: Confirm banner
- **WHEN** two orders are unconfirmed
- **THEN** the banner reads "2 unbestätigte Bestellungen" and "Bestätigen" confirms them

### Requirement: REQ-SCR-03 Billing tab
The Abrechnung tab SHALL behave as `04-ui-ux.md` §3.3 and §4.7 specify: three month tabs, source chips Alle, Kantine and Automaten, a combined list sorted newest first, the Gesamt, Belege and Zuschuss summary, `de-AT` EUR formatting, inline errors, and pull-to-refresh that forces a fetch.

#### Scenario: Source filter totals
- **WHEN** the user selects Kantine
- **THEN** only Gourmet bills are listed and the Automaten share of the totals is zero

#### Scenario: Month switch
- **WHEN** the user taps another month and its data is not cached
- **THEN** a spinner shows until the core delivers it, then the list appears

#### Scenario: Empty month
- **WHEN** a month has no entries
- **THEN** "Keine Abrechnungsdaten für diesen Monat" is shown

### Requirement: REQ-SCR-04 Settings tab and dialogs
The Einstellungen tab SHALL show the rows, hints and Datenschutz link of `04-ui-ux.md` §3.4, and a dialog component SHALL provide the alert and confirm dialogs of §4.1 for every screen. Row hints SHALL follow live session and theme state.

#### Scenario: Row hints
- **WHEN** the Kantine session is active for user "demo"
- **THEN** the Kantine-Zugangsdaten hint reads "Angemeldet als demo"

#### Scenario: Datenschutz
- **WHEN** the user taps Datenschutz
- **THEN** a dialog titled "Datenschutz" shows the exact message of §3.4

### Requirement: REQ-SCR-05 Kantine login screen
The Kantine-Zugangsdaten screen SHALL behave as `04-ui-ux.md` §3.5: prefilled saved credentials, an empty-field alert, save before login, the "Speichern..." busy state, the success and failure alerts, and an Abmelden button that ends the session but keeps saved credentials.

#### Scenario: Empty field
- **WHEN** the user taps Speichern with an empty password
- **THEN** the alert "Fehler" / "Bitte Benutzername und Passwort eingeben" appears and nothing is saved

#### Scenario: Failed login still saves
- **WHEN** the user submits wrong credentials
- **THEN** the credentials are saved and the alert "Login fehlgeschlagen" shows the mapped German message

### Requirement: REQ-SCR-06 Automaten login screen
The Automaten-Zugangsdaten screen SHALL behave as `04-ui-ux.md` §3.6 with its own title, subtitle and messages, and a successful login SHALL load Automaten transactions into Abrechnung without a restart.

#### Scenario: Session text
- **WHEN** the Ventopay session is active
- **THEN** the screen shows "Automaten-Sitzung aktiv" and no username

#### Scenario: Logout keeps credentials
- **WHEN** the user taps Abmelden
- **THEN** the session ends and the saved Automaten credentials remain

### Requirement: REQ-SCR-07 Appearance screen
The Darstellung screen SHALL behave as `04-ui-ux.md` §3.7: the Design options System, Hell and Dunkel and the five accent swatches filled with the light-mode colour, each applying immediately and persisting.

#### Scenario: Swatch selected state
- **WHEN** the accent Gold is selected in dark mode
- **THEN** the swatch stays the light-mode Gold fill with a thick ring and a checkmark

### Requirement: REQ-SCR-08 Notifications screen
The Benachrichtigungen screen SHALL behave as `04-ui-ux.md` §3.8: the Bestell-Erinnerung switch and the twelve time chips from 11:00 to 13:45, the company-location flow and its alerts, and the 12 or 24 hour notification log with its inactive, active and expired states and the mail action.

#### Scenario: Reminder time chips
- **WHEN** the reminder is switched on
- **THEN** twelve chips from 11:00 to 13:45 in 15-minute steps appear and the tapped chip is persisted at once

#### Scenario: Company location saved
- **WHEN** location and notification permissions are granted and a fix is obtained
- **THEN** the location is stored through the core, the geofence starts, and the alert "Gespeichert" appears

#### Scenario: Expired log
- **WHEN** the log recording has expired with entries
- **THEN** "Log per E-Mail senden" and "Log verwerfen" are offered

### Requirement: REQ-SCR-09 App bootstrap and background refresh
At launch the app SHALL show cached menus, orders and billing before any network call, auto-login saved credentials for both services (Ventopay first, without its own billing fetch), fetch the operator broadcast banner, and restore the geofence. Returning to the foreground SHALL refresh orders, billing and the broadcast without duplicate concurrent scrapes.

#### Scenario: Cache first
- **WHEN** the app starts with cached menus and no network
- **THEN** the cached menus are visible immediately

#### Scenario: No duplicate billing scrape
- **WHEN** both services have saved credentials at launch
- **THEN** billing is fetched once for both sources

#### Scenario: Broadcast banner
- **WHEN** the core returns a non-empty broadcast
- **THEN** a dismissible banner shows and reappears on the next foreground return
