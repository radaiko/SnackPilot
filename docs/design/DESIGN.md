# SnackPilot design inventory

Status: inventory of what exists and what the **SnackPilot Desktop Ledger**
extension adds. This is a map, not a new design system and not a replacement:
every mobile value below stays exactly as it is, and the desktop extension is
derived from those values. The approved brief is
[`desktop-ui-redesign.md`](desktop-ui-redesign.md); the requirements are
`docs/requirements/04-ui-ux.md` and `08-gpui-desktop.md`.

Implementation homes:

| Layer | File |
| --- | --- |
| Colour token table (plain data, gpui-free) | `src/ui/model/src/tokens.rs` |
| Geometry, contrast and ink maths (gpui-free) | `src/ui/model/src/desktop.rs` |
| Recipes, palette resolution, gpui colours | `src/ui/app/src/theme.rs` |
| Shared components | `src/ui/app/src/components/*` |
| Shell (rail, workspace, overlays) | `src/ui/app/src/app.rs`, `src/ui/app/src/lib.rs` |

## 1. Incumbent tokens (unchanged)

Source: `snackpilot_model::tokens`, resolved per appearance by
`AppTheme::build`. `both(light, dark)` means the token carries an appearance
pair; `fixed` means one value for both.

### Colour

| Group | Tokens |
| --- | --- |
| Structure | `BACKGROUND` (`#F5F5F7`/`#000000`), `SURFACE` (`#fff`/`#1C1C1E`), `SURFACE_VARIANT` (`#EDEDF0`/`#2C2C2E`), `BORDER` (`#D6D6D8`/`#38383A`), `BORDER_INPUT` (`#CECED0`/`#48484A`) |
| Text | `TEXT_PRIMARY` (`#1D1D1F`/`#F5F5F7`), `TEXT_SECONDARY` (`#6E6E73`/`#A1A1A6`), `TEXT_TERTIARY` (`#AEAEB2`/`#636366`) |
| Accent | `PRIMARY` (`#D4501A`/`#FF6B35`), `PRIMARY_DARK` (`#B84415`/`#D4501A`), `PRIMARY_SURFACE` (`#FFF1EB`/`#2A1A10`) |
| Success | `SUCCESS` (`#34A853` fixed), `SUCCESS_SURFACE`, `SUCCESS_TEXT`, `SUCCESS_BORDER` |
| Warning | `WARNING` (`#F5A623` fixed), `WARNING_SURFACE`, `WARNING_TEXT`, `WARNING_BORDER` |
| Error | `ERROR` (`#D93025`/`#EA4335`), `ERROR_SURFACE`, `ERROR_TEXT` |
| Overlay | `OVERLAY` (`rgba(255,255,255,0.7)`/`rgba(0,0,0,0.7)`) |
| iOS glass | `GLASS_SURFACE`, `GLASS_SURFACE_VARIANT`, `GLASS_HIGHLIGHT`, `GLASS_SHADOW_EDGE`, `GLASS_PRIMARY`, `GLASS_SUCCESS`, `GLASS_WARNING`, `GLASS_ERROR`, `BLUR_TINT_*`, `BLUR_INTENSITY*` |
| Android glass fallback | `ANDROID_GLASS_SURFACE`, `ANDROID_GLASS_SURFACE_VARIANT`, `ANDROID_GLASS_HIGHLIGHT`, `ANDROID_GLASS_SHADOW_EDGE`, `AccentColors::android_glass_primary` |

Five accents (`ACCENTS`) each carry a light and a dark `AccentColors` triple;
`AccentColor::System` resolves from the OS appearance. No desktop token
introduces a hue: the desktop palette is a projection of this table.

### Type

Bundled Inter Regular/Semibold/Bold. Mobile sizes are those the screens use
today (page title 28/700, body 14/20, metadata 12/16, tab labels, dialog
17/22); they are deliberately not re-scaled. Desktop sizes are listed in §2.

### Recipes (`AppTheme`)

