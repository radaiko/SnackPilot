# SnackPilot desktop UI redesign

## Decision and scope

**Mode: Operate. This is a desktop redesign and a shipping-platform promotion, not a refinement.** Replace the phone-in-a-window presentation on wide macOS windows with a labelled navigation rail and a persistent master–detail workspace. Extend the incumbent `AppTheme` system; do not introduce a second design system or duplicate screen implementations. Name the extension **SnackPilot Desktop Ledger**.

The product remains a German-language cafeteria ordering and billing tool for employees: choose lunch, check that orders are confirmed, and reconcile Kantine and Automaten receipts. Desktop means **macOS** in this brief, matching the existing desktop host. Windows, Linux, a web app, new background services, and restoring the retired Tauri/Velopack stack are not implied.

The seven human decisions are fixed: labelled sidebar, two-column master–detail, keyboard control, pointer/focus states, native macOS menus, denser content without the 480 px desktop cap, and a single responsive UI code path. The responsive threshold is **720 logical content-area px, inclusive**, only on the desktop host. The human's threshold overrides generic 768/1024 design defaults. iOS and Android retain their current layout at every device width.

Only this brief is authored here. It contains design decisions and acceptance conditions, not implementation tasks. `spec` owns requirements amendments, the work breakdown, dependencies and release engineering.

## Basis and evidence limits

Read as the incumbent contract:

- `docs/requirements/00-overview.md`, `04-ui-ux.md` (especially §§2, 3.5, 4.1, 7–8), `08-gpui-desktop.md`, and `03-features/orders.md`.
- `README.md`, `tools/devops/README.md`, and `Makefile` for product and current shipping scope.
- `src/ui/app/src/{app.rs,lib.rs,theme.rs,bin/desktop.rs,gallery.rs}`, all eight screen modules, the navigation model, desktop platform bridge, billing model, and the shared button, chip, card, switch, text-field, dialog, tab-bar, order and bill components.

`docs/design/DESIGN.md` and `artifacts/ui/` do not exist in this checkout. The existing design system nevertheless exists in requirements §7/§8 and `theme.rs`; it wins. Before implementation, the spec handoff must establish `DESIGN.md` as an inventory of that incumbent system plus the explicitly desktop-scoped extension below: palette, Inter scale, spacing, radii/elevation, motion, Lucide mapping and named components. This brief is self-contained without that future inventory and does not authorize changing mobile tokens.

Images actually inspected are under `docs/evidence/gpui-kit-ui-rewrite/`: `4.1/b-gallery-light.png`, `4.2/{tab0,dialog}.png`, `5.7/a-ios-menus-after-bootstrap.png`, `5.4/0-settings.png`, `5.5/a-ios-appearance.png`, `5.6/a-ios-notifications.png`, and the `5.2`/`5.3` orders/billing auth-gate images. They show the incumbent phone surfaces, not the proposed desktop workspace. No existing 768/1280 desktop screenshots or authenticated order/billing screenshots were available in the inspected evidence. The layouts below are **proposed contracts, not descriptions of a rendered desktop UI**. No new screenshots or visual validation were produced. Skill reference-file reads and shell execution were denied by this subagent's permissions; the supplied skill's embedded contract fields are followed, but its external reference templates could not be loaded. There is no active OpenSpec change directory to validate yet.

## Direction

**World:** a printed canteen weekly menu beside a receipt ledger on an employee's desk. Dates, menu categories, approval words and aligned euro amounts are the organising material. **Anti-reference:** an administrative SaaS dashboard with KPI cards, avatars, search bars and a decorative topbar. Four destinations earn a rail; comparing a short list with the complete dish or receipt earns a split pane. Neither is a reason to add more chrome.

Use opaque surfaces on desktop, the existing warm Orange accent by default, and the existing user-selectable Smaragd, Beere, Gold and Ozean themes. System appearance is the default preference; Hell and Dunkel remain explicit choices. Orange belongs to SnackPilot's existing food/ordering identity, not a new desktop brand. No gradients, decorative glass, large metrics, invented food photography, uppercase eyebrow labels or emoji icons. Existing literal menu category names such as `MENÜ I` remain data, not decorative headings.

Six palette anchors from the incumbent Orange theme (light / dark; aliases, not screen literals):

| Role | Existing token | Light | Dark |
|---|---|---|---|
| Working surface | `surface` | `#FFFFFF` | `#1C1C1E` |
| Ink | `text_primary` | `#1D1D1F` | `#F5F5F7` |
| Secondary ink | `text_secondary` | `#6E6E73` | `#A1A1A6` |
| Accent | `primary` | `#D4501A` | `#FF6B35` |
| Danger | `error` | `#D93025` | `#EA4335` |
| Success | `success` | `#34A853` | `#34A853` |

Keep existing `background`, borders, semantic tints and all five accent tables. Every colour and measurement below is resolved through `AppTheme` tokens/recipes, including hover and focus. Hex values appear here only as palette documentation. Do not insert colour literals or ad-hoc spacing into screens.

**Type:** bundled Inter only, using the existing Regular, Semibold and Bold assets; no additional font dependency. Desktop titles 24/30, detail headings 20/26, body and table cells 14/20, controls 14/20 Semibold, metadata/table headers 12/16. Amounts use tabular figures, trailing alignment. Full dish text is 16/24 in the detail pane. Long prose is at most 60 characters per line. Mobile type and recipes are unchanged, including existing parity exceptions to the generic 16 px body floor.

**Layout concept:** four quiet destinations beside one date-led list and one complete record.

**Signature:** the selected list row has a narrow accent rule, and its date/category or receipt date is repeated at the top of the adjoining detail. The user always knows which meal or receipt they are acting on. The rule is selection information, not ornament; no second signature is added. The existing new-menu toast is the sole authored arrival-motion moment.

Critique outcome: the meal date, `MENÜ` categories, confirmation state, two receipt sources and EUR alignment are specific to this product. There is no global search, account avatar, dashboard overview, rail collapse control, fake update badge or redundant toolbar. Billing totals stay a single compact reconciliation line, not hero cards. Inter and all colours are retained for continuity rather than replaced for novelty.

## Responsive geometry and shell contract

All sizes are logical px, excluding the native titlebar and macOS menu bar. Window content fills the available height. There is no web viewport-unit requirement in this native app.

| Width / host | Shell | Content and panes |
|---|---|---|
| 360; desktop below 720 | Existing flat phone layout, labelled 64 px bottom bar, no desktop safe-area strip | Existing single column, max 480; centred only when available width exceeds 480. Existing cards, day navigation, floating submit action and settings push/back. No new item-detail routes. |
| 720–1023; desktop | Fixed 176 px labelled rail; no bottom bar | Main region has 16 px outer padding. Workspace fills the remainder: at 720 it is 512 px = 248 px master + 1 px divider + 263 px detail. Master stays 248 px in this range; detail gets the remainder. |
| 1024+; desktop; 1280 reference | Fixed 192 px rail; no bottom bar | Main region has at least 24 px outer padding. Workspace max 960 px, centred within the region right of the rail. Remaining interior width excluding the 1 px divider is split 56% master / 44% detail. At 1280, main-region gutters are 64 px each; panes approximately 537/422 px. |
| iOS / Android, including 720/1280 | Incumbent platform-specific phone shell | Never select the desktop rail/table dialect from width alone. Existing safe-area and iOS-glass/Android-flat rules remain authoritative. |

At 768 desktop, the same compact split is 560 px: 248 / 1 / 311. This is an evidence checkpoint, not another breakpoint. No icon-only rail at any width. No user-resizable divider or nested three-column workspace in this change.

360 desktop / phone structure:

```text
+----------------------------------+
| day / segment / month controls    |
| inline banners                   |
| existing full-width cards        |
|                                  |
|              submit if pending    |
+----------------------------------+
| Menüs | Bestell. | Abrech. | Einst.|  actual labels remain unabridged
+----------------------------------+
Settings detail replaces body and has « Einstellungen.
```

720 desktop (also the structure at 768):

```text
+----------------+------------------------------------------+
| SnackPilot     | Menüs                         refresh    |
|                | <      Mo., 9. Nov.       >              |
| Menüs          | banner / progress                        |
| Bestellungen   +--------------------+---------------------+
| Abrechnung     | MENÜ I             | Mo., 9. Nov.        |
| Einstellungen  | dish excerpt       | MENÜ I              |
|                | price / state      | complete dish       |
|                |--------------------| allergens / price   |
|                | MENÜ II            | explicit action     |
| Version …      +--------------------+---------------------+
|                |                 Bestellen (2)            |
+----------------+------------------------------------------+
       176          master 248       1       detail 263
```

1280 desktop, Bestellungen example (workspace capped at 960):

```text
+------------------+-------------------------------------------------------+
| SnackPilot       |       Bestellungen                       refresh       |
|                  |       Kommende (8) | Vergangene (24)    Auswählen       |
| Menüs            |       2 unbestätigte Bestellungen      Bestätigen       |
| Bestellungen     |       +--------------------------------+-------------+ |
| Abrechnung       |       | Datum  Bestellung       Status  | date        | |
| Einstellungen    |       | 10.11. MENÜ I         Bestätigt | MENÜ I      | |
|                  |       | 11.11. MENÜ II        Ausstehend| full dish   | |
|                  |       | …                              | status      | |
|                  |       |                                | Stornieren  | |
| Version …        |       +--------------------------------+-------------+ |
+------------------+-------------------------------------------------------+
        192           64               960 workspace                64
```

### Shared shell component tree

`Root → responsive layout container → {navigation rail or existing bottom bar, active screen content}`. A wide active screen contains `WorkspaceHeader → screen toolbar(s) → inline banners → SplitWorkspace(MasterList, DetailPane) → optional ActionStrip`. Overlays are painted above the whole shell in existing order: blocking loading, toast, then dialog. Dialogs occlude both rail and workspace.

- Rail: `SnackPilot` in 16/24 Semibold in a 56 px brand row; 12 px inset. Four rows in current `Tab::ALL` order, 44 px minimum height, 4 px gap, 12 px horizontal padding, 20 px Lucide icon, 10 px icon/text gap, 14/20 Semibold label. Map `utensils`, `receipt`, `wallet`, `settings` exactly as today. Bottom footer `Version {version}` in 12/16 secondary ink; not clickable. No credentials in the rail.
- Workspace header: active tab title, minimum 56 px height; title may wrap. Menüs/Bestellungen/Abrechnung have a trailing 44×44 refresh button using Lucide `refresh-cw`, tooltip `Aktualisieren`. Einstellungen has none. Native titlebar title is simply `SnackPilot`, not a duplicate custom toolbar.
- Toolbar and action strip: minimum 56 px, controls at least 44 px tall, gap 8; wrap rather than overlap. Screen toolbars are above both panes, not inside the narrow master. Banners span both panes, remain visible while their content scrolls, and wrap their text. Broadcast content remains above the active screen and below the native titlebar.
- Workspace surface: one opaque panel, 1 px outer border, radius 12, no shadow. A 1 px vertical divider separates panes. Master list and detail body scroll independently. Headers, filters and operation actions do not scroll out with records. Bottom list padding is 16 on wide desktop, not mobile's 100 px reservation.
- Detail padding: 16 at 720–1023, 24 at 1024+. The empty detail prompt is centred with 16 px inset. Full detail text wraps; there is no horizontal scrolling. Footer actions wrap to multiple lines if needed.
- At short heights, the body scrolls; modal messages can scroll independently of their action row. The rail scrolls if its four destinations and footer do not fit; navigation must never disappear below an unreachable edge.

## Desktop Ledger system extension

These are new desktop-scoped token/recipe names, not alternate global values. Existing mobile component defaults remain unchanged. The desktop host below 720 retains existing dimensions and fills but gains keyboard focus and pointer behaviour. Wide-only aliases below apply at 720+ unless explicitly marked “all desktop”.

### Token inventory

| New token | Exact value / resolution |
|---|---|
| `desktop.breakpoint` | 720 |
| `desktop.expandedBreakpoint` | 1024 |
| `desktop.contentMax` | 960 |
| `desktop.railWidthCompact`, `desktop.railWidthExpanded` | 176, 192 |
| `desktop.outerCompact`, `desktop.outerExpanded` | 16, 24 |
| `desktop.masterCompact`, `desktop.masterExpandedFraction` | 248, 0.56 of interior width after divider |
| `desktop.space.1/2/3/4/5/6` | 4 / 8 / 12 / 16 / 20 / 24; zero allowed for flush adjacency |
| `desktop.targetMin`, `desktop.iconTarget`, `desktop.iconSize` | 44, 44, 20; all desktop targets |
| `desktop.brandHeight`, `desktop.headerMin`, `desktop.toolbarMin`, `desktop.actionStripMin` | 56 each |
| `desktop.navRowMin`, `desktop.navGap`, `desktop.navIconGap` | 44, 4, 10 |
| `desktop.tableRowMin`, `desktop.tableHeaderHeight`, `desktop.tableCellPadX`, `desktop.tableCellGap` | 56, 36, 12, 12 |
| `desktop.menuRowMin`, `desktop.settingsRowMin` | 88, 72; grow for wrapped data |
| `desktop.orderDateCompact`, `desktop.orderDateExpanded`, `desktop.orderStatusExpanded` | 80, 104, 104 |
| `desktop.billDateExpanded`, `desktop.billAmountWidth`, `desktop.batchCheckColumn` | 112, 96 expanded / 80 compact, 44 |
| `desktop.dividerWidth`, `desktop.selectionRule` | 1, 3; selection rule is inset and does not change row width |
| `desktop.radiusPanel`, `desktop.radiusControl`, `desktop.radiusNav`, `desktop.radiusBadge` | 12, 8, 8, 8; table rows have radius 0 |
| `desktop.title`, `desktop.detailTitle`, `desktop.dish`, `desktop.body`, `desktop.meta` | Inter 24/30 Semibold, 20/26 Semibold, 16/24 Regular, 14/20 Regular, 12/16 Regular |
| `desktop.controlText`, `desktop.headerText` | Inter 14/20 Semibold, 12/16 Semibold |
| `desktop.formMax`, `desktop.textMeasureMax` | 440 px, 60 characters |
| `desktop.focusWidth`, `desktop.focusGap` | 2, 2; all desktop; reserve 4 px so the ring is never clipped |
| `desktop.focusInk`, `desktop.focusGapFill` | `text_primary`, underlying container fill; ring remains distinct from selection |
| `desktop.panelFill`, `desktop.railFill`, `desktop.hoverFill`, `desktop.pressedFill` | `surface`, `background`, `surface_variant`, `border` |
| `desktop.selectionFill`, `desktop.selectionInk`, `desktop.selectionRuleInk` | `primary_surface`, `text_primary`, `primary` |
| `desktop.secondaryInk`, `desktop.borderInk`, `desktop.disabledInk` | `text_secondary`, `border`, `text_tertiary` |
| `desktop.actionFill` | `primary_dark` in light appearance, `primary` in dark appearance |
| `desktop.actionHoverFill`, `desktop.actionPressedFill` | Linear-sRGB mix of action fill with `text_primary` at 8%, 14% respectively |
| `desktop.dangerFill`, `desktop.dangerHoverFill`, `desktop.dangerPressedFill` | `error`; same 8% / 14% mix with `text_primary` |
| `desktop.onAction`, `desktop.onDanger` | Choose the higher-WCAG-contrast ink of existing constant `SURFACE.light` (white) and `BACKGROUND.dark` (black), independently for the actual rest/hover/pressed fill. No new hue and no assumption that white works with every accent. |
| `desktop.readableInk(fill, preferred)` | Use preferred only when WCAG contrast against fill is at least 4.5:1, otherwise `text_primary`; for solid filled actions use the black/white resolution above. Text on desktop semantic tints uses `text_primary`. |
| `desktop.dialogMax`, `desktop.dialogInset`, `desktop.dialogPad`, `desktop.dialogRadius` | 340, 40, 24, 14; existing modal geometry, all desktop |
| `desktop.dialogTitle`, `desktop.dialogMessage`, `desktop.dialogButtonText`, `desktop.dialogButtonGap` | Inter 17/22 Semibold, 14/20 Regular, 15/20 Semibold, 10 |
| `desktop.toastDurationIn`, `desktop.toastHold`, `desktop.toastDurationOut` | Existing 300 ms / 4000 ms / 300 ms |
| `desktop.dialogFade` | 120 ms linear opacity; no scale or slide |

