# UI shell delta

## MODIFIED Requirements

### Requirement: REQ-UI-04 Four-tab navigation
The app SHALL present Menüs, Bestellungen, Abrechnung and Einstellungen with the icons of `docs/requirements/04-ui-ux.md` §2. iOS SHALL retain its icon-only floating pill and Android its labelled bottom bar. Desktop below 720 logical content px SHALL retain the labelled bottom bar; at 720 and above it SHALL use the labelled Desktop Ledger rail with the same destinations and intents.

#### Scenario: Tab switch
- **WHEN** the user activates Bestellungen by tab, rail, shortcut or macOS menu
- **THEN** the Orders screen is shown and the destination is marked active in the current host/layout style
- **AND** changing destination resets the departing preview/cursor/settings detail and push stack, not pending menu changes

#### Scenario: Re-tapping the active tab
- **WHEN** the user activates the already active destination
- **THEN** nothing navigates and preview, pending changes, scroll and form input are not reset

#### Scenario: Mobile never gets the desktop rail
- **WHEN** iOS or Android is rendered at 360, 720 or 1280 logical px
- **THEN** it retains its current mobile shell, safe-area rules, density and navigation rather than selecting desktop presentation from width alone

### Requirement: REQ-UI-07 Desktop shell
macOS and Windows SHALL render the shared UI in a native decorated, resizable window, minimum content size 360×480, default 1180×780 clamped to the work area. Desktop SHALL use the phone layout below 720 logical content px and Desktop Ledger at 720 and above, follow system appearance, and keep one main session. Linux remains compile-only where supported by the pinned stack.

#### Scenario: macOS window
- **WHEN** the desktop binary is started on the Mac with demo data
- **THEN** a decorated resizable window titled `SnackPilot` shows the shared UI with native window controls

#### Scenario: Wide window
- **WHEN** desktop content width is 720–1023 logical px, excluding native window/menu chrome
- **THEN** a 176 px labelled rail replaces the bottom bar and the workspace has 16 px outer gutters, a 248 px master, 1 px divider and remaining-width detail
- **AND** at width 720 the workspace is 512 px with a 263 px detail; at 768 it is 560 px with a 311 px detail

#### Scenario: Expanded desktop
- **WHEN** desktop content width is at least 1024 logical px
- **THEN** the rail is 192 px, outer gutters at least 24 px, and workspace max 960 px centred right of the rail, with interior width split 56% master / 44% detail excluding the 1 px divider
- **AND** at width 1280 the workspace gutters are 64 px each and the obsolete 480 px cap does not constrain it

#### Scenario: Phone fallback and short windows
- **WHEN** desktop content width falls below 720 or height is constrained
- **THEN** the narrow desktop uses current flat phone content centred up to 480 px, labelled 64 px bottom bar and no mobile safe-area strip
- **AND** short-window content, rail and modal messages scroll without hiding navigation or modal actions beyond reach

#### Scenario: Native window lifecycle
- **WHEN** the main Mac window is closed and the Dock activates the app again
- **THEN** the same session reopens one main window, without a second model/bootstrap; normal window bounds are remembered when host support allows
- **AND** closing or quitting is not logout or credential deletion

## ADDED Requirements

### Requirement: REQ-UI-10 Stable workspace selection
Navigation, settings-detail kind, filters, orders segment, stable record preview IDs, list cursors and explicit no-preview state SHALL belong to the gpui-free model. Preview SHALL be distinct from pending menu and batch-order selection. Resize/resume SHALL preserve valid in-memory state without domain actions. GPUI focus/input objects SHALL remain view resources, not duplicate navigation state.

#### Scenario: First selection and stable identity
- **WHEN** a wide record list is first entered, then a row is clicked or arrowed to
- **THEN** it initially shows the screen's no-selection prompt, then immediately previews the selected record without push/pop, fetch or mutation
- **AND** menus use `{id}|{day}`, orders `position_id`, receipts the existing `g-…`/`v-…` IDs, never persisted row indexes; hover alone never selects