`card_surface`, `banner_surface`, `tinted_banner(bg)`, `button_primary`,
`button_secondary`, `button_danger`, `input_field`, `circle_button`, `fab`,
`selected_chip`, `unselected_chip`, `divider` — each a `Recipe`
(`bg`, `radius`, optional `(border width, colour)`, optional shadow) applied by
`theme::apply`. The iOS glass dialect and the Android flat dialect are chosen by
`AppTheme::flat`; neither changes with the desktop shell.

## 2. Desktop Ledger extension (new)

All of it is host-gated: only a host that reports `HostKind::Desktop` and a
content width at or above 720 px leaves the phone shell, so iOS and Android
render byte-identically at every width.

### Geometry tokens

`snackpilot_model::desktop` owns the numbers and the breakpoint rule
(`layout_mode`); `desktop_geometry(host, width)` returns the tiles. The shell
never derives a width itself.

| Brief token | Implementation |
| --- | --- |
| `desktop.breakpoint`, `desktop.expandedBreakpoint` | `layout_mode` gates at 720 and 1024, desktop host only |
| `desktop.contentMax` | `CONTENT_MAX = 960` |
| `desktop.railWidthCompact`, `desktop.railWidthExpanded` | `RAIL_WIDTH_COMPACT = 176`, `RAIL_WIDTH_EXPANDED = 192` |
| `desktop.outerCompact`, `desktop.outerExpanded` | `OUTER_COMPACT = 16`, `OUTER_EXPANDED = 24` (gutters are what remains after the 960 cap) |
| `desktop.masterCompact`, `desktop.masterExpandedFraction` | `MASTER_COMPACT = 248`; expanded splits the interior minus the divider 56/44 |
| `desktop.dividerWidth`, `desktop.selectionRule` | `DIVIDER_WIDTH = 1`; selection rule 3 px (`record_table`) |
| `desktop.targetMin` | `TARGET_MIN = 44` |
| phone column | `PHONE_CONTENT_MAX = 480` (below 720 the desktop frame is the centred phone column) |
| contrast floors | `TEXT_MIN_CONTRAST = 4.5`, `CONTROL_MIN_CONTRAST = 3.0` |
| `desktop.actionHoverFill`, `desktop.actionPressedFill` | `ACTION_HOVER_MIX = 0.08`, `ACTION_PRESSED_MIX = 0.14` mixed with `text_primary` |

Remaining brief tokens (`space.1–6`, `brandHeight`, `headerMin`,
`toolbarMin`, `actionStripMin`, `navRowMin`, `navGap`, `navIconGap`,
`tableRowMin`, `tableHeaderHeight`, `tableCellPadX`, `tableCellGap`,
`menuRowMin`, `settingsRowMin`, column widths, radii, type scale, `formMax`,
`focusWidth`/`focusGap`, dialog and toast timings) live as module constants at
the component that paints them — `desktop_navigation.rs` (56/44/4/12/20),
`workspace.rs` (56/56/8/44), `record_table.rs` (36/56/12/12/3). `menuRowMin`,
`settingsRowMin`, the orders/billing column widths and the detail typography
belong to tasks 1.3/1.4, which own those screens; the toast and dialog timings
are the incumbent 300/4000/300 ms and 120 ms fade and are unchanged.

### Colour projection

`AppTheme.desktop` is a `DesktopPalette` built by `build_desktop_palette` from
that same token table (`snackpilot_model::desktop` does the maths, `theme.rs`
only converts to gpui colours):

| Brief token | Implementation |
| --- | --- |
| `desktop.panelFill` | `panel_fill` → `SURFACE` |
| `desktop.railFill` | `rail_fill` → `BACKGROUND` |
| `desktop.focusInk` | `focus_ink` → `TEXT_PRIMARY` |
| `desktop.actionFill` | `action_fill` → `PRIMARY_DARK` in light, `PRIMARY` in dark |
| `desktop.actionHoverFill`, `desktop.actionPressedFill` | `action_hover_fill`, `action_pressed_fill` (8 %/14 % mix) |
| `desktop.dangerFill` (+ hover/pressed) | `danger_fill` → `ERROR`, same mixes |
| `desktop.onAction`, `desktop.onDanger` | `on_fill` → whichever of white/black has the higher contrast on that fill |
| `desktop.readableInk` | `readable_ink(fill, preferred)` |
| `desktop.accentOnPanel`, `desktop.accentOnRail`, `desktop.accentOnSelection` | `readable_ink(panel_fill, accent)`, `readable_ink(rail_fill, accent)`, `readable_ink(selection_fill, accent)` — accent text keeps the accent only where it clears 4.5:1 on the fill it actually sits on |
| `desktop.accentBoundary` | `boundary_ink(selection_fill, accent)` — the selected card's 1 px edge keeps the accent only where it clears 3:1, else `text_primary` |
| `desktop.hoverFill`, `desktop.pressedFill`, `desktop.selectionFill`, `desktop.selectionInk`, `desktop.secondaryInk`, `desktop.borderInk` | incumbent `surface_variant`, `border`, `primary_surface`, `text_primary`, `text_secondary`, `border` |