The incumbent tertiary ink and some accent/semantic text colours are not assumed to pass small-text contrast on every background. For new desktop text use readable ink resolution, not global token changes. Semantic status remains recognisable by words and optional icon as well as tint. Visible focus and input/control outlines must reach 3:1 against adjacent fills; use `focusInk` for the interactive boundary if the existing decorative border does not. Do not change the mobile palette to repair desktop contrast.

### Recipe and component inventory

| New named recipe / composition | Contract |
|---|---|
| `desktop.sidebarSurface` / `DesktopNavigationRail` | `railFill`, square outer corners, 1 px trailing `borderInk`; rail metrics above. |
| `desktop.navigationRow` | Transparent at rest, radius 8; hover `hoverFill`; selected `selectionFill`, `selectionInk`, inset 3 px accent rule; pressed `pressedFill`; focus ring independent. |
| `desktop.panelSurface` / `SplitWorkspace` | `panelFill`, radius 12, 1 px `borderInk`, no elevation; divider and independent scrolling as above. |
| `desktop.tableHeader`, `desktop.tableRow` / `RecordTable` | Header `background`, 36 px; rows `surface`, 56 px minimum, 1 px bottom border, no shadow, no zebra striping. Hover `hoverFill`; selected tint + accent rule. Read-only cells do not get pointer cursors. One keyboard stop for the list, not every cell. |
| `desktop.menuRow` / desktop presentation of `MenuCard` | 88 px minimum, body padding 12, no individual rounded card or shadow; selected preview styling is separate from pending-order styling. Same item data and toggle callback as phone. |
| `desktop.settingsRow` / desktop presentation of `SettingsRow` | 72 px minimum, padding 12, title 14/20 Semibold, hint 12/16, whole row target; same data/hints as phone. |
| `desktop.detailSurface` / `DetailPane` | Same panel fill; no nested card for an entire record. Heading and content hierarchy below; no back button on wide desktop. |
| `desktop.toolbarSurface`, `desktop.actionStrip` | `surface`, square corners; action strip has 1 px top border. Wrap controls with gaps 8, padding 12, minimum height 56. |
| `desktop.buttonPrimary`, `desktop.buttonDanger`, `desktop.buttonSecondary` | Same shared button implementation, desktop recipe variant; 44 px minimum height, radius 8, paddingX 16, control text. Filled recipes use action/danger tokens; secondary uses surface, readable ink and a visible control outline. No button shadow. |
| `desktop.inputField`, `desktop.filterOption`, `desktop.statusBadge` | Input min 44, paddingX 12, radius 8, surface and `border_input`, stronger boundary when needed. Filter min 44, selected tint + check/underline as appropriate. Status badge passive, padding 8/4, radius 8, semantic tint and readable ink; not a button. |
| `desktop.focusRing` | 2 px `focusInk`, 2 px container-colour gap, same shape as target; paint outside target without layout shift. Applied to every interactive element, including legacy narrow-desktop controls. |
| `desktop.dialogSurface` | Existing §4.1 surface, border 0.5, radius 14, padding 24, title/message alignment retained; desktop buttons at least 44 px tall. |
| `WorkspaceHeader`, `WorkspaceToolbar`, `SelectionActionStrip` | Layout compositions of existing buttons/chips/text; no new visual language. |
| `MenuDetail`, `OrderDetail`, `ReceiptDetail` | Extract/reuse the existing record content, formatting and actions; desktop-only containment, not second screen/business-logic copies. |

No new runtime dependency is required by this design: reuse bundled Inter, vendored Lucide, gpui-kit, existing keyring and model/core facilities. Native menu integration and signing use host facilities. Any dependency subsequently chosen by `spec` must be explicitly named there; a data-table or animation library is not implicitly approved.

## Navigation, selection and state

- Navigation destinations remain the four existing tabs. Rail click and Cmd+1–4 call the same model navigation intent; reselecting the active tab is a no-op. Switching to a different tab retains today's push-stack reset semantics.
- The gpui-free model owns the selected settings-detail kind, selected menu composite key, order position ID, receipt ID, orders segment, list cursor and explicit “no preview” state. Existing selected day, month/source filters and pending sets remain model-owned. GPUI focus handles and text-input objects are view resources, not navigation state. No navigation enum, list selection, filter or route may exist only in the desktop host.
- Entering a wide list initially shows **no selected record** and the prompt below. Selecting a row by pointer or arrows updates its detail immediately without push/pop, fetch or mutation. Hover never selects. No double-click action is required.
- Stable identity is mandatory: menus use `{id}|{day}`, orders `position_id`, billing the existing `g-…`/`v-…` IDs. Never use row index as persisted identity. Refresh preserves selected identity if it survives. If it disappears, clear the preview and focus the nearest surviving row, with no automatic action. Day/month/source/segment changes clear their record preview, not the model's pending menu changes.
- A different tab resets the departing preview/cursor and selected settings detail. Existing pending selections are not submitted or cleared by navigation. Re-tapping the current tab does not reset either selection or scroll.
- Crossing 720 changes presentation, not domain state: no refetch, login attempt, save, order toggle or cancellation. Settings selected on wide desktop becomes the existing pushed settings detail when narrowed; its fields and unsaved text survive the resize. Expanding a pushed settings detail re-exposes the settings master and shows that same detail without a duplicate push. Menus/orders/billing become the current inline phone cards when narrowed; no extra detail route is introduced. On expansion, restore the model's selected record if still present.
- Background/resume preserves active tab, filters, pending changes and valid selected record IDs in memory. The existing foreground refresh can invalidate a removed ID, but cannot reset navigation merely because layout re-rendered. Disk restoration across process termination is not added by this brief.
- Esc precedence: visible dialog; otherwise order multi-selection mode; otherwise settings detail or selected record preview; otherwise no action. Esc never discards menu pending changes and never quits the app. When closing a preview, retain list cursor/focus; next arrow or explicit row activation can open a preview again.

## Per-screen contracts

The shared shell, tokens, states and keyboard rules above apply to every contract. “360” means the current mobile-style presentation on a narrow desktop; native iOS/Android remain incumbent at all widths. New titles and controls described as wide-only do not leak into phone layouts. Existing German strings referenced below are retained exactly, including punctuation and pluralisation.

### 1. Menüs and menu detail