#### Scenario: Reconcile refresh and filters
- **WHEN** refreshed data replaces a visible list
- **THEN** a surviving preview ID remains selected; a removed one clears preview and restores the cursor/focus to the nearest surviving row without acting on it
- **AND** day/month/source/segment changes clear that list's preview; pending menu changes survive, and billing context changes reset master scroll

#### Scenario: Crossing the breakpoint
- **WHEN** a desktop window moves through 719 → 720 → 719 with a selected settings form, unsaved input, record preview or pending changes
- **THEN** only containment changes: narrow settings pushes the same detail, expansion exposes it alongside its master, and input objects/text survive without duplicate pushes
- **AND** menus/orders/billing use current inline narrow cards without a new detail route; valid selected identities return on expansion, with no login/save/order/cancel/refetch caused by resizing

#### Scenario: Foreground preservation
- **WHEN** the app backgrounds and resumes while its data refreshes
- **THEN** active destination, filters, pending changes and valid preview identities remain in memory; only disappeared data invalidates a preview, and no disk restoration of navigation is implied

### Requirement: REQ-UI-11 Desktop keyboard command routing
Desktop SHALL support the approved focused keyboard map at every width: Cmd+1–4, Cmd+comma and Cmd+R on macOS; Ctrl equivalents on Windows. Esc SHALL cancel dialog, then exit batch selection, then close preview/settings detail, then do nothing. Arrows SHALL act only in the focused group. Mutation keys SHALL ignore repeat, respect eligibility/busy guards and never bypass explicit actions.

#### Scenario: Navigation and refresh
- **WHEN** a tab/settings/refresh shortcut is pressed without a modal
- **THEN** it uses the same intent as the matching on-screen control; a changed tab focuses its heading and active-tab reselection does nothing
- **AND** refresh acts once only on Menüs/Bestellungen/Abrechnung when not already refreshing or mutating, using existing refresh pipelines

#### Scenario: List and option traversal
- **WHEN** the rail/list/option group has focus
- **THEN** Tab exits the group, arrows move without wrapping and reveal the focused item; Home/End go first/last in lists and appearance groups
- **AND** rail Up/Down only moves focus until Enter/Space; wide-list Up/Down previews, from no cursor Down starts first and Up last; Enter focuses detail's first enabled control or read-only scroll region
- **AND** narrow lists keep current card semantics with no new detail route; Space toggles a focused order's batch check only in batch mode; day/option arrows belong only to their focused group

#### Scenario: Inputs and scope
- **WHEN** the user edits a credential field or selects readable receipt/dish text
- **THEN** native text editing, selection, clipboard and undo/redo take precedence over list/day commands; masked password copy/cut is disabled and Return in an enabled credential field submits Save once
- **AND** Windows keeps Alt+F4 and OS-reserved keys native, macOS keeps its standard quit/hide/minimise/close shortcuts, and no Delete key or list activation directly cancels an order

#### Scenario: Esc does not discard pending meals
- **WHEN** Esc successively closes a modal, exits batch mode and closes a preview
- **THEN** each key affects only the highest-precedence active layer, never submits/clears menu pending changes, never quits, and retains list cursor/focus after closing preview

### Requirement: REQ-UI-12 Desktop interaction feedback
All enabled desktop actions SHALL have pointer, hover, press and visible keyboard-focus states, including narrow legacy controls. Inputs SHALL use I-beam and passive data SHALL NOT appear actionable. Focus SHALL use the brief's independent 2 px ring/2 px gap without clipping or layout shift. Disabled controls SHALL have no activation/hover/pointer behaviour and SHALL be skipped in traversal.

#### Scenario: Complete interactive inventory
- **WHEN** desktop buttons, rail/bottom tabs, day controls, toggles, checks, filters, swatches, settings/back/privacy rows, refresh/retry, dialog actions or broadcast dismiss are hovered or keyboard-focused
- **THEN** each shows the corresponding Desktop Ledger state, with >=44×44 non-overlapping targets, and focus remains visible when selected or after a theme change
- **AND** text/amounts/badges do not get a hand cursor and essential status, error or cutoff information is not hover-only