`DesktopAction { fill, hover, pressed, ink }` is the resolved pair the buttons
consume. Contrast is proven in the model
(`desktop_contrast_all_accents`): 4.5:1 for text and 3:1 for control and focus
boundaries across all five accents, both appearances, at rest, hover and press,
including accent text resolved against the panel, rail and selection fills.

### Recipes and components

| Brief name | Implementation | Contract |
| --- | --- | --- |
| `desktop.sidebarSurface` / `DesktopNavigationRail` | `components/desktop_navigation.rs::desktop_rail` | `railFill`, 56 px brand row, four 44 px rows (20 px icon, 4 px gap, 12 px inset), hover `surface_variant`, active `desktop.accentOnRail` (accent resolved to >=4.5:1, else `text_primary`), inactive text and icons `text_secondary`, version footer that is not a control, scrolls at short heights |
| `desktop.navigationRow` | same module | active row doubles as the selection; hover `surface_variant`; focus ring independent of selection; one tab stop on the rail with a roving Up/Down/Home/End cursor, Enter/Space activates; a changed destination moves focus, an active re-selection is a no-op |
| `desktop.panelSurface` / `SplitWorkspace` | `theme.workspace_panel()` + `components/workspace.rs::split_workspace` | `panelFill`, radius 12, 1 px border, no shadow; 1 px divider; both panes scroll independently |
| `WorkspaceHeader` | `components/workspace.rs::workspace_header` | min 56 px, tab title, trailing 44×44 `refresh-cw` with tooltip `Aktualisieren` for Menüs/Bestellungen/Abrechnung, none for Einstellungen |
| `desktop.actionStrip` | `components/workspace.rs::action_strip` | min 56 px, 8 px gap, wraps, 1 px top border |
| `desktop.detailSurface` empty state | `components/workspace.rs::detail_prompt` | centred prompt at 16 px inset |
| `desktop.tableHeader`, `desktop.tableRow` / `RecordTable` | `components/record_table.rs` | header 36 px on `background`; rows `surface`, min 56, 1 px bottom border, no zebra/shadow; hover `surface_variant`; selected `primary_surface` + always-present 3 px inset rule so width never changes; `record_cell` is read-only (no pointer cursor), the row is the single target |
| `desktop.buttonPrimary/Danger/Secondary` | `components/button.rs::desktop_action` | min 44, radius 8, paddingX 16, no shadow, derived fills and readable ink; secondary transparent + 1 px border + `surface_variant` hover |
| `desktop.menuRow`, detail panes, order/receipt tables | tasks 1.3/1.4 | reuse these primitives; no second screen implementation |
| `desktop.settingsRow` | `components/settings_row.rs::settings_row_desktop` | 72 px min, 12 px padding, title 14/20 Semibold, hint 12/16, whole row a target; selection tint + 3 px rule; ring independent of selection; Enter/Space opens |
| focus ring | `theme.rs::focus_frame` / `desktop_focus` | 2 px `focusInk` + 2 px container gap, reserved 4 px on every desktop interactive surface including legacy narrow controls; mobile keeps its own focus defaults |
| native menus | `desktop/macos.rs` | the approved German SnackPilot/Ablage/Bearbeiten/Darstellung/Fenster inventory; checked active destination; content items disable with no window/while modal; native Quit/Hide/Close/Minimise/Zoom; About uses real metadata |
| desktop dialog | `components/dialog.rs` | one resolution per open dialog; initial focus OK/confirm; Tab/Shift+Tab and Left/Right cycle inside the action row; Enter/Space activate the focused action; Esc/backdrop resolve index 0; repeats ignored; focus restored to the origin |