**Purpose:** compare the selected day's dishes, explicitly build pending changes, and submit the existing cross-day batch once.

**Layout per width:**

- **360:** unchanged §3.1 day navigator, grouped MenuCards, banners, empty/auth states, swipe-between-days and existing FAB. Tapping a card keeps its current pending-toggle semantics. There is no new phone menu-detail screen.
- **720:** wide header `Menüs`; toolbar has existing 48 px previous/next day controls and centre date/index (centre still jumps to today). Master is a compact grouped menu list; detail is the selected complete menu. Submit strip spans the two panes below them. No desktop drag-to-change-day behaviour; use buttons or the keyboard map.
- **1280:** same hierarchy, wider excerpt column and detail padding 24. Keep one list, not a grid of menus or days. There is no third date sidebar.

**Component tree:** `WorkspaceHeader(Menüs, refresh) → DayNavigator → refresh/progress/error banners → SplitWorkspace(GroupedMenuRows, MenuDetail | prompt) → pending ActionStrip`. Toast is shell-owned. Group order is `MENÜ I`, `MENÜ II`, `MENÜ III`, `SUPPE & SALAT`, `UNKNOWN`; omit empty groups and suppress the standalone `SUPPE & SALAT` group heading exactly as the incumbent does.

**Exact tokens/content:** group heading uses `desktop.controlText`, readable ink; each menu row contains category/title 14/20 Semibold, dish excerpt up to two 14/20 lines, then price and one status 12/16. If the subtitle is empty, use the existing title fallback; do not fabricate a dish. Row grows beyond `menuRowMin` when required. Detail header has formatted day in `meta`, category/title in `detailTitle`, then full dish in `dish`, existing `Allergene: {letters}` only when there are letters, original price string in 16/24 Semibold, status and explicit toggle button. Price is never recomputed from its display string.

**States and copy:**

| Item condition | Desktop row status / detail toggle |
|---|---|
| Available, not ordered, not pending, before cutoff | No status badge; `Zur Bestellung hinzufügen` |
| Pending new order | Check + `Vorgemerkt`; `Auswahl aufheben` |
| Ordered, not pending cancellation | `Bestellt`; `Stornierung vormerken` when the existing model allows the toggle |
| Pending cancellation | `Wird storniert`; `Stornierung aufheben` |
| Unavailable, not ordered | `Ausverkauft`; no enabled add action |
| Cutoff, available, not ordered | `Geschlossen`; no enabled add action |

Preview selection uses the accent rule/tint. A pending order is represented by its check and status, not by treating the row cursor as a pending toggle. Pending cancellation uses the existing struck-through excerpt and warning status, but does not fade the entire desktop text below readable contrast. Record preview stays available for unavailable/cutoff items. Mutation eligibility always comes from the existing model; in particular the brief does not reinterpret the incumbent ordered-item toggle or submission cutoff semantics.

No selection prompt: `Menü auswählen`. No menus: `Keine Menüs verfügbar` in the master; detail prompt remains. Auth failure: `Nicht angemeldet` and `Gehe zu Einstellungen, um Zugangsdaten einzugeben` across the workspace instead of an empty split. Initial auth/data loading with no cache uses the existing spinner treatment inside content. Cached rows remain visible during refresh; `Aktualisiere...` is inline. Errors display the sanitized model message and `Erneut versuchen`, invoking the existing menu refresh. Progress labels remain `Wird in den Warenkorb gelegt...`, `Bestellung wird bestätigt...`, `Bestellung wird storniert...`, `Menüs werden aktualisiert...`.

Pending submit strip appears iff pending count > 0 and no operation is running. Its single trailing primary action uses the exact current count logic: `Bestellen ({n})`, `Stornieren ({n})`, or `Änderungen bestätigen ({n})`. At 720 the long mixed label wraps within the strip; the button grows, never clips. During submission the progress banner occupies its place in the hierarchy and mutation controls are disabled against duplicate activation. Navigation/reading remain available unless a genuinely blocking model operation requires the existing overlay. Do not add an extra confirmation dialog or a second cart workflow.

**Responsive/input:** row click or Up/Down previews only on wide desktop. Tab reaches the detail toggle and then the batch submit action; Enter/Space activates the focused explicit button. Day arrows are never captured while editing a field. Trackpad/wheel scroll the hovered pane. Resize preserves pending keys and selected identity.

**Acceptance:** previewing a dish never changes pending count or calls the server; explicit add/toggle mirrors today's domain action; all pending days are included by the existing submit pipeline, not just the displayed day. First/last day controls disable correctly; noon/cutoff changes do not silently change preview selection. Full long dish and allergens remain readable in the 263 px detail.

**Non-goals:** favourites, search, calendar picker, drag ordering, a cart drawer, dietary interpretation, new cutoff rules or new network request shapes.

### 2. Bestellungen and order detail

**Purpose:** scan upcoming/past orders and approval status, inspect the actual dish and cancel using the existing single/batch flows.

**Layout per width:**

- **360:** unchanged §3.2 cards, segments, confirmation banner, single cancel modal, `Auswählen` and existing multi-select strip.
- **720:** header `Bestellungen`; segment toolbar `Kommende ({n})`, `Vergangene ({n})`, with `Auswählen` only for upcoming. Controls may wrap. Master is a table with columns `Datum` (80 px) and `Bestellung` (remaining width); the latter shows category then status. A complete dish is in detail, not squeezed into another tiny column.
- **1280:** table columns `Datum` (104), `Bestellung` (flex), `Status` (104). Bestellung shows category plus a one-line dish excerpt; detail shows its full text. Segment and confirmation controls remain above the split.

**Component tree:** `WorkspaceHeader → SegmentTabs + selection entry → unconfirmed/error/progress banners → SplitWorkspace(OrderTable, OrderDetail | prompt) → optional SelectionActionStrip`.

**Exact tokens/content:** `desktop.tableHeader`, `desktop.tableRow`, 12 px cell padding/gaps; dates in table are de-AT `dd.MM.yyyy`. Detail: long formatted date (`meta`), order title/category (`detailTitle`), full description (`dish`), approval badge and trailing explicit `Stornieren` danger action. Description fallback remains menu description lookup, then non-empty order subtitle, then title. Do not invent an order amount: the domain does not provide one. Keep existing upstream order ordering; no new clickable sort headers.

**States/copy:** badges `Bestätigt` / `Ausstehend`; prompt `Bestellung auswählen`; empties `Keine kommenden Bestellungen` / `Keine vergangenen Bestellungen`; unauthenticated `Anmeldung erforderlich`. Unconfirmed upcoming banner is exactly `1 unbestätigte Bestellung` or `{n} unbestätigte Bestellungen`, with `Bestätigen` for the existing all-unconfirmed operation, not only the selected row. Keep model errors and progress labels. Cached records remain during refresh.

Single cancellation is an explicit detail action, not an entire-row click or keyboard Delete action. The existing themed modal is `Bestellung stornieren`, message `"{order.title}" stornieren?`, actions `Behalten` / `Stornieren`. Past orders have no cancellation action. Upcoming cutoff orders show disabled `Stornieren` with visible explanatory text `Stornierung nicht mehr möglich.`; eligibility is from the existing 09:00 Europe/Vienna rule. While one cancellation runs, the affected row/detail shows spinner + `Wird storniert`, and all cancellation entry points are disabled. Re-evaluate the cutoff on the existing cadence without waiting for mouse movement. Selection/reading may continue.

