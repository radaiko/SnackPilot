# UI screens delta

## MODIFIED Requirements

### Requirement: REQ-SCR-01 Menus tab
Menüs SHALL retain existing grouping, pending identities, toggle eligibility, all-days submit, exact count/progress labels, refresh and core cutoff decisions. Mobile/narrow desktop SHALL use the Day Tray dish-first cards, readable statuses and reserved wrapping pending dock. Wide desktop SHALL retain Desktop Ledger preview/detail and explicit actions. Rendering, selection styling and resize SHALL NOT add requests.

#### Scenario: Toggle and submit
- **WHEN** an eligible item is explicitly toggled and Bestellen (1) activated
- **THEN** the existing phases include `Wird in den Warenkorb gelegt...` and `Bestellung wird bestätigt...` and the item becomes ordered only after existing flow succeeds
- **AND** mobile dock submits all pending days using the exact existing three label forms, with no second submit while progress is active

#### Scenario: Day navigation
- **WHEN** forward arrow is activated or mobile/narrow content swiped left
- **THEN** next available day is shown, last-day arrow is inactive, today selection and claim10/commit50/edge0.3 remain unchanged
- **AND** wide desktop uses focused controls rather than drag-to-change-day

#### Scenario: Cutoff day
- **WHEN** selected day is past cutoff
- **THEN** new-order toggles follow the existing core decision, ordered/selected edge cases retain incumbent model eligibility and wide desktop preview remains read-only

#### Scenario: Preview versus pending change
- **WHEN** a wide menu row is clicked or arrowed to
- **THEN** detail immediately shows day/category, complete dish, optional allergens and original price without pending-count change, request or push
- **AND** preview rule/tint remains separate from pending; explicit actions stay `Zur Bestellung hinzufügen`, `Auswahl aufheben`, `Stornierung vormerken`, `Stornierung aufheben`
- **AND** `Vorgemerkt`, `Bestellt`, `Wird storniert`, `Ausverkauft`, `Geschlossen` remain distinct; unavailable/cutoff new items cannot be added

#### Scenario: Wide menu hierarchy and states
- **WHEN** wide Menüs shows populated/empty/auth/loading/refresh/error/progress data
- **THEN** header/day toolbar/banners span panes, category order and SUPPE & SALAT suppression stay intact, detail initially says `Menü auswählen`
- **AND** exact strings/retry remain, initial spinner replaces only absent data, cache remains, long dish/allergens fit263 px detail
- **AND** the nonbusy strip spans panes with existing three count labels/all-days submission, without a new cart/confirmation

#### Scenario: Mobile dish and pending hierarchy
- **WHEN** mobile/narrow Menüs renders a long dish and mixed pending changes at360
- **THEN** subtitle (title only if empty) appears as body text up to four lines under neutral category heading, allergens wrap without loss beside/above unchanged price
- **AND** pending check occupies an inline slot distinct from status badge, pending-cancel is dashed/struck but readable, status precedence stays cancellation→ordered→unavailable→cutoff
- **AND** wrapping dock and progress region clear navigation; last card is fully scrollable above them; cached refresh retains cards/checks/day with neutral status, not a blocking overlay

### Requirement: REQ-SCR-02 Orders tab
Bestellungen SHALL retain Kommende/Vergangene counts, all-unconfirmed confirmation, exact cancel dialog, existing batch flow, single-flight guard, local-midnight split and cutoff decisions. Mobile/narrow desktop SHALL use Day Tray cards and wrapping controls without whole-content dimming. Wide desktop SHALL retain table/detail, preview separate from batch checks and cancellation only through explicit actions.

#### Scenario: Cancel one order
- **WHEN** an eligible cancel is explicitly activated and Stornieren confirmed
- **THEN** mobile/narrow shows spinner in its44 px slot without dimming readable text, hides other cancel controls and wide desktop shows `Wird storniert` while disabling entry points
- **AND** order disappears after existing core refresh without request or eligibility changes

#### Scenario: Batch cancel
- **WHEN** selection mode selects three eligible IDs and invokes existing batch flow
- **THEN** core receives one batch cancel with incumbent immediate-versus-dialog semantics