Incumbent components keep their mobile contracts: `tab_bar` (bottom bar and
iOS pill), `card`, `chip`, `text_field`, `dialog`, `toast` (300/4000/300 ms,
120 ms fade), `loading_overlay`, `day_navigator`, `menu_card`, `order_item`,
`bill_card`, `segment_tabs`, `settings_row`, `spinner`, `fab`, `swipe`,
`pull_refresh`, `banner`, `broadcast_banner`, `time_chips`, `switch`. The
desktop shell reuses the tab icons (`tab_bar::tab_icon`) rather than
introducing a second icon map, and paints overlays above the whole shell.

### State ownership

Layout mode, tile widths, the active tab, the orders segment, settings-detail
kind and every list preview/cursor are model state
(`snackpilot_model::desktop`, `state.rs`). The shell keeps only view resources:
the push stack mirror, input entities, swipe gesture, dialog/toast and the
debug-gallery knobs. Resize reports `host_kind` and `content_width`; crossing
720 changes presentation, never domain state.

### Operation dispatch

Blocking flows are dispatched at the shared intent boundary
(`app::RefreshIntent`, `app::run_blocking`): the same model methods run, on the
background executor for every desktop host — the rail is irrelevant, so a
narrow desktop window that projects as the phone shell dispatches off-thread
too — while cached content keeps painting. Pointer, shortcut/menu and
foreground paths all use it: workspace-header refresh, credential save,
Menüs submit, Bestellungen confirm/cancel (single and batch), billing reload,
month/source selection and the resume refresh. A phone or tablet host keeps
the incumbent synchronous path, no extra request is made, and a repeat
dispatch is rejected by the model's claim or the shell's mutation gate, never
by a disabled visual alone. The rail footer and About use `desktop::product_version()`/`product_build()`:
the common release metadata when the packaging step supplies
`SNACKPILOT_PRODUCT_VERSION`/`SNACKPILOT_BUILD`, otherwise explicit debug
metadata (`Entwicklung`/`debug`). The app crate's placeholder `0.0.0` is never
shown. Task 1.6 injects the real release values.

### Deferred desktop shell wiring

The desktop-only compositions `workspace::split_workspace`,
`record_table::record_row`/`record_header` and `workspace::action_strip` exist
and are exercised by the gallery, but the shared root still renders the
incumbent screen bodies until the screens own their master/detail split. That
wiring was delivered in **task 1.3** (Menüs, the first real master/detail and
action strip), **task 1.4** (Bestellungen and Abrechnung, which consume
`record_row`) and **task 1.5** (settings containment, the roving record-list
focus and the desktop dialog/keyboard/menu layer above the whole shell). Task
1.2 wired the desktop loading and toast overlay variants for every non-phone
layout; no second screen implementation is introduced.

### Settings containment and the single narrow push

The model owns the projection (`AppModel::settings_contained`): containment
starts at the desktop compact breakpoint, so both compact (720–1023) and
expanded desktop keep the one `settings_detail` identity beside the master and
push nothing, while the phone projection (a phone host, or a desktop below 720)
pushes exactly one `Screen::SettingsDetail`. Crossing 719 → 720 → 719 therefore
keeps that one push slot from ever doubling: at 719 the open detail is the
pushed screen, at 720 the push is dropped into containment, and back at 719 it
is pushed again once. The one `InputState` pair created on entry stays mounted,
its unsaved text survives, and the service binding is still the same detail
kind. The shell asks the model instead of deriving a layout rule of its own, so
the rendered containment and the model's navigation stack cannot disagree.

### Interaction inventory (task 1.5)

- **Commands:** Cmd+1–4, Cmd+comma and Cmd+R on macOS; Ctrl equivalents on
  Windows. The policy is the model's (`desktop::command_allowed`): the
  navigation chords work while a text field has focus and only a modal takes
  them away, Cmd+R and the other flow commands stay outside the TextInput
  scope, and native editing keys are never captured. A repeat or the other
  platform's modifier is rejected before the model's guarded command runs.
  Cmd+R is enabled only on Menüs/Bestellungen/Abrechnung while no flow is busy
  or claimed. Only macOS registers the Bearbeiten key equivalents, because only
  macOS has that menu bar.