**Multi-selection:** distinguish the current preview row from checked order IDs. `Auswählen` enters existing mode and adds a 44 px checkbox hit column; checkbox visual is 20×20. At 720 the remaining Bestellung cell wraps. Clicking the row body previews, clicking its checkbox toggles the batch set without a second event; Space on the focused row toggles its checkbox only in this mode. The strip exposes `Stornieren ({n})` (disabled for n=0 or busy) and `Fertig`. Batch eligibility, request grouping, and immediate-versus-dialog behaviour remain exactly the current model's `cancel_selected_orders` flow; this design adds no new batch confirmation. Esc exits selection mode before closing a preview. Switching to past exits selection mode through the existing model intent, without firing cancellation.

**Responsive/input:** Up/Down moves preview; Enter focuses detail unless a checkbox/control has focus. On shrink use existing order cards, not an order-detail push. Checked IDs and segment survive resize. Single cancellation refresh clears a disappeared preview ID and restores focus to the nearest surviving row.

**Acceptance:** no row activation can cancel or approve an order. A preview and a batch check remain visibly different when on the same row. Count pluralisation, all-unconfirmed scope, past/upcoming split at device-local midnight, model cutoffs and error paths are unchanged. At both table densities all status words remain available without hover.

**Non-goals:** individual approval API, invented prices, export, new sorting, keyboard shortcuts that immediately cancel, revised batch semantics.

### 3. Abrechnung and receipt detail

**Purpose:** reconcile the selected month's Kantine/Automaten receipts without opening and closing each item.

**Layout per width:**

- **360:** existing three month tabs, source chips, summary bar, complete BillCards and pull-to-refresh; no new bill-detail route.
- **720:** header `Abrechnung`; existing three month options in their own full-width toolbar. Second row has `Alle`, `Kantine`, `Automaten`; third compact summary line wraps if needed. Table columns `Datum / Quelle` (flex) and `Betrag` (80), dates above source labels. Receipt detail is alongside it.
- **1280:** same filters, table columns `Datum` (112), `Quelle` (flex), `Betrag` (96). Date cell shows `dd.MM.yyyy` and `HH:mm` below. Amount column is trailing-aligned/tabular throughout; no table-body horizontal scrolling.

**Component tree:** `WorkspaceHeader → MonthOptions → SourceFilters → SummaryLine → error/refresh banner → SplitWorkspace(ReceiptTable, ReceiptDetail | prompt)`.

**Exact tokens/content:** table recipes and body/meta tokens; summary uses `desktop.body` and Semibold amounts, never separate metric cards. Preserve `Gesamt`, `Belege`, and conditional `Zuschuss`. Display `Zuschuss` only when >0 and source is not Automaten; totals/counts respect the source filter. Its text uses readable ink; success tint or icon may reinforce it without being the only distinction.

Receipt detail starts `Beleg` (`detailTitle`), long de-AT date and `HH:mm`, source badge `Kantine` or `Automaten`, and amount 20/26 Semibold. All currency uses the existing `format_eur`/de-AT EUR convention; no new manual formatting. Gourmet detail reuses item rows: `{count}x`, full wrapping description, trailing formatted line total; quantity column 28 px and amount column 80 px with 8 px gaps. At compact detail width, put a line total on a trailing second line when necessary rather than truncate a description. Ventopay detail shows only its non-empty restaurant description; individual transaction items are not rendered. No receipt-number field, location, per-receipt subsidy or tax is invented from unavailable `BillingEntry` fields.

**States/copy:** no selection `Beleg auswählen`; empty `Keine Abrechnungsdaten für diesen Monat`; unauthenticated only when both services lack authentication: `Anmeldung erforderlich`. One signed-in source remains fully usable. Initial spinner only with no entries; cached receipts/selection remain readable during refresh. Model error banner remains inline; refresh button / Cmd+R is the desktop equivalent of existing pull-to-refresh, with no new scrape sequence. Source filters without data remain available and lead to the honest existing empty state.

**Responsive/input:** Up/Down follows the existing merged newest-first order and updates the detail; Enter moves focus into detail for keyboard scrolling. Month/source changes clear preview and reset master scroll; model totals immediately follow the selected context. Arrowing through rows never triggers a fetch. Receipt text is selectable; amounts are not buttons.

**Acceptance:** negative, zero and large amounts align without clipping; identical date/amount receipts remain distinct by existing IDs; Gourmet quantities and Ventopay optional descriptions preserve current display semantics. Filtering never leaves a detail for a hidden source. At 720 the full amount and source label remain visible even if a date wraps.

**Non-goals:** charts, spending goals, editable receipts, CSV/PDF export, sorting controls, invoice downloads, fabricated receipt fields.

### 4. Einstellungen master

**Purpose:** reach the two independent service accounts, appearance and supported notification settings.

**Layout per width:**

- **360:** existing scroll list with dividers and chevrons; pushes retain `Einstellungen` back affordance. Desktop still hides notifications when unsupported.
- **720:** header `Einstellungen`, compact settings rows in master; selected form/detail to the right without push navigation. Initial detail prompt `Einstellung auswählen`.
- **1280:** same split and row order, not a grid or separate settings window. Keep forms at most 440 px and align them to the leading edge of detail.

**Component tree:** `WorkspaceHeader → SplitWorkspace(SettingsRows + Datenschutz, SettingsDetail | prompt)`.

**Exact tokens/content:** `desktop.settingsRow`, title/control text and meta hint; order `Kantine-Zugangsdaten`, `Automaten-Zugangsdaten`, `Darstellung`, capability-gated `Benachrichtigungen`. Gourmet hint `Angemeldet als {username}` or `Nicht angemeldet`; Ventopay `Sitzung aktiv` or `Nicht angemeldet`; appearance `System` / `Hell` / `Dunkel`. On compact desktop the two long credential titles wrap; no abbreviated service names. Privacy remains a 44 px-minimum text button `Datenschutz` below the list and opens the existing exact §3.4 privacy alert. It does not select a fake detail row.

**States:** session/appearance hints update without resetting selection. No artificial loading or “empty settings” placeholder. Errors belong to the relevant form/modal. Shipping does not create an Updates card without a real updater; version lives in the rail and About.

**Responsive/input:** arrows select only available rows. Enter moves to the selected detail's first control. Esc closes the detail and returns focus to its master row. Shrink/expand preserves the selected form and its input text. If a tab change leaves settings, retain today's navigation reset rather than auto-saving credentials.

**Acceptance:** both service forms are reachable independently in signed-out, single-source and fully signed-in states. Unsupported notifications are hidden at every desktop width, not just wide. Selecting Darstellung does not log out or touch service credentials.

**Non-goals:** sidebar account menu, account creation, all-settings search, preferences window, fake updater.

### 5. Kantine-Zugangsdaten detail

**Purpose:** edit securely stored Gourmet credentials, explicitly save/login, or log out.

**Layout per width:** **360** retains §3.5 back/title/form; **720** puts the same form in the 263 px detail without back row; **1280** uses detail padding 24 and form max 440. Fields stack at every width. The long title wraps, never shrinks below its token size.

**Component tree:** `DetailPane → title → username label/input → password label/secure input → Speichern → authenticated session section + Abmelden`. Use `desktop.detailTitle`, `desktop.inputField`, gap 16, label `headerText`, label-to-field gap 4; save is full-width min 44. Session block top gap 24; danger action separate from Save, never inline with a password field.

**Copy/states:** title `Kantine-Zugangsdaten`; labels `Benutzername`, `Passwort`; placeholders `Benutzername eingeben`, `Passwort eingeben`; action `Speichern` / disabled `Speichern...`; session `Angemeldet als: {username}` and `Abmelden` only authenticated. Saved credentials prefill on entry, never on every repaint. Password remains masked, supports paste, and never appears in logging, tooltips, About or screenshots. Save with missing input opens `Fehler` / `Bitte Benutzername und Passwort eingeben`. Success alert `Gespeichert` / `Kantine-Zugangsdaten sicher gespeichert`; failure `Login fehlgeschlagen` / sanitized model error or `Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.`.