#### Scenario: Confirm banner
- **WHEN** two orders are unconfirmed
- **THEN** `2 unbestätigte Bestellungen` and Bestätigen confirm all unconfirmed orders, not just preview/checks
- **AND** mobile360/390 stacks text over48 px button, wraps counts and selection strip without clipping

#### Scenario: Desktop order table and detail
- **WHEN** upcoming/past orders render wide
- **THEN** compact columns remain Datum80 and Bestellung with category/status; expanded date104, flex Bestellung/excerpt and status104
- **AND** dates remain de-AT dd.MM.yyyy, full description fallback/status remain without invented amount/sort
- **AND** initial detail `Bestellung auswählen` and empty/auth/error/refresh strings/cache behaviour stay intact

#### Scenario: Explicit cancellation and cutoff
- **WHEN** wide detail cancel is evaluated
- **THEN** preview never cancels/approves; eligible action opens `Bestellung stornieren` / `"{order.title}" stornieren?` with Behalten/Stornieren
- **AND** past detail has no cancel, cutoff upcoming disables it with `Stornierung nicht mehr möglich.` on existing30-second cadence
- **AND** mobile uses the same exact dialog and model cutoff, shows a visible disabled cancel without callback and retains readable description fallback

#### Scenario: Batch checks are not previews
- **WHEN** upcoming desktop enters Auswählen
- **THEN** wide44 px check column and row-Space toggle only batch ID once, body previews and check/preview stay separate
- **AND** Stornieren ({n}) disables zero/busy, Fertig/Esc exit, switching past exits without cancellation and segment/checked IDs survive resize/local-midnight parity
- **AND** mobile selection retains whole-card toggle, reserved44 px check area and wrapping counted Stornieren/Fertig controls, with no simultaneous cancel action from that tap or new gesture

### Requirement: REQ-SCR-03 Billing tab
Abrechnung SHALL retain three months, Alle/Kantine/Automaten, newest-first stable merge, existing filtered Gesamt/Belege/conditional Zuschuss, de-AT values, errors and forced refresh. Mobile/narrow desktop SHALL use Day Tray cards, primary-total summary and persistent loading-context controls; wide desktop SHALL retain receipt table/detail. Either authenticated service SHALL suffice; receipt bodies SHALL remain read-only.

#### Scenario: Source filter totals
- **WHEN** Kantine is selected
- **THEN** only Gourmet bills are listed and Automaten share is zero; subsidy remains conditional and absent for Automaten

#### Scenario: Month switch
- **WHEN** an uncached month is selected
- **THEN** initial spinner shows with current month/source controls until its data arrives, never old-month receipts under new-month label

#### Scenario: Empty month
- **WHEN** loading ends without entries
- **THEN** `Keine Abrechnungsdaten für diesen Monat` appears

#### Scenario: Desktop billing workspace
- **WHEN** billing renders wide
- **THEN** month/source toolbars and compact reconciliation span split, conditional Zuschuss remains from current totals
- **AND** compact Datum / Quelle and trailing Betrag80, expanded date112/flex source/amount96 remain tabular without horizontal scroll
- **AND** initial detail `Beleg auswählen` and Beleg/date/time/source/amount/full Gourmet lines or nonempty Ventopay restaurant stay unchanged without invented fields

#### Scenario: Source independence and refresh
- **WHEN** one service is authenticated, refresh runs or context changes
- **THEN** signed-in source remains usable; only neither-authenticated gates; selected-context cache stays readable and errors inline, without fabricated missing-source totals
- **AND** desktop context change clears hidden preview/resets master scroll; arrows preview existing merged order without fetch
- **AND** equal date/amount entries keep distinct IDs; negative/zero/large amounts and long lines fit compact detail

#### Scenario: Mobile receipt readability
- **WHEN** either source renders long receipt data at360
- **THEN** month labels wrap, filters are >=44 with check+tint and summary shows Gesamt first then wrapping Belege/conditional Zuschuss
- **AND** date/amount are body Semibold, source/time neutral readable metadata; quantities28 px, descriptions wrap and amounts trail without changing formatter output or inventing Ventopay items

