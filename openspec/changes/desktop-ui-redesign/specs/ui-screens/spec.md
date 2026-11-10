# UI screens delta

## MODIFIED Requirements

### Requirement: REQ-SCR-01 Menus tab
The Menüs tab SHALL behave as `docs/requirements/04-ui-ux.md` §3.1, §4.4 and §4.5 specify on mobile/narrow desktop: day navigator, grouped menu cards, pending order/cancel toggles, floating submit action, progress, empty/error states and core cutoff decision. Wide desktop SHALL use the approved grouped master–detail presentation and explicit toggle/submit actions without changing pending keys, eligibility or submit semantics.

#### Scenario: Toggle and submit
- **WHEN** the user explicitly toggles a menu on a day before the cutoff and activates "Bestellen (1)"
- **THEN** the progress banner shows "Wird in den Warenkorb gelegt..." and "Bestellung wird bestätigt..." and the menu shows as ordered afterwards

#### Scenario: Day navigation
- **WHEN** the user activates the forward day arrow, or swipes left on mobile/narrow content
- **THEN** the next available day is shown, and at the last day the arrow is inactive
- **AND** wide desktop uses the day controls/focused keyboard map instead of drag-to-change-day

#### Scenario: Cutoff day
- **WHEN** the selected day is past the cutoff
- **THEN** new-order toggles are blocked by the existing core decision, ordered-item toggle eligibility remains incumbent, and desktop record preview remains available without ordering

#### Scenario: Preview versus pending change
- **WHEN** a wide menu row is clicked or arrowed to
- **THEN** detail immediately shows its day/category, complete dish, optional allergens and original price string without pending-count change, server call or push
- **AND** the row shows the separate preview rule/tint; explicit detail actions are `Zur Bestellung hinzufügen`, `Auswahl aufheben`, `Stornierung vormerken` or `Stornierung aufheben` according to current pending/ordered state
- **AND** statuses distinguish `Vorgemerkt`, `Bestellt`, `Wird storniert`, `Ausverkauft` and `Geschlossen`; unavailable/cutoff new items have no enabled add action

#### Scenario: Wide menu hierarchy and states
- **WHEN** wide Menüs displays populated, empty, unauthenticated, loading, refresh, error or submit-progress data
- **THEN** the header/day toolbar/banners span both panes, master preserves category order and SUPPE & SALAT heading suppression, and detail starts with `Menü auswählen` until selected
- **AND** existing empty/auth/progress strings and sanitized errors/retry are retained, initial spinner only replaces absent data, cached rows remain during refresh, and long dishes/allergens fit the 263 px compact detail
- **AND** a nonbusy pending strip spans both panes with the existing three count-label forms and submits all pending days once; no second cart or confirmation flow is added

### Requirement: REQ-SCR-02 Orders tab
Bestellungen SHALL retain the Kommende/Vergangene counts, all-unconfirmed confirmation banner, single-cancel dialog, batch-cancel flow, single-flight guard and cutoff decisions of `04-ui-ux.md` §3.2/§4.6. Mobile/narrow desktop SHALL retain cards. Wide desktop SHALL use the approved table/detail presentation, with preview distinct from batch checks and cancellation only through explicit actions.

#### Scenario: Cancel one order
- **WHEN** the user activates an eligible order's explicit cancel control and confirms "Stornieren"
- **THEN** the affected mobile/narrow card dims and other mobile/narrow cancel controls are hidden; desktop shows `Wird storniert` on the affected row/detail and disables cancellation entry points
- **AND** the order disappears after core refresh, with no eligibility or request-shape change

#### Scenario: Batch cancel
- **WHEN** the user enters selection mode, selects three eligible orders and invokes the existing batch flow
- **THEN** the core receives one batch cancel for all three ids, with the same incumbent immediate-versus-dialog semantics

#### Scenario: Confirm banner
- **WHEN** two orders are unconfirmed
- **THEN** the banner reads "2 unbestätigte Bestellungen" and "Bestätigen" confirms all unconfirmed orders, not just a previewed row

#### Scenario: Desktop order table and detail
- **WHEN** upcoming or past orders render at wide desktop width
- **THEN** compact table columns are `Datum` (80 px) and `Bestellung` with category/status; expanded columns use date 104 px, flex Bestellung with excerpt, status 104 px
- **AND** table dates use de-AT `dd.MM.yyyy`, status words remain visible, and detail shows the full existing description fallback and approved state without inventing an amount or sorting controls
- **AND** initial detail says `Bestellung auswählen`, empty/auth/error/refresh states retain existing wording, and cached records remain during refresh

#### Scenario: Explicit cancellation and cutoff
- **WHEN** a wide row is previewed or its detail's cancel action is evaluated
- **THEN** preview never cancels or approves; eligible upcoming detail offers `Stornieren` opening the existing `Bestellung stornieren` / `"{order.title}" stornieren?` dialog with `Behalten` / `Stornieren`
- **AND** past detail has no cancel action; cutoff upcoming detail disables it with `Stornierung nicht mehr möglich.`, reevaluated on the existing 30-second cadence

#### Scenario: Batch checks are not previews
- **WHEN** `Auswählen` is entered on upcoming desktop orders
- **THEN** a 44 px checkbox hit column is added, row body previews, checkbox/row-Space toggles only its batch ID once, and selected preview/check remain visually distinct
- **AND** `Stornieren ({n})` disables for zero/busy, `Fertig` exits, Esc exits batch mode before preview, and switching to past exits via the model without cancellation
- **AND** segment and checked IDs survive resize and the device-local-midnight upcoming/past split is unchanged