**Responsive/input:** Tab username → password → Save → Logout when available. Return in either single-line field invokes Save once when enabled, not a global action behind a modal. Cmd+A/C/X/V and undo/redo behave as text editing; copying/cutting masked password content is disabled. Resizing does not rebuild inputs from secure storage. Save retains persist-before-validate semantics; logout retains current credential/cookie semantics. There is no auto-save or unsaved-change modal.

**Acceptance:** exact existing auth pipeline/error copy, no duplicate save on key repeat/double click, errors do not navigate away, form remains operable with keyboard only at 720 and after narrowing.

**Non-goals:** password reveal, autofill provider integration, alternate account slots, revised validation/storage order.

### 6. Automaten-Zugangsdaten detail

**Purpose:** the same explicit credential flow for Ventopay, independently of Gourmet.

**Layout per width:** **360** current §3.6 push/back; **720** same stacked form in compact detail; **1280** same 440 px max. Component tree, tokens, states, keyboard, acceptance and non-goals are identical to Kantine except the following exact content and service binding.

Title `Automaten-Zugangsdaten`; subtitle `Für Automaten und Kassenabrechnungen` in body/secondary ink, gap 4 below title and 24 before fields. Success `Gespeichert` / `Automaten-Zugangsdaten sicher gespeichert`; authenticated session text `Automaten-Sitzung aktiv`, never an invented username. Use Ventopay's own busy, auth, save/login and logout intents. A Gourmet operation must not make this form report Ventopay success.

### 7. Darstellung detail

**Purpose:** keep current system/light/dark and accent preference editing, with visible keyboard selection.

**Layout per width:** **360** current §3.7 back/title and two cards. **720** detail title, then `Design` and `Akzentfarbe` sections separated by 24 px; no extra nested outer card. Three design options remain equal-width in one row, each min 44 high, gap 8, wrapping labels. Five accent targets wrap into as many leading-aligned 56 px columns as fit, gap 8. **1280** same hierarchy; all five can occupy one row if the available inner width permits, otherwise wrap by the same rule. Do not force five swatches into 215 px.

**Component tree/tokens:** `DetailPane → title → Design(three filter options) → Akzentfarbe(five swatch targets)`. `desktop.detailTitle`, `desktop.controlText`, `desktop.filterOption`; design icons remain current Smartphone/Sun/Moon at 20. Swatch visual remains 40×40, target min 44×44 plus visible label; checkmark and selected border remain, with an independent focus ring around the full target.

**Copy/states:** `System`, `Hell`, `Dunkel`; `Orange`, `Smaragd`, `Beere`, `Gold`, `Ozean` in existing order. Light-mode swatch colours remain even in dark mode. Names and selected check distinguish similar colours. Changes persist immediately through current model setters. System reflects current macOS appearance; unsupported alternate Dock-icon mutation must not be claimed merely because the accent changes.

**Responsive/input:** Tab reaches each option group; Left/Right moves among its choices and selects the choice, Home/End first/last. In a wrapping swatch group arrows still follow logical order, not guessed visual coordinates. Theme change preserves focus. Esc closes detail, not undoing an already persisted choice.

**Acceptance:** all ten appearance/accent combinations render legibly; focus is distinguishable from selection; resizing doesn't revert a preference or lose the current detail.

**Non-goals:** new themes, custom accent picker, new fonts, decorative preview mockups, desktop Dock-icon promise.

### 8. Benachrichtigungen detail / capability boundary

**Purpose:** retain the existing mobile screen without suggesting unsupported desktop delivery.

**Layout per width:** on **360, 720 and 1280 desktop**, this row/detail is absent with today's capabilities (`notifications=false`, `geofence=false`). This shipping promotion does not silently enable a previously stubbed OS service. A stale restored/debug route shows `Benachrichtigungen` and `Benachrichtigungen sind auf diesem Gerät nicht verfügbar.` with no toggles, time chips, location buttons or log-export success claims; narrow has `Einstellungen` back, wide follows the detail container. This fallback is not a new navigable destination.

On iOS/Android at **all three widths**, keep §3.8's current scrollable sections and layout: Bestell-Erinnerung with `Aktiviert`, `Uhrzeit` and twelve 11:00–13:45 chips; Standort-Benachrichtigungen; Benachrichtigungs-Log with inactive/active/expired branches. Android battery hint remains Android-only. Existing permission/error messages, log cap and email behaviour remain unchanged.

**Component tree/tokens:** existing notification screen on capable hosts; unavailable fallback uses `detailTitle`, `body`, readable secondary ink and the shared responsive detail container. No new notification component/system dependency.

**States/input:** unavailable is a truthful capability state, not a disabled imitation of the mobile controls. If native desktop notifications are separately approved later, that is another platform-service contract; this brief must not be interpreted as its approval.

**Acceptance:** desktop launch/bootstrap does not request unsupported permissions; no apparent “enabled” control whose platform implementation is a no-op. Mobile notification screenshots and flows remain unchanged.

**Non-goals:** adding macOS reminders/geofencing, mail integration, background menu checks or local notification support as part of this UI redo.

### 9. Themed dialogs, privacy and About

**Purpose:** retain the exact existing alert/confirm flows and make them fully keyboard operable; expose standard macOS About without a second settings surface.

**Layout per width:** **360** content width `min(340, 360−80)=280`; **720** and **1280** width 340. Centre in the entire window content area, not just the detail or max-width workspace. Native titlebar/menu bar are outside the modal. iOS/Android retain current §4.1 geometry. No native alert substitution.

**Component tree/tokens:** `DialogOverlay(overlay fill; occludes shell) → DialogSurface → title → optional message → button row`. Existing 0.5 px border, 14 px radius, padding 24, centred 17 px title and 14/20 message, 8 px title/message gap, 20 px before actions, action gap 10. Desktop buttons min 44 high. On a short window the message scrolls, title/actions remain visible; button labels wrap before truncating. Keep cancel outline, primary OK, danger confirm recipes and their contrast-resolved desktop inks.

**Copy:** alert has `OK`; default confirm `Abbrechen` / `Bestätigen`; cancellation override `Behalten` / `Stornieren`. Credential/permission messages remain as specified above and in §3.8. Privacy alert retains the exact incumbent paragraph:

> Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.

Desktop analytics currently remains a no-op. Add a desktop-only second paragraph to avoid implying otherwise: `Auf dem Mac werden derzeit keine Nutzungsstatistiken gesendet.` Do not change mobile privacy copy.

About menu opens one themed alert titled `Über SnackPilot`; body lines `SnackPilot`, `Version {version} ({build})`, blank line, `Bestellungen und Abrechnung für die Betriebskantine.`, blank line, `Basierend auf GourmetClient von patrickl92.`, then `github.com/patrickl92/GourmetClient`. Version/build are real release metadata, not hardcoded placeholders. Single `OK` button; no animated icon, credentials, device diagnostics, unimplemented support link or update action. macOS retains its normal app icon/menu branding; the in-app modal does not need another logo.

**States/input:** exactly one dialog at a time. Focus is trapped; alert initially focuses OK, confirm initially focuses the visibly ringed confirm button. Enter activates the focused action (initially the requested confirm), Space activates focused button, Tab/Shift+Tab cycle only within the modal, Left/Right can move between its action buttons. Esc/backdrop always resolves button index 0/cancel, never confirmation. Body click does not dismiss. The key event that opened the dialog and its auto-repeat must not also confirm it. Disable a dispatched mutation immediately against repeat activation. On dismiss restore the originating control; if its record disappeared restore the nearest list row, otherwise screen heading. Application quit/hide shortcuts stay native; navigation/refresh commands are disabled while modal.