### Requirement: REQ-SCR-04 Settings tab and dialogs
Einstellungen SHALL retain live independent session/theme hints, destination order, capability-gated notifications and existing Aptabase Datenschutz authority. Mobile/narrow desktop SHALL use Day Tray wrapping rows/back navigation and shared accessible-sized dialogs. Wide desktop SHALL retain settings containment and desktop modal/privacy policy. All dialogs SHALL preserve exact call shapes, text and single resolution.

#### Scenario: Row hints
- **WHEN** Gourmet is authenticated for demo
- **THEN** Kantine hint is `Angemeldet als demo`; Ventopay's independent hint stays truthful
- **AND** mobile long hints wrap without colliding with reserved chevron; rows min80 and privacy min48

#### Scenario: Datenschutz
- **WHEN** Datenschutz opens
- **THEN** it describes self-hosted Aptabase under07-release authority, not historical TelemetryDeck, with only desktop's existing truthful no-statistics paragraph added
- **AND** this change adds no privacy substitution

#### Scenario: Desktop settings master
- **WHEN** wide settings row is selected
- **THEN** initial `Einstellung auswählen` is replaced by same detail without push/back row; order remains credentials, Darstellung, capability-gated notifications
- **AND** long titles wrap, forms cap440, privacy stays independent, version in rail without Updates card

#### Scenario: Both credential forms and appearance
- **WHEN** a service form is used at720/1280 then resized through719
- **THEN** same fields/input entities keep unsaved text, service, secure masking, prefill-on-entry, save-before-login, busy state and exact dialogs/logout
- **AND** theme/accent keep setters, wrapping targets, independent focus and desktop option traversal; appearance never logs out or promises desktop dynamic icon parity

#### Scenario: Mobile modal reachability
- **WHEN** a long alert or confirm opens at360 with keyboard/large type
- **THEN** body scrolls, action targets remain visible/reachable, backdrop/index0 cancels and resolution/focus ownership remain single, without background activation

### Requirement: REQ-SCR-05 Kantine login screen
Kantine-Zugangsdaten SHALL retain §3.5 credential behaviour: prefill, empty-field alert, persist-before-login, Speichern... busy, exact success/failure alerts and Logout retaining credentials/cookies. Mobile/narrow desktop SHALL use the Day Tray keyboard-safe440-cap form, min48 inputs/actions and neutral outlined Logout, preserving input entities/native editing. This rework SHALL NOT add mobile Return-submit.

#### Scenario: Empty field
- **WHEN** Speichern is activated with an empty password
- **THEN** Fehler / Bitte Benutzername und Passwort eingeben appears and nothing is saved

#### Scenario: Failed login still saves
- **WHEN** wrong credentials are submitted
- **THEN** credentials persist before login and Login fehlgeschlagen shows existing mapped message/fallback

#### Scenario: Mobile keyboard and session
- **WHEN** Kantine form at360 has focused username/password, long error or active session
- **THEN** field and Save/dialog actions remain reachable, password stays masked/non-copyable/non-cuttable, repeated Save is claimed once and Logout preserves fields/credentials/cookies
- **AND** title wraps without shrinking, session text is exact and independent of Ventopay

### Requirement: REQ-SCR-06 Automaten login screen
Automaten-Zugangsdaten SHALL retain §3.6 independent Ventopay binding, exact title/subtitle/session/dialog strings, prefill and save-before-login semantics. Mobile/narrow desktop SHALL use the shared Day Tray form contract with Ventopay busy state. Successful login SHALL load Automaten transactions into Abrechnung without restart; styling SHALL NOT consolidate services or alter credential/keyring behaviour.

#### Scenario: Session text
- **WHEN** Ventopay is authenticated
- **THEN** Automaten-Sitzung aktiv appears without invented username

#### Scenario: Logout keeps credentials
- **WHEN** Abmelden is activated
- **THEN** Ventopay session ends while saved credentials/cookies remain