#### Scenario: Disabled and busy
- **WHEN** an action is disabled or busy
- **THEN** it cannot dispatch through pointer, keyboard or menus, has no enabled hover feedback, and busy indication reserves label width with a spinner rather than changing geometry
- **AND** hover never sticks after mobile touch or changes existing mobile styles

### Requirement: REQ-UI-13 Desktop Ledger visual system
Desktop SHALL extend the incumbent theme with the exact Desktop Ledger tokens, recipes, typography and geometry of the approved brief, not a second design system. Desktop text SHALL meet 4.5:1 contrast, focus/control boundaries 3:1, across all five accents and both appearances. Mobile tokens SHALL stay unchanged. Selection/hover/resize SHALL be immediate; reduced motion SHALL suppress authored movement/fades.

#### Scenario: Desktop visual recipes
- **WHEN** a wide workspace renders
- **THEN** it uses opaque Inter/Lucide surfaces, one bordered radius-12 split panel without shadow, independent pane scrolling, fixed/wrapping headers/toolbars/actions and the inset 3 px accent selection rule
- **AND** body/table text is 14/20, full dish text 16/24, amounts trailing/tabular, long prose limited to 60 characters per line, with no gradients, fake metrics or decorative chrome

#### Scenario: Accent contrast and motion
- **WHEN** any light/dark and accent combination is selected or reduced motion is enabled
- **THEN** desktop readable ink resolves against actual fills, filled-action ink independently resolves to the higher-contrast black/white at rest/hover/press, and mobile colour values do not change
- **AND** the existing toast keeps 300/4000/300 ms timing anchored to the workspace, dialog fade is 120 ms, and reduced motion removes toast movement/modal fade without removing the message or dismiss semantics

### Requirement: REQ-UI-14 Native macOS menus and isolated dialogs
macOS SHALL expose the approved German SnackPilot, Ablage, Bearbeiten, Darstellung and Fenster menus at every width, dispatching the same enabled intents as the UI. About SHALL use real version/build and retained GourmetClient credit. Themed dialogs SHALL isolate input, trap/restore focus and resolve once; Enter activates the focused button and Esc/backdrop resolves index 0, never confirms.

#### Scenario: Native menu inventory
- **WHEN** the macOS app launches or has no visible main window
- **THEN** the standard menu inventory/order/shortcuts in the brief remains native, including About, Settings, Services, Hide/Quit, Close, contextual Edit, checked four destinations, Refresh/Back, Minimise/Zoom and Bring All to Front
- **AND** content actions disable when unavailable or blocked by a modal; quit/hide/minimise retain native semantics; no fake updater or gallery menu appears in release

#### Scenario: About and privacy
- **WHEN** About or Datenschutz is opened on macOS
- **THEN** the themed About alert uses the exact approved German copy, real version/build and `github.com/patrickl92/GourmetClient`, while Datenschutz retains the incumbent paragraph plus `Auf dem Mac werden derzeit keine Nutzungsstatistiken gesendet.`
- **AND** Windows privacy uses the equivalent truthful desktop-only sentence `Unter Windows werden derzeit keine Nutzungsstatistiken gesendet.` without altering mobile text

#### Scenario: Modal keyboard and occlusion
- **WHEN** an alert or confirm is shown at narrow or wide desktop width
- **THEN** it centres over the entire window, occludes rail/workspace, initially focuses OK/confirm, traps Tab/Shift+Tab, supports Left/Right action traversal and Enter/Space activation
- **AND** backdrop/Esc resolves cancel/index 0, body click does not dismiss, opener key/repeat cannot confirm, one dispatch disables mutation immediately, and no key/click reaches the background
- **AND** dismissal restores the originating control, nearest surviving row if removed, otherwise screen heading; long messages scroll independently of visible actions