**Acceptance:** pointer, Enter, Space, Esc and backdrop all resolve the same underlying single dialog API correctly; no click/key passes through to the rail, another order or the background form. Destructive confirmation remains visibly labelled and is never triggered merely by list selection.

**Non-goals:** stacked dialogs, sheets replacing all settings details, native alert restyling, new confirmation flows or system permission replacements.

## Keyboard map and focus contract

These bindings apply on macOS at both compact and wide widths. Do not replace native text editing or intercept OS-reserved commands. Phone touch gestures remain unchanged. All mutation shortcuts ignore key auto-repeat and obey the exact same busy/eligibility guard as pointer activation.

| Key | Scope | Result |
|---|---|---|
| Cmd+1 / 2 / 3 / 4 | App, no modal | Menüs / Bestellungen / Abrechnung / Einstellungen. Active destination is a no-op. When changed, focus the new screen heading, not a destructive action. |
| Cmd+, | App, no modal | Einstellungen; same navigation as Cmd+4. |
| Cmd+R | Menüs/Bestellungen/Abrechnung, no modal, not already refreshing or mutating | Existing active-tab refresh pipeline once; disabled elsewhere. No extra request variant. |
| Esc | App | Cancel modal; otherwise exit batch selection; otherwise close preview/settings detail or pop narrow settings stack; at root no-op. Does not clear pending menu changes. |
| Tab / Shift+Tab | App | Move across interactive regions and controls in visual reading order; one tab stop per navigation/list/option group, not every record cell. Reveal focused item without changing unrelated scroll panes. |
| Up / Down | Focused rail | Move focus through four destinations without activating; Enter/Space activates. No wrap at ends. |
| Up / Down | Focused wide record/settings list | Select previous/next visible record, reveal it and update detail; no mutation. From no cursor, Down selects first, Up selects last. No wrap. In narrow settings list, move focus only; Enter pushes the detail. |
| Up / Down | Focused narrow menu/order/billing list | Move the keyboard cursor between existing cards, scroll it into view; do not add a new detail presentation. |
| Home / End | Focused list | First/last record, same preview behaviour as arrows. In an input retain native text meaning. |
| Left / Right | Focused Menüs day navigator | Previous/next available date; do not wrap or bypass end-disabled controls. Centre activation jumps to today with existing nearest-date fallback. |
| Left / Right | Focused segment/month/source/design/accent option group | Move to neighbouring option and select it immediately, no wrap. In inputs retain caret movement. |
| Enter | Focused wide list row | Keep preview selected and focus first enabled control in detail; if read-only, focus detail scroll region. Never order/cancel by list activation. |
| Enter | Focused narrow card or settings row | Activate only its existing action; a narrow menu card toggles pending selection as today. Passive bill/order cards remain read-only outside their explicit actions. |
| Enter / Space | Focused button/chip/switch/checkbox | Same enabled action as pointer. In batch mode Space on focused order row toggles its check, not preview selection. |
| Enter | Dialog | Activate focused action, initially OK/confirm. One event, one resolution. |
| Enter | Credential field | Submit Save once when enabled and no modal. |
| Cmd+A/C/X/V, Cmd+Z, Shift+Cmd+Z | Editable text | Standard selection/copy/cut/paste/undo/redo; no app-level Select All orders, and no copy/cut of masked passwords. |
| Cmd+W | App | Close window normally; no logout, credential deletion or background task promise. |
| Cmd+M, Cmd+H, Option+Cmd+H | App | Native minimise, hide app, hide others. |
| Cmd+Q | App | Native quit; never a model logout. |

Focus order wide: active rail item/group → optional broadcast dismiss → screen toolbar controls → banner actions → master list → detail controls/scroll region → operation strip. On first launch do not auto-focus a login field or pop up a keyboard. The rail, each option group and each list use a roving cursor; Tab exits the group. Read-only detail bodies can receive scroll focus but do not masquerade as buttons. Arrow keys belong only to the focused group; they must not change a day while the person scrolls a receipt or types a username.

## Pointer, hover, focus and disabled states

- Every enabled actionable component has a pointer cursor, visible hover feedback and keyboard focus treatment: rail rows, narrow bottom tabs, buttons/FAB, day controls, menu toggles, selection checkboxes, chips/segments/options/swatches, settings/back/privacy rows, switches, refresh/retry, dialog buttons and broadcast dismiss. Text inputs use an I-beam; selectable receipt/dish text uses text selection, not hand cursor. Passive badges/summary values never imply a click.
- Hover is enabled only with a real hover-capable pointer; no sticky hover after touch on phones. Use a host capability equivalent rather than literal web media queries. Mobile style/density is unchanged.
- Neutral control hover uses `hoverFill`, press uses `pressedFill`. Selected neutral rows keep their selection tint/rule on hover and add a 1 px `focusInk` inset boundary so hover cannot erase selected state. Explicit filled actions use their named action/danger hover/press tokens. Swatches keep their exact colour and add the inset boundary rather than darken the colour being selected.
- Focus uses the named ring in addition to state fill; show for keyboard/programmatic focus, not every mouse click. Input focus is always visible while editing. Focus moves logically and is never hidden under sticky controls, clipped at pane edges or lost on theme changes. A focused selected row shows both selection rule and outer focus ring.
- Disabled controls have no click/keyboard handler, no pointer cursor or hover change, and are omitted from traversal unless they are an explanatory read-only list record. Use `surface_variant`/`disabledInk`, not an opacity reduction on a whole readable record. Busy controls reserve their label width and add a 16 px spinner; they do not jump in size. Full-height empty state is never rendered on top of valid cached rows.
- Information is never hover-only. Tooltip `Aktualisieren` supplements the icon, not status/cutoff/error text. No tooltip is needed to read a full selected dish or receipt. At least 44×44 hit areas are required, even when a checkbox/swatch/icon is drawn smaller; neighbouring hit areas must not overlap.

## Motion

Keep the existing `Neue Menüs verfügbar!` toast's 300 ms slide/fade in, 4000 ms hold and 300 ms out. On wide desktop anchor it 8 px below the workspace top, inset 16 and constrained to workspace width; it does not cover the rail or intercept input. Its arrival is the one expressive motion moment, tied to genuinely new menus.

Desktop selection, hover, focus, tab changes and breakpoint changes are immediate: no animated column resize, bouncing row, crossfade that mixes two receipts or full-window loading shimmer. Dialog opacity fade is 120 ms, no transform. System reduce-motion disables toast movement and modal fade, retaining the message for 4000 ms and dismiss semantics. Spinners remain functional indicators, not extra choreography. Mobile's existing day swipe/toast behaviour is not redesigned here.

## Native macOS host and menu bar

Use the standard decorated/resizable macOS window with normal traffic-light controls and title `SnackPilot`. Default content size **1180×780**, minimum width **360**; clamp first launch to the visible work area. Minimum height **480**; content still scrolls when the OS constrains it further. Remember ordinary window bounds when supported by the host, not navigation state in host-only storage. Keep a single main window; Dock activation after closing it reopens that window rather than launching a second model/session. Do not introduce a custom titlebar, toolbar imitation or preference window.

Native menu names are German; separators below indicate normal system menu separators, not in-app controls. OS menus use macOS rendering and typography, not the Inter recipe.