#### Scenario: Mobile independent form
- **WHEN** Automaten form renders at360 with keyboard, long title/subtitle or busy Save
- **THEN** exact strings wrap, Save remains reachable/min48 and only ventopay_busy controls its busy presentation
- **AND** login/logout changes only its independent session display; success remains Automaten-Zugangsdaten sicher gespeichert and existing billing refresh occurs

### Requirement: REQ-SCR-07 Appearance screen
Darstellung SHALL retain immediate/persisted System/Hell/Dunkel, five existing accent IDs/order/light-mode fills and icon coupling. Mobile/narrow desktop SHALL use Day Tray's single inline theme group and deterministic five-column swatches, with deliberate large-text3+2 fallback and independent readable check/ring/focus. Wide desktop preferences and keyboard policy SHALL remain unchanged.

#### Scenario: Swatch selected state
- **WHEN** Gold is selected in dark appearance
- **THEN** its light-mode fill remains, with readable black/white check and independent2 px TEXT_PRIMARY ring plus2 px background gap on mobile

#### Scenario: Five readable options
- **WHEN** Darstellung renders at360 normally then at150% effective type
- **THEN** all five names fit default five columns, large text deliberately uses3+2 without clipping, theme options stack only when needed
- **AND** preference/accent remain independent, local/offline setters persist immediately, System follows OS and app-icon coupling remains incumbent

### Requirement: REQ-SCR-08 Notifications screen
Capable mobile Benachrichtigungen SHALL retain §3.8 reminder switch/twelve times, permissions, location flow and inactive/active/expired12/24-hour log/mail semantics. Day Tray SHALL provide wrapping secondary actions and readable selected/disabled controls without changing the200-entry cap. Unsupported desktop SHALL hide the destination at every width and show only its truthful fallback on stale/debug routes.

#### Scenario: Reminder time chips
- **WHEN** reminder is enabled on capable mobile
- **THEN** twelve11:00–13:45 quarter-hour chips appear, tapping persists immediately and saved selection is revealed on entry
- **AND** off hides chips, denied permission keeps off with exact existing alert/settings path; no new picker/range

#### Scenario: Company location saved
- **WHEN** location/notification permissions and fix succeed
- **THEN** existing core storage/geofence/alert sequence remains, with wrapping min48 neutral setup/removal and exact busy/failure/status text

#### Scenario: Expired log
- **WHEN** log expires with entries
- **THEN** Log per E-Mail senden and Log verwerfen remain, with unchanged status/count/expiry/export recipient/subject/body/failure/discard behaviour
- **AND** inactive duration actions remain12/24 hours but use secondary min48 wrapping controls; only C2 changes the hint

#### Scenario: Unsupported desktop fallback
- **WHEN** desktop opens settings or stale/debug notifications at any width
- **THEN** normal list hides the row and stale detail says `Benachrichtigungen sind auf diesem Gerät nicht verfügbar.` without controls, permission requests or mail-success claims
- **AND** narrow back affordance and wide containment stay intact

## ADDED Requirements

### Requirement: REQ-SCR-10 Existing authentication gates
Anmeldung SHALL mean the existing Menüs/Bestellungen/Abrechnung gates, not a new route or credential surface. Gates SHALL keep exact current text, readable neutral hierarchy and a min48 outlined Einstellungen action selecting the existing tab. Pending authentication SHALL show a spinner rather than false signed-out text using authoritative UI-model lifecycle status. Cached authenticated content SHALL NOT be discarded merely for offline/refresh state.

#### Scenario: Pending, signed-out and failed authentication
- **WHEN** existing service lifecycle is pending, signed-out/no-credentials or failed
- **THEN** pending shows spinner, signed-out shows incumbent gate plus Einstellungen, failure retains exact model error separately and the same settings path
- **AND** no timer, new credential read/network call, empty-array inference or new auth flow determines that projection

#### Scenario: Billing service independence
- **WHEN** either billing service is authenticated
- **THEN** billing content remains usable without a global gate or requirement to connect the other service

#### Scenario: Gate destination
- **WHEN** Einstellungen is activated on any signed-out gate
- **THEN** existing settings tab is selected once with current active-tab/navigation policy, without new route/provider chooser or automatic credential deep link