### Requirement: REQ-SCR-03 Billing tab
Abrechnung SHALL retain the three month options, Alle/Kantine/Automaten filters, newest-first merge, Gesamt/Belege/conditional Zuschuss, de-AT EUR formatting, inline errors and forced-refresh semantics of `04-ui-ux.md` §3.3/§4.7. Mobile/narrow desktop SHALL retain complete bill cards and pull-to-refresh; wide desktop SHALL use the approved receipt table/detail and refresh control/shortcut.

#### Scenario: Source filter totals
- **WHEN** the user selects Kantine
- **THEN** only Gourmet bills are listed and the Automaten share of totals is zero

#### Scenario: Month switch
- **WHEN** the user activates another month and its data is not cached
- **THEN** a spinner shows until core data arrives, then the list appears

#### Scenario: Empty month
- **WHEN** a month has no entries
- **THEN** "Keine Abrechnungsdaten für diesen Monat" is shown

#### Scenario: Desktop billing workspace
- **WHEN** billing renders at wide desktop width
- **THEN** month/source toolbars and one compact wrapping reconciliation line span the split; Gesamt/Belege and Zuschuss >0 only for a non-Automaten filter use current totals, never hero cards
- **AND** compact columns are `Datum / Quelle` and trailing `Betrag` (80 px); expanded columns are `Datum` (112 px, date/time), flex `Quelle`, `Betrag` (96 px); amounts use existing EUR formatting/tabular alignment without horizontal scrolling
- **AND** detail initially says `Beleg auswählen`, shows `Beleg`, long date/time, source, amount and full Gourmet quantity/description/line-total rows or only non-empty Ventopay restaurant description, never invented receipt fields

#### Scenario: Source independence and refresh
- **WHEN** only one billing service is signed in, data refreshes, or context is changed
- **THEN** the signed-in source remains usable, auth gate appears only if neither is signed in, cache stays readable during refresh, and errors remain inline
- **AND** month/source changes clear hidden preview and reset master scroll, while arrows preview in existing merged order without fetching
- **AND** identical date/amount receipts keep distinct stable IDs, negative/zero/large amounts and long line descriptions fit compact detail without truncating essential data

### Requirement: REQ-SCR-04 Settings tab and dialogs
Einstellungen SHALL show the rows/hints and Datenschutz link of `04-ui-ux.md` §3.4, capability-gated notifications, and shared themed alert/confirm dialogs of §4.1. Hints SHALL follow live session/theme state. Wide desktop SHALL select the same settings forms beside its master without push/pop; narrow settings SHALL retain back navigation. Desktop dialogs and privacy amendments SHALL follow the approved brief.

#### Scenario: Row hints
- **WHEN** the Kantine session is active for user "demo"
- **THEN** the Kantine-Zugangsdaten hint reads "Angemeldet als demo"

#### Scenario: Datenschutz
- **WHEN** the user activates Datenschutz
- **THEN** a dialog titled "Datenschutz" shows the exact incumbent paragraph of §3.4
- **AND** desktop alone appends its truthful no-statistics paragraph, while mobile copy is unchanged

#### Scenario: Desktop settings master
- **WHEN** wide Einstellungen is entered and a settings row is selected
- **THEN** the initial prompt `Einstellung auswählen` is replaced by its corresponding existing detail, with no back row or duplicate push
- **AND** order is Kantine-Zugangsdaten, Automaten-Zugangsdaten, Darstellung and capability-gated Benachrichtigungen; long titles wrap, forms max 440 px, Datenschutz is an independent action, and version appears in the rail without an Updates card

#### Scenario: Both credential forms and appearance
- **WHEN** either service form is used at 720 or 1280 and the window narrows/expands while editing
- **THEN** the same stacked fields, secure masking, prefill-on-entry, save-before-login, empty/failure/success copy, busy Save and authenticated Logout from REQ-SCR-05/06 remain operable without recreating input or switching services
- **AND** appearance retains System/Hell/Dunkel and five original light-mode swatches, immediately persisted via the same setters, with wrapping targets, independent selection/focus and keyboard option traversal
- **AND** selecting appearance never logs out and desktop accent changes do not claim dynamic application-icon parity

### Requirement: REQ-SCR-08 Notifications screen
On capable mobile hosts Benachrichtigungen SHALL retain `04-ui-ux.md` §3.8: reminder switch, twelve 11:00–13:45 chips, location alerts and inactive/active/expired 12/24-hour log with mail action. Unsupported desktop SHALL hide the destination at every width and SHALL expose only a truthful unavailable fallback for a stale/debug route, never functioning-looking controls for no-op services.

#### Scenario: Reminder time chips
- **WHEN** the reminder is switched on on a capable mobile host
- **THEN** twelve chips from 11:00 to 13:45 in 15-minute steps appear and the tapped chip is persisted at once

#### Scenario: Company location saved
- **WHEN** location and notification permissions are granted and a fix is obtained on a capable host
- **THEN** the location is stored through core, geofence starts, and the alert "Gespeichert" appears

#### Scenario: Expired log
- **WHEN** log recording has expired with entries on a capable mobile host
- **THEN** "Log per E-Mail senden" and "Log verwerfen" are offered

#### Scenario: Unsupported desktop fallback
- **WHEN** macOS or Windows opens Einstellungen or a stale/debug notification route at any width
- **THEN** the normal list hides Benachrichtigungen and the stale detail displays `Benachrichtigungen sind auf diesem Gerät nicht verfügbar.` without toggles, permission requests, time chips, location actions or mail-success claims
- **AND** narrow fallback has the incumbent Einstellungen back affordance, wide fallback uses the existing detail container, and all mobile notification layouts remain unchanged