| Menu | Ordered contents and action |
|---|---|
| **SnackPilot** | `Über SnackPilot` → themed About alert; separator; `Einstellungen …` Cmd+, → settings tab; separator; standard `Dienste` submenu; separator; `SnackPilot ausblenden` Cmd+H, `Andere ausblenden` Option+Cmd+H, `Alle einblenden`; separator; `SnackPilot beenden` Cmd+Q. |
| **Ablage** | `Fenster schließen` Cmd+W. No New/Open/Save for nonexistent documents. |
| **Bearbeiten** | `Widerrufen` Cmd+Z, `Wiederholen` Shift+Cmd+Z; separator; `Ausschneiden` Cmd+X, `Kopieren` Cmd+C, `Einsetzen` Cmd+V, `Alles auswählen` Cmd+A. Enable by text focus/selection; password copy/cut unavailable. |
| **Darstellung** | `Menüs` Cmd+1, `Bestellungen` Cmd+2, `Abrechnung` Cmd+3, `Einstellungen` Cmd+4 (checked active destination); separator; `Aktualisieren` Cmd+R; `Zurück` Esc. Enable only when the corresponding command can act. No user toggle for rail or density. |
| **Fenster** | Standard `Minimieren` Cmd+M, `Zoomen`, separator, `Alle nach vorne bringen`, standard main-window entry. Use native semantics. |

Menus and on-screen controls dispatch identical intents. The global menu is present even below 720 and while no main window is visible; content actions disable when unavailable. A modal disables Settings/tab/refresh/back menu commands that could escape it; Esc is routed to its cancel action. Cmd+Q/H/M remain native. No release-only gallery menu or fake update command. `--gallery`, its section/page arguments and environment/property debug entries keep working and remain excluded from release; a gallery launch is not forced into a production tab.

## Shipping desktop: required amendment to 08-gpui-desktop.md

The following is the design decision for the requirements change, not a claim that packaging or platform support is already implemented. Replace the dev-only Desktop paragraph with a shipping macOS platform contract and keep the architecture/known-regressions/measured-artifacts sections truthful.

1. **Scope:** one gpui-kit Rust UI ships on iOS, Android and macOS. Desktop is no longer only a development/gallery addendum. `make ship` exposes macOS alongside iOS/Android and can select it independently; “all” includes all three. Explicitly supersede the old mobile-only/development-only statements, including the affected cross-references in `00`, `04`, `07` and README. Do not silently rewrite historical v1 descriptions.
2. **Layout:** record the desktop-host-only 720 threshold, 176/192 rail widths, 960 px workspace max, compact/expanded master split, phone fallback, preserved mobile dialects and no 480 cap on wide desktop. Name Desktop Ledger as an extension of §7/§8. Record logical-pixel measurements and native titlebar exclusion.
3. **Interaction:** include the four tab shortcuts, focused-list selection, Esc precedence, dialog Return/Tab trapping, hover/focus requirements, native menu inventory and model-owned stable preview state. Wide details do not push/pop; narrow settings keep the existing back affordance. Record no state reset on resize/resume.
4. **Distribution:** macOS release is an installable **`SnackPilot.app`**, bundled with icon, fonts/assets and the existing `dev.radaiko.gourmetclient` identity. Proposed delivery is a Developer-ID-signed, hardened-runtime, notarized and stapled app in a signed/notarized **`SnackPilot-{version}.dmg`** with a standard app-to-Applications install layout. Signing identity/secrets remain external. `make ship` reports validation failure rather than naming an unsigned debug binary a release. No App Store or auto-update promise is added. Engineering must resolve supported CPU slices and minimum macOS version before promising a distributable release; that is a release constraint, not a visual choice.
5. **Release identity:** common product version and build metadata feed About and rail, and macOS has its own platform artifact/tag alongside the existing mobile artifacts. No silent change to iOS/Android identifiers, signing or release flow. Dry-run must retain its no-version/git/counter-change property and must not silently distribute an artifact.
6. **Durable local state:** shipping desktop must not use the development temp-directory path as its durable cache/settings home. Use macOS application-support storage; credentials remain OS-keychain-backed under the existing service/slots, never in the bundle, DMG, notes or logs. Installation/update must not overwrite the user's state. This is a shipping prerequisite, not a request to change scraping or credential semantics.
7. **Capabilities and honest gaps:** desktop notifications/geofencing remain unsupported in this change; notification settings stay hidden and analytics remains a no-op. Accent selection affects theme, with no claim of dynamic Dock-icon parity. Preserve the accepted GPUI accessibility-tree regression and themed-alert policy explicitly. Keyboard/focus improvements are not evidence of VoiceOver parity. Do not represent no-op mail/settings calls as supported desktop features.
8. **Development:** preserve `--gallery` and existing demo/debug arguments in debug builds. Retain measured artifact sizes as dated historical measurements; collect real desktop release size/startup evidence rather than extrapolating from mobile. `make check` remains the repository gate; packaging/signing and actual macOS UI evidence are additional release evidence, not replacements for it.

## Acceptance and evidence contract for implementation

This is a design handoff, not an implementation/test result. No check, build, shipping artifact or visual PASS is claimed here. The implementation must supply:

- Screenshots of each four-tab root and each reachable detail at **360, 720 and 1280 logical content widths**, with known demo/fixture data and no real credentials. Add **390 and 768** checkpoints for the general UI-review evidence convention and mobile before/after comparisons. Capture compact/expanded light and dark desktop gallery states with hover/focus/selected/pending/disabled examples. Do not scale a phone image and label it desktop evidence.
- A populated menu list with separate preview/pending states; long German dish/allergen text; unavailable/cutoff states; mixed pending submit count; confirmation/cancel/error/progress; upcoming/past orders; batch checks distinct from preview; both billing sources, long receipt lines, negative/large amounts; and every credential/appearance detail. Empty/auth/no-cache-loading/cached-refresh/error states must remain discernible. Explicitly show 720's 263 px detail and long settings titles without clipping.
- Keyboard-only path through all tabs, every visible control, both login forms and the destructive-cancel dialog; focus-trap/restore, no auto-repeat mutation, arrow-scope and disabled-input evidence. Confirm actual Cmd+Q/About/Edit/View menus in a macOS build, not a simulated screenshot.
- Resize evidence across **719 → 720 → 719**, including unsaved input, pending menu changes, receipt/order selection and active modal, plus background/resume preserving navigation/selection while model data refreshes. On mobile, before/after screenshots show no new rail/table dialect and no spacing/colour drift.
- Assertions for pure model selection identity, navigation/breakpoint projection, tab re-tap no-op, Esc precedence, stable refresh selection and existing domain eligibility. UI checks cover geometry/focus/input and calls to existing intents; offline fixtures cover data. No real host traffic for tests. `make check` output and the review verdict belong to the implementation handoff.
- A signed/notarized install-and-relaunch check for the real macOS release artifact, durable settings/keychain continuity, valid identity/version/assets, and retained debug gallery capability in debug builds. Signing success is not inferred from compilation.

## Assumptions and open questions

**Explicit assumptions:** macOS is the only newly shipping desktop OS; native decorated window and one main session; no new notification/geofence/analytics/updater implementation; no runtime dependency required by the visual design; no process-restart restoration beyond existing persistence. Each assumption is independent and must remain visible when `spec` scopes the change. They do not relax request-shape safety, de-AT formatting, persist-before-validate or existing mobile parity semantics.

**Open release questions, not open design choices:** which macOS minimum version and CPU architectures are supported, and which Developer ID/notarization identity is available? Confirm the proposed direct-download DMG channel before `spec` commits packaging details; a Mac App Store decision changes signing/distribution scope. These do not reopen the approved layout/keyboard decisions. There are no unresolved visual taste decisions in this brief. Missing desktop screenshots and inaccessible external skill references are evidence limitations recorded above, not proof of a rendered outcome.
