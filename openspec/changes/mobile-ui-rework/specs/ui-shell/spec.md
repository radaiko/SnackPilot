# UI shell delta

## MODIFIED Requirements

### Requirement: REQ-UI-04 Four-tab navigation
The app SHALL present Menüs, Bestellungen, Abrechnung and Einstellungen with the existing bundled tab icons. iOS SHALL retain its icon-only floating pill and Android its labelled bottom bar. Desktop below720 logical content px SHALL retain its labelled bottom bar; at720 and above it SHALL use the Desktop Ledger rail with the same destinations and intents. Mobile inactive ink SHALL follow REQ-UI-15.

#### Scenario: Tab switch
- **WHEN** the user activates Bestellungen by tab, rail, shortcut or macOS menu
- **THEN** Orders is shown and marked active in the current host/layout style
- **AND** changing destination resets departing preview/cursor/settings detail and push stack, not pending menu changes

#### Scenario: Re-tapping the active tab
- **WHEN** the user activates the active destination
- **THEN** nothing navigates and preview, pending changes, scroll and form input are not reset

#### Scenario: Mobile never gets the desktop rail
- **WHEN** iOS or Android renders at360,720 or1280 logical px
- **THEN** it retains platform navigation, safe-area handling and the single-column Mobile Day Tray presentation, never a desktop rail selected by width alone

### Requirement: REQ-UI-06 Bundled fonts, icons and German text
The app SHALL bundle/register Inter and Lucide without system-font glyph dependencies. German strings SHALL remain verbatim except the approved in-app toast `Neue Menüs verfügbar` and log hint `Zeichnet für 12 oder 24 Stunden Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.` Dates, numbers and currency SHALL retain de-AT formatting; existing Aptabase privacy authority SHALL remain unchanged.

#### Scenario: Umlauts render
- **WHEN** Menüs, Abrechnung or Bestell-Erinnerung is shown
- **THEN** every glyph renders from the bundled font on all targets

#### Scenario: de-AT formatting
- **WHEN** a bill amount of1234.5 EUR is shown
- **THEN** it uses the existing de-AT decimal comma and currency formatting

#### Scenario: Exactly two approved substitutions
- **WHEN** the new-menu in-app toast or diagnostic log hint is shown under C1/C2 approval (human, 2026-11-13)
- **THEN** only those exact two strings differ from incumbent German copy
- **AND** OS notification strings, busy labels, dialog/error bodies, formatter output, source/category caps and external/operator text do not change

### Requirement: REQ-UI-07 Desktop shell
macOS and Windows SHALL render the shared UI in a native decorated, resizable window, minimum content size360×480, default1180×780 clamped to the work area. Desktop SHALL use flat Mobile Day Tray below720 logical content px and Desktop Ledger at720 and above, follow system appearance, and keep one main session. Linux remains compile-only where supported by the pinned stack.

#### Scenario: macOS window
- **WHEN** desktop starts on Mac with isolated demo data
- **THEN** a decorated resizable SnackPilot window shows shared UI with native controls

#### Scenario: Wide window
- **WHEN** desktop content width is720–1023 excluding native chrome
- **THEN** a176 px rail replaces the bottom bar; workspace outer gutters16, master248, divider1 and remaining detail are unchanged
- **AND** at720 workspace512/detail263; at768 workspace560/detail311

#### Scenario: Expanded desktop
- **WHEN** desktop content width is at least1024
- **THEN** rail192, gutters at least24 and workspace max960 centred right of rail remain unchanged, interior split56/44 excluding divider
- **AND** at1280 workspace gutters are64 each, without the obsolete480 cap

#### Scenario: Phone fallback
- **WHEN** desktop content width falls below720
- **THEN** flat Day Tray uses min(viewport−32,560) centred content and440 form cap with labelled64 px bottom bar, no mobile safe-area strip
- **AND** desktop native editing, keyboard/focus/menu policy, one input instance, off-thread dispatch and unsupported notification hiding remain host-gated, not rail-gated

