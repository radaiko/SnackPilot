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

Desktop refreshes are dispatched at the shared intent boundary
(`app::RefreshIntent`): the same model methods run, on the background executor
for every desktop layout (compact and expanded), so cached content keeps
painting while the synchronous flow runs. No extra request, no mobile change
(mobile stays synchronous), and a repeat dispatch is rejected by the model's
claim, not by a disabled visual. Only the workspace-header refresh goes
off-thread; the Menüs and Bestellungen mutations stay synchronous and belong to
tasks 1.3/1.4. The rail footer and About use `desktop::product_version()`/`product_build()`:
the common release metadata when the packaging step supplies
`SNACKPILOT_PRODUCT_VERSION`/`SNACKPILOT_BUILD`, otherwise explicit debug
metadata (`Entwicklung`/`debug`). The app crate's placeholder `0.0.0` is never
shown. Task 1.6 injects the real release values.

### Deferred desktop shell wiring

The desktop-only compositions `workspace::split_workspace`,
`record_table::record_row`/`record_header` and `workspace::action_strip` exist
and are exercised by the gallery, but the shared root still renders the
incumbent screen bodies until the screens own their master/detail split. That
wiring is deliberately **deferred to task 1.3** (Menüs, which introduces the
first real master/detail and action strip) and **task 1.4** (Bestellungen and
Abrechnung, which consume `record_row`). Task 1.5 completes the wiring: every non-phone layout
renders the settings master/detail containment (from the 720 compact
breakpoint), the same credential/appearance/notifications content is reused
beside the master, and the desktop dialog/keyboard/menu layer sits above the
whole shell. Task 1.2 wired the desktop loading and toast overlay variants for
every non-phone layout; no second screen implementation is introduced.

### Settings containment and the single narrow push

Containment at 720 is **app-level presentation**, not model state: the model
keeps one `settings_detail` identity and its single narrow push, and the shell
decides per frame whether that detail is the pushed screen (below 720) or the
right-hand pane of the split (720 and above). That is why 719 → 720 → 719
never re-enters the form: no model transition happens, the one `InputState`
pair created on entry stays mounted and its unsaved text survives, and the
service binding is still the same detail kind. A model-side "wide" push slot
would be the alternative, but `src/ui/model/src` is outside task 1.5's file
allowlist and adding one would duplicate the identity the model already owns.

### Interaction inventory (task 1.5)

- **Commands:** Cmd+1–4, Cmd+comma and Cmd+R on macOS; Ctrl equivalents on
  Windows. A repeat or the other platform's modifier is rejected before the
  model's guarded command runs. Cmd+R is enabled only on Menüs/Bestellungen/
  Abrechnung while no flow is busy or claimed.
- **Esc precedence:** dialog → batch selection → preview/settings detail →
  nothing. It never clears pending menu changes and never quits.
- **Focus:** rail is one stop with a roving cursor; settings master roving
  Up/Down/Home/End; appearance Design and Akzentfarbe are one stop each with
  Left/Right/Home/End selecting the neighbouring option without wrapping;
  credential Return submits Save once (guarded by the shell mutation gate).
  Arrows are never bound globally, so text editing and read-only receipt
  scrolling keep their native meaning. Tab/Shift+Tab never leaves a modal.
- **Dialogs:** one resolution object per dialog; the opening key's auto-repeat
  cannot confirm; pointer, Enter/Space, Esc and backdrop share one dispatch;
  focus is captured on open and restored on close. When the confirmed flow
  deleted the record whose control held focus (a cancelled order), its handle
  has no element left, so the wide workspace restores the screen heading
  instead; the narrow shell keeps the incumbent exact restore.
- **Native Mac host:** standard decorated window titled `SnackPilot`, default
  1180×780 (clamped to the visible work area), minimum 360×480; Dock reopen
  reuses the one installed model and reopens one window. `desktop::application_hook`
  registers that rule on the application and `src/bin/desktop.rs` applies it
  before `run`. The Bearbeiten items keep their native OS selectors and carry
  the brief's key equivalents (⌘Z/⇧⌘Z/⌘X/⌘C/⌘V/⌘A) through app key bindings;
  the app installs no handler for them, so text editing keeps its own bindings.
- **Privacy/About:** the incumbent Datenschutz paragraph plus the desktop-only
  no-statistics sentence; About shows `SnackPilot`, `Version {version}
  ({build})`, the product line and the GourmetClient credit.
- **Capabilities:** notifications/geofence remain unsupported on desktop; the
  settings row is hidden at every width and a stale/debug route shows only the
  truthful unavailable sentence.