- **Esc precedence:** dialog → batch selection → preview/settings detail →
  nothing. It never clears pending menu changes and never quits.
- **Focus:** the rail is a roving group (Up/Down/Home/End, Enter/Space
  activates, an active re-selection is a no-op). The settings master, the two
  appearance groups, the Bestellungen segment group and each wide record list
  are one tab stop each. The settings master and the rail move with
  Up/Down/Home/End, appearance Design and Akzentfarbe and the segment group
  with Left/Right; the Menüs, Bestellungen and Abrechnung tables keep per-row
  focus handles driven by Up/Down/Home/End (`record_table::RecordListFocus`),
  which preview the record and scroll it into view without wrapping. Enter on
  a record row hands focus to the detail: the preview's first enabled action
  when it has one, otherwise the detail's read-only scroll region
  (`record_table::detail_entry`). Every
  option group selects the neighbour immediately; credential Return submits
  Save once (guarded by the shell mutation gate). Arrows are never bound
  globally, so text editing and read-only receipt scrolling keep their native
  meaning. Tab/Shift+Tab never leaves a modal.
- **Dialogs:** one resolution object per dialog; the opening key's auto-repeat
  cannot confirm; pointer, Enter/Space, Esc and backdrop share one dispatch;
  focus is captured on open and restored on close. When the confirmed flow
  deleted the record whose control held focus (a cancelled order), its handle
  has no element left, so the wide workspace restores the nearest surviving
  row (the model's reconciled cursor) and falls back to the screen heading only
  when no row survives; the narrow shell keeps the incumbent exact restore.
- **Native Mac host:** standard decorated window titled `SnackPilot`, default
  1180×780 (clamped to the visible work area), minimum 360×480; Dock reopen
  reuses the one installed model and reopens one window. `desktop::application_hook`
  registers that rule on the application and `src/bin/desktop.rs` applies it
  before `run`. The Bearbeiten items keep their native OS selectors and carry
  the brief's key equivalents (⌘Z/⇧⌘Z/⌘X/⌘C/⌘V/⌘A) through app key bindings;
  the app installs no handler for them, so text editing keeps its own bindings.
- **Privacy/About:** the human-approved Aptabase Datenschutz paragraph
  (`07-release.md` is authoritative, not the historical TelemetryDeck text)
  plus the desktop-only no-statistics sentence; About shows `SnackPilot`,
  `Version {version} ({build})`, the product line and the GourmetClient
  credit.
- **Evidence harness:** a debug-only `--evidence` / `SNACKPILOT_EVIDENCE=1`
  run stores its cache in an `evidence` subdirectory, installs an in-memory
  dummy credential pair and switches the model to isolated mode before
  `bootstrap`, so the launch order skips the live broadcast, the permission
  prompt and the geofence restore. The normal credential slots are never read
  or written, and a release build cannot reach any of it.
- **Capabilities:** notifications/geofence remain unsupported on desktop; the
  settings row is hidden at every width and a stale/debug route shows only the
  truthful unavailable sentence.

## 3. Mobile Day Tray extension (approved 2026-11-13)

Status: an **approved extension**, not a replacement system. Section 1 (the
incumbent token inventory) and section 2 (the Desktop Ledger) are unchanged and
stay pinned; this section only records what the compact presentation adds and
which shared components task 1.2 moved onto it. The approved brief is
[`mobile-ui-rework.md`](mobile-ui-rework.md); the requirements are
`docs/requirements/04-ui-ux.md` plus this change's `REQ-UI-15` and `REQ-UI-16`.

The extension is a **presentation projection**, never `theme.desktop`: the wide
Desktop Ledger keeps the recipes and palette of section 2, and no base colour,
radius, type size or public component signature changes.

### Extension tokens

| Extension token | Implementation |
| --- | --- |
| `mobile.space.1/2/3/4/5/6/8` | `tokens::mobile::SPACE_*` (4/8/12/16/20/24/32), exposed as `AppTheme::mobile.space_*` |
| `mobile.contentMax`, `mobile.formMax` | `CONTENT_MAX = 560`, `FORM_MAX = 440` |
| `mobile.target`, `mobile.actionHeight` | `TARGET = 44`, `ACTION_HEIGHT = 48` |
| `mobile.type.headline/body/meta` | `TYPE_*` 24/32, 16/24, 14/20 (bundled Inter; the 12 px Android tab labels stay the one exception) |
| `mobile.radius.option/status` | `RADIUS_OPTION = 8`, `RADIUS_STATUS = 8` |
| `mobile.focusWidth/focusGap/focusInk` | `FOCUS_WIDTH = 2`, `FOCUS_GAP = 2`, `focus_ink` = pinned `TEXT_PRIMARY` |
| `mobile.secondaryInk(fill)` | `secondary_ink(fill, dark)`: pinned `TEXT_SECONDARY` at >=4.5:1 on the painted fill, else `TEXT_PRIMARY` |
| `mobile.statusInk(fill, preferred)` | `status_ink(fill, preferred, dark)`: the existing semantic ink at >=4.5:1, else `TEXT_PRIMARY`; no new hue |
| `mobile.selectionFill/Ink/Boundary` | `selection_fill` (iOS `GLASS_PRIMARY`, Android the accent's `PRIMARY_SURFACE`, composited over the card), `selection_ink` = `TEXT_PRIMARY`, `selection_boundary` = accent `PRIMARY` at >=3:1 on fill and neighbour, else `TEXT_PRIMARY` |
| `mobile.actionFill/Ink` | `action_fill` = accent `PRIMARY_DARK` light / `PRIMARY` dark; `action_ink` = whichever of `SURFACE.light`/`BACKGROUND.dark` reads better |
| `mobile.dangerInk(fill)` | `danger_ink(fill, dark)`: `ERROR_TEXT` at >=4.5:1, else `TEXT_PRIMARY` |
| `mobile.pressedFill(fill)` | `pressed_fill` = 14 % `TEXT_PRIMARY` mixed into the resting fill; fill-only, no scale/bounce |
| `mobile.disabledFill/Ink` | `SURFACE_VARIANT` and its readable secondary projection; never a whole-content opacity |

Geometry is a pure host/width projection: `mobile::geometry(host, width)` is
`min(viewport - 32, 560)` with the 440 px form cap, `Some` for a phone/tablet
host at any width and for a desktop host below the model's 720 px breakpoint,
`None` for the wide Ledger. `lib.rs::compact_width` uses the host-reported width
when there is one and the live viewport otherwise; `AppShell` centres the column
itself, so the old 480 px narrow-desktop cap is gone.

### Shell

- One heading in the active tab's name at 24/32 Bold, 16 px below the safe-area
  top and 16 px above the screen controls (`app::shell_heading`,
  `shell_heading_row`). A pushed settings screen draws its own 48 px back row and
  title, so the tab heading is not repeated there.
- The safe-area top is applied once, outside the centred column; the column is
  `min(viewport - 32, 560)` (440 on a credential form).
- The bottom reservation is measured (`app::obstruction_reservation`): the
  navigation height only when it is painted over the content (the iOS pill),
  plus a present dock's measured height, plus the 16 px scroll-end gap. A bar in
  normal flow already owns its height and is never counted twice; the shell adds
  no blind 100 px padding.
- iOS keeps its 248x56 r28 icons-only pill with the active dot, Android its 64 px
  labelled bar with the system inset reserved below it; no mobile rail appears at
  768 or 1280 and pushed screens keep the existing navigation topology.

### Components moved onto the projection (task 1.2 only)

| Component | Compact presentation |
| --- | --- |
| `card` | 16 px padding, 8 px gap, pinned `card_surface` radii (16 iOS/14 Android); body/meta roles and readable ink |
| `chip` / `chip_row` | 44 px option recipe, r8, accent tint + 20 px `Check` + 1 px resolved boundary when selected, readable unselected ink, 8 px gap |
| `button` (`mobile_action`) | 48 px high, primary on `actionFill`/`actionInk`, destructive on the pinned `ERROR` fill, neutral outlined secondary, 14/12 radii, no coloured shadow; disabled swaps to `disabledFill`/`disabledInk` and installs no callback; press is fill-only and cancels on outside release or scroll claim |
| `text_field` | 48 px minimum, r12, 16/12 inner padding, `input_field` fill and `borderInput` boundary; one input entity |
| `banner` | named spacing, meta role, readable neutral/semantic ink on the painted fill; `Aktualisiere...` is a neutral 16 px spinner row |
| `broadcast_banner` | warning tint with readable status ink; dismissal is a real 44x44 target with the Lucide `X` at 20 px |
| `dialog` | 44 px actions with the compact action/neutral recipes, r8; the body is the flexible scroll region so the action row stays visible; index 0 stays cancel on backdrop/Esc with one resolution |
| `toast` | approved C1 copy `Neue Menüs verfügbar`, 300/4000/300 ms unchanged, reduced motion drops the travel (0 px) without changing the lifetime |
| `switch` | 44 px row target, body-role label, accent track when on |
| `time_chips` | 44 px tall r8 options, 12 px padding/8 px gap, selected tint + boundary + 20 px `Check`, horizontal strip unchanged |
| `tab_bar` | incumbent pill/bar geometry; inactive destinations use the readable secondary projection on the bar fill instead of tertiary; 12 px Android labels retained |
| `navigation` (back) | 48 px minimum row, `ChevronLeft` 24, primary ink, no blind 100 px padding |

### Menüs (task 1.3)

| Component | Compact presentation |
| --- | --- |
| `day_navigator` (`mobile_day_navigator`) | 48×48 arrows with 20 px Lucide `ChevronLeft`/`ChevronRight`, disabled ends use the readable disabled ink instead of 0.3 opacity; min-48 centre target with the date in the headline role and the `{i} / {n}` ordinal in the metadata role, 4 px gap; today/short-format/edge semantics unchanged. The wide Ledger keeps the incumbent toolbar. |
| `menu_card` | 16 px padding, 8 px gaps, 8 px after the card, card radii 16/14; the dish is the `subtitle` (title only when empty) in the body role up to four lines under a neutral body Semibold category heading 8 px above the group (24 px after it); badge in the metadata role on its semantic surface at r8/8x4 with the precedence cancellation → ordered → unavailable → cutoff; pending selection is the accent tint + 1 px resolved boundary + inline 24 px `Check` slot and 8 px gap (never a floated check); pending-cancel is dashed and struck on readable ink, not whole-card opacity; allergens wrap above the unchanged right-aligned price; no state dims readable dish text; eligibility unchanged. |
| `fab` (`mobile_dock`) | Replaces the absolute FAB column: full column minus the common gutters, min 48, r14 iOS/r12 Android, 16/12 padding, centred wrapping body text on `actionFill`/`actionInk`; exactly `model.fab_label()` while changes wait; during an operation the same band shows the existing exact phase with a 16 px spinner and no callback; painted 8 px above platform navigation inside the shell's measured reservation. |
| `swipe` | Thresholds unchanged (claim 10, switch 50, edge damping 0.3); reduced motion suppresses the painted travel only, never the day decision or its events. |

The Menüs screen drops its legacy `.p(16)`/`.pb(100)`: the shell's centred
column supplies the 16 px gutter and its measured reservation (navigation +
present dock + 16) supplies the scroll-end clearance. `app::navigation_top`
gives the dock's 8 px separation on both navigation dialects.

`loading_overlay` keeps its initial 36 px spinner card. Screen bodies, the
remaining screen contracts and their dock/navigator/card projections belong to
tasks 1.4-1.11; the gallery gains one component **state board**
(`SNACKPILOT_GALLERY_PAGE=mobile`) for the five accents in light/dark on the
host dialect plus the Menüs cards/dock board, and is not proof of a working
screen.

### Not changed here

Base palette values, accent values, `AppTheme`'s incumbent recipes, the Desktop
Ledger geometry/palette/components, host capabilities (native editing, keyboard,
off-thread dispatch, desktop notification hiding), all German copy except the
approved C1 in-app toast and C2 log hint, formatter output, and every motion
constant except the reduced-motion removal of toast travel.