#### Scenario: Short windows preserve width-selected layout
- **WHEN** height is constrained at any width
- **THEN** width-selected layout remains and content/navigation/modal messages are scroll-reachable without hiding modal actions

#### Scenario: Native window lifecycle
- **WHEN** the main Mac window closes and Dock activates the app again
- **THEN** the same session reopens one main window without a second model/bootstrap, remembering bounds where supported
- **AND** close/quit is not logout or credential deletion

## ADDED Requirements

### Requirement: REQ-UI-15 Mobile Day Tray presentation
Compact shared screens SHALL follow the Mobile Day Tray contracts in this change's design: three text roles, named spacing, readable derived ink, measured obstruction clearance and single-column caps. Base palette values and wide Desktop Ledger SHALL remain unchanged. Meaningful text SHALL meet4.5:1 and control/selection/focus boundaries3:1 against actual composited resting/pressed fills for all accents and appearances.

#### Scenario: Geometry and platform identity
- **WHEN** iOS glass or Android flat renders at360/390/768 logical px
- **THEN** content widths are328/358/560 with16/16/104 gutters, forms cap440, safe top applies once and existing navigation identity is retained
- **AND** mobile at1280 retains560 content cap; no record grid or sidebar appears
- **AND** titles24/32 Bold, body16/24 and metadata14/20 use bundled Inter, with12 px Android tab labels the intentional exception

#### Scenario: Accent, focus and ink
- **WHEN** any of five accents in light/dark is rendered in rest/pressed/selected/disabled/focused states
- **THEN** readable secondary/status/danger ink falls back to existing primary text as needed, filled action ink resolves to existing black/white, and independent focus2/gap2 does not shift or clip
- **AND** pending selection uses accent tint and visible check/boundary, primary action uses token-derived fill, no coloured action shadow is added, disabled context remains readable

#### Scenario: Scope-safe projection
- **WHEN** desktop crosses719→720→719 or mobile resizes768→390
- **THEN** only presentation changes; inputs, pending changes, filters, segment and valid identities survive without fetch/mutation caused by resize
- **AND** wide Desktop Ledger geometry/recipes and all host capabilities remain unchanged

### Requirement: REQ-UI-16 Mobile interaction and reachability
Mobile controls SHALL provide non-overlapping44×44 targets, primary actions48 high, independent focus and fill-only pressed feedback. Long text SHALL wrap before clipping, dialogs SHALL keep actions reachable and resolve once, and measured navigation/dock clearance SHALL reveal the last record. Existing input, gesture and mutation guards SHALL remain authoritative; reduced motion SHALL remove travel without changing toast lifetime or events.

#### Scenario: Short viewport and large type
- **WHEN** a360 px viewport uses at least150% effective text scale or a credential keyboard reduces usable height
- **THEN** headings/control labels wrap, controls grow/scroll, focused input and Save remain reachable and the last record clears navigation and any measured dock without double padding

#### Scenario: Focus, press and repeated action
- **WHEN** an enabled control is focused, pressed, released outside, scrolled or activated repeatedly
- **THEN** focus is independent of selection, press cancels on outside release/scroll claim, Enter/Space activates a non-input control once and existing guards reject duplicate mutations
- **AND** disabled actions have no callback, read-only receipts/default order bodies are not buttons, native editing/IME and masked copy/cut restrictions remain unchanged

#### Scenario: Overlays and reduced motion
- **WHEN** toast, broadcast or a long modal is presented, including reduced motion
- **THEN** safe-area placement and44×44 broadcast dismissal remain usable, modal body scrolls with actions visible and backdrop/index0 cancels once without pass-through
- **AND** normal toast timing remains300/4000/300 ms, reduced-motion toast has no travel and retains4000 ms lifetime, and reduced-motion day navigation swaps without spring while gesture thresholds/events stay unchanged
