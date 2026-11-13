# SnackPilot mobile UI rework — Day Tray

## Status

**Design proposal; human approval required before specification or implementation.** Mode: **Operate**. This is a mobile presentation redesign, not an incidental refinement. The design step is needed: the incumbent inventory explicitly preserves mobile values, including the hierarchy and contrast problems addressed here. Extend that inventory as **SnackPilot Mobile Day Tray**; do not replace the brand or Desktop Ledger. No new standalone screen, navigation destination, dependency or design system is proposed. A pending-action dock is a new composition of existing primitives, not a checkout flow.

Written on `spec/mobile-ui-rework` (confirmed from `.git/HEAD`). Only this brief is authored. `spec` owns tasks, approved requirement amendments and implementation scope. Nothing here grants permission to edit protected requirements, source files or desktop contracts.

Read: `AGENTS.md`, README, `00-overview.md`, `04-ui-ux.md` including its colour/accent tables and dropped-surface summary, `07-release.md` identity section, `docs/design/DESIGN.md`, `tokens.rs`, `theme.rs`, all seven requested screen modules and every shared component module. Screen bodies and their mobile component paths were inspected; desktop-only primitives were inspected to establish the boundary, not redesigned.

Visual evidence inspected: `docs/evidence/gpui-kit-ui-rewrite/5.7/a-ios-menus-after-bootstrap.png`, `5.4/0-settings.png`, `5.4/a-login-empty.png`, `5.5/a-ios-appearance.png`, `5.6/a-ios-notifications.png`, `5.2/a-ios-orders-gate.png`, `5.3/a-ios-billing-gate.png`, `4.1/b-gallery-light.png`, `4.1/b2-gallery-light-page2.png`; and `docs/evidence/desktop-ui-redesign/1.2/shell-390-light.png`, `shell-768-light.png`, `shell-1280-dark.png`. The last three are **gallery compositions**, not proof that the mobile app has a sidebar or that a populated phone order/receipt screen was seen. No files were found under `artifacts/ui/`. Populated mobile orders and receipts are source-audited, not runtime visually verified. Available mobile images are historical evidence, not fresh captures of this branch.

Tool limitations: shell execution and reading the skill's external reference files (`contract.md`, `craft.md`, `motion.md`) were denied. The main `ui-design` skill was loaded. No build, simulator run, `make check` or OpenSpec validation was performed; this document is not a passing implementation handoff. Missing current captures are an explicit evidence gate below.

## Purpose

Employees need to read the actual lunch, select changes without mistaking them for completed orders, confirm once and understand their canteen/vending receipts. Preserve the direct, German-speaking, two-service product. Spend visual attention on today's menu and pending action; make settings and accounting quieter. Use the existing Rust gpui-kit view tree and native safe areas, logical pixels and input behaviour—not HTML, CSS breakpoints or a web shell.

## Audit findings

Each numbered line is one finding with its owning file. Counts are not inflated by counting inherited shared issues again on every screen. A requirement-mandated appearance can still be a design weakness, but changing it requires approval. Functional parity is never classified as slop.

### Menüs — 4

- M1 — `src/ui/app/src/screens/menus.rs`: 22 px orange category headings compete with the date, selection and submit action; accent spends itself on repeated chrome (§3.1 parity recipe, proposed amendment).
- M2 — `src/ui/app/src/components/menu_card.rs`: the main text renders `item.title` instead of the required dish `subtitle`; screenshot repeats `MENÜ I` inside its own group rather than presenting the lunch (code/spec defect, not a copywriting opportunity).
- M3 — `src/ui/app/src/components/menu_card.rs`: 10/11/13/16 px type plus 2/6/10 px spacing creates tiny badges and an unconsidered rhythm (§4.5 largely inherited).
- M4 — `src/ui/app/src/components/fab.rs`: absolute bottom/right offsets and an unconstrained mixed-action label risk masking content or overflowing at 360; the all-days pending change deserves a reserved, wrapping location (§3.1 amendment).

### Bestellungen — 3

- O1 — `src/ui/app/src/components/order_item.rs`: four competing type sizes and orange category metadata weaken the dish/status hierarchy (§4.6 inherited).
- O2 — `src/ui/app/src/components/order_item.rs`: 32 px cancel circle has no expanded mobile hit area in the code; the red text glyph is neither a consistent icon nor an adequate standalone touch target (spec expects hit slop).
- O3 — `src/ui/app/src/screens/orders.rs`: unconfirmed text and `Bestätigen` share an unwrapping row, while `Auswählen` and the selection strip have no explicit 44 px floor; long counts can crowd the phone.

### Abrechnung — 4

- B1 — `src/ui/app/src/screens/billing.rs`: equal visual weight for Gesamt/Belege/Zuschuss reads as mini dashboard metrics rather than a filtered receipt total (§3.3 inherited).
- B2 — `src/ui/app/src/components/bill_card.rs`: every amount is orange and Automaten gets a success-tinted source badge although source is not success (§4.7 inherited).
- B3 — `src/ui/app/src/screens/billing.rs`: source filters use 12 px text, 6 px vertical padding and radius 20, making small pills rather than considered touch controls (§3.3 inherited).
- B4 — `src/ui/app/src/components/bill_card.rs`: 10/12/13/14/18 px type and 6 px line gaps create more hierarchy levels than receipt data warrants (§4.7 inherited).

### Einstellungen — 2

- S1 — `src/ui/app/src/components/settings_row.rs`: tertiary hint and chevron are too faint to carry account status and navigation; the title lacks explicit shrink/wrap protection against the trailing chevron.
- S2 — `src/ui/app/src/screens/settings.rs`: 24 px margins on both sides of every divider spread four equal-weight rows over most of the phone, with no screen heading (historical screenshot and §3.4).

### Kantine-Zugangsdaten — 3

- K1 — `src/ui/app/src/screens/login.rs`: 13/14/15/28 px text and 6/14 px form spacing are a patchwork rather than three deliberate roles.
- K2 — `src/ui/app/src/components/text_field.rs`: mobile field wrapper does not explicitly set 16 px input type, a 48 px minimum or a visible independent focus boundary; actual kit defaults must not determine the form contract.
- K3 — `src/ui/app/src/screens/login.rs`: both Save and authenticated Logout are full-width filled actions, making a routine destructive escape compete with the form's purpose.

### Automaten-Zugangsdaten — 3

- V1 — `src/ui/app/src/screens/login.rs`: the same form spacing/type patchwork as K1 affects this distinct surface; its explanatory subtitle must remain subordinate, not become a fourth role.
- V2 — `src/ui/app/src/components/text_field.rs`: same missing mobile input size/target/focus contract as K2, including the masked field.
- V3 — `src/ui/app/src/screens/login.rs`: same competing Save/Logout filled surfaces as K3; two independent service sessions must not look like a SnackPilot account.

### Anmeldung (existing auth-gate surface, not a route) — 2

- A1 — `src/ui/app/src/screens/menus.rs`: expected signed-out status uses error red and a tiny tertiary instruction; no actual login failure is necessary for this state.
- A2 — `src/ui/app/src/screens/orders.rs`, `billing.rs`: faint, isolated `Anmeldung erforderlich` has no clear next action; neither module defines an `Anmeldung` page.

### Darstellung — 3

- D1 — `src/ui/app/src/screens/appearance.rs`: three equal-sized nested option cards inside another card are redundant containers; a mutually exclusive choice needs a group, not a card grid (§3.7 inherited).
- D2 — `src/ui/app/src/screens/appearance.rs`: 5 × 56 px minimum swatch targets + 4 × 16 px gaps = 344 px inside only 280 px at a 360 px viewport after page/card padding; the required single row cannot fit, and current wrapping is accidental.
- D3 — `src/ui/app/src/screens/appearance.rs`: labels use tertiary/primary regardless of the actual fill, while a same-colour swatch border cannot independently identify selection (inherited contrast weakness).

### Mitteilungen / existing Benachrichtigungen — 4

- N1 — `src/ui/app/src/screens/notifications.rs`: location setup and two diagnostic durations all use orange filled buttons; rare diagnostics compete with the useful reminder.
- N2 — `src/ui/app/src/screens/notifications.rs`: 12/13/14/15/18/28 px text, 6 px icon gap and radius 10 hint add avoidable local variants.
- N3 — `src/ui/app/src/screens/notifications.rs`: the long location action needs explicit wrapping; `Log verwerfen` and the Android settings hint lack explicit two-dimensional 44 px target floors.
- N4 — `src/ui/app/src/screens/notifications.rs`: the log hint says 24 hours although the adjacent choices offer 12 or 24; this is vague/misleading inherited copy, not permission to alter log duration semantics.

### Shared component/system findings — 14

- C1 — `src/ui/app/src/theme.rs`: `on_primary` is always white despite bright dark accents and Gold; mobile currently lacks the incumbent desktop's per-fill readable-ink projection.
- C2 — `src/ui/model/src/tokens.rs`: `TEXT_TERTIARY` is suitable for disabled decoration, not the numerous meaningful inactive labels, dates and hints assigned to it; do not change the pinned token to fix misuse.
- C3 — `src/ui/app/src/theme.rs`: primary/FAB coloured shadows turn action into glow (§7 inherited); restrained mobile variants should remove them, not recolour the brand.
- C4 — `src/ui/app/src/components/card.rs`: generic card uses a 10 px content gap while mobile screens repeat their own spacing; one named grid is missing.
- C5 — `src/ui/app/src/components/chip.rs`: generic chips use 6 px gaps/padding and do not establish a touch-target floor; status and interactive choice need different geometry.
- C6 — `src/ui/app/src/components/banner.rs`: small accented refresh banners and local screen duplicates have inconsistent 12/13 px text and 12/14 px spinners.
- C7 — `src/ui/app/src/components/broadcast_banner.rs`: text `✕` and horizontal-only padding leave the dismiss target narrower than 44 px.
- C8 — `src/ui/app/src/components/day_navigator.rs`: text glyph arrows and 2 px metadata gap differ from Lucide back/chevrons; ordinal metadata is tertiary though useful.
- C9 — `src/ui/app/src/components/dialog.rs`: 10 px button gap and several small text sizes need the mobile scale; preserve modal resolution, backdrop cancellation and focus ownership.
- C10 — `src/ui/app/src/components/loading_overlay.rs`: elevated spinner card adds another ornamental container; no-content loading should be quiet, and cached content must not be blanketed.
- C11 — `src/ui/app/src/components/segment_tabs.rs`: inactive tertiary tab labels and 14 px typography compromise readable navigation; equal tab widths themselves are intentional navigation, not repeated-card slop.
- C12 — `src/ui/app/src/components/tab_bar.rs`: inactive tertiary icons/Android labels have insufficient emphasis; the iOS pill and 5 px dot are deliberate platform parity, not grounds to replace navigation unilaterally.
- C13 — `src/ui/app/src/components/time_chips.rs`: 14/10 px padding, radius 10 and no explicit target height add another option recipe outside the shared chip geometry.
- C14 — `src/ui/app/src/components/toast.rs`: `Neue Menüs verfügbar!` shouts; preserve its event and lifetime, remove the exclamation only after copy approval.

Coverage with no additional slop finding: `button.rs` uses the inventoried recipes (colour/target problems already C1/C3/C5); `spinner.rs` rotates only to communicate real work, not decorative looping; `swipe.rs` and `pull_refresh.rs` are interaction primitives, not ornament; `switch.rs` already uses the one accent and must gain the common target/focus treatment, not a new illustration. `mod.rs` is inventory only. `desktop_navigation.rs`, `record_table.rs`, `workspace.rs` and all desktop variants are outside redesign scope. No emoji decoration, hue gradients, mixed icon brand palette, greeting headline, hype paragraph or gratuitous bubbling animation was found in the audited mobile paths. Do not invent audit failures to justify a redesign.

Required behaviour, **not slop**: persist-before-validate; logout retaining credentials and cookies; device-local midnight splitting upcoming/past; Vienna 09:00 eligibility; `de-AT` formatting; 200-entry diagnostic cap; independent Gourmet/Ventopay authentication.

Audit counts: Menüs 4; Bestellungen 3; Abrechnung 4; Einstellungen 2; Kantine form 3; Automaten form 3; Anmeldung gates 2; Darstellung 3; Benachrichtigungen 4; shared 14. **42 findings** including 28 attributed to surfaces. No extra finding is implied for a nonexistent Anmeldung route or Mitteilungen inbox.

## Direction

1. **Product:** SnackPilot, the employee's dated lunch tray and canteen/vending receipt book.
2. **Character:** a dated canteen ticket: legible dish names, aligned prices, explicit pending marks, one decisive action.
3. **World:** the workplace lunch counter's printed day sheet; not a food-delivery storefront or a dashboard of coloured tiles.
4. **Moments:** richness belongs in the day tray's selection and its pending-order dock; receipts use alignment, not decoration.
5. **Signature:** a selected lunch gets a small inset check beside its dish, directly echoed by the all-days counted action dock—choosing food is visibly not yet ordering it.

Use existing palette roles, never a second brand: `BACKGROUND` #F5F5F7/#000000, `SURFACE` #FFFFFF/#1C1C1E, `TEXT_PRIMARY` #1D1D1F/#F5F5F7, `TEXT_SECONDARY` #6E6E73/#A1A1A6, default `PRIMARY` #D4501A/#FF6B35, `ERROR_TEXT` #B3261E/#F28B82. Status surfaces/inks are existing semantic pairs. Other accents remain user preferences, not icon-specific colours. Inter remains the only family; use Regular/Semibold/Bold already bundled.

One-column concept at all mobile widths: **context → records → pending action → platform navigation**. More width increases margins, not category tiles. At 1280 a mobile host retains the 560 px content cap; the desktop host retains Desktop Ledger. No mobile sidebar.

```text
360 phone                 390 phone                  768 tablet/mobile host
safe top                  safe top                   safe top
<  date / ordinal  >      <   date / ordinal   >      104 | < date / ordinal > | 104
16 | category       |16   16 | category         |16        | category         |
   | dish           |       | dish             |          | dish             |
   | allergens price|       | allergens   price|          | allergens   price|
   | next category  |       | next category     |          | next category     |
   | counted action |       | counted action    |          | counted action    |
platform tabs + safe      platform tabs + safe        platform tabs + safe

1280 mobile host: 360 | same 560 px single content column | 360
1280 desktop host: incumbent rail + workspace; no redesign here
```

Critique decisions: the day/date, Gourmet category order, actual dish and all-days pending count make this specific to SnackPilot. There is no invented hero, rail, meal photography, greeting, total-price cart calculation, checkout sheet, dotted receipt edge or accessory animation. Neutral settings groups are not food-themed. The existing glass/flat dialects remain; no new blur engine is required.

## Screen contracts

### Common mobile contract (applies to every surface below)

All dimensions are logical px. At 360: 16 px outer inset, 328 px content; at 390: 16 px outer inset, 358 px content. At 768: centre a 560 px content column (104 px gutters); backgrounds and platform bottom navigation still span the viewport. Between widths use `min(viewport − 32, 560)`. At 1280 on mobile use the same cap. No side-by-side menu/order/card grid. Credential forms cap at 440 px at 768 (164 px gutters). Safe-area top is applied once, outside the column. Pushed screens have a 48 px minimum back row, then 16 px gap to title, then 24 px to content; titles wrap, never ellipsize. No invented top safe-area spacer.

Use the three Mobile Day Tray text roles defined below everywhere except the intentionally preserved 12 px Android tab labels. The shell supplies a `Menüs`/`Bestellungen`/`Abrechnung`/`Einstellungen` heading in the headline role, 16 px below safe top and 16 px above screen controls. Avoid duplicate headings inside cards. A scroll view owns records/forms; heading and context controls remain outside the record scroll, but at short/large-type viewports the whole content may scroll so controls never consume all available content height. Dock and navigation remain reachable. Scroll-end clearance = actual navigation obstruction + actual dock height (when present) + 16; do not keep a blind 100 px padding in addition to shell reservation. If navigation is already outside the viewport allocated to the scroll region, do not count it twice.

iOS keeps its incumbent 248×56 r28 floating pill, icons-only and active dot, bottom=max(safeBottom,8); Android keeps its 64 px full-width labelled bar, with the system inset reserved outside its content. Neither is an ordinary content card. At 768 neither becomes Desktop Ledger. Pushed screens retain current navigation topology and active-tab identity; changing bar visibility on pushes is not part of this design.

Focus/pressed/disabled on every screen follows the shared contract below. No hover-dependent mobile action. All copy remains byte-identical except the explicitly proposed rows in the copy table, and those rows are blocked pending approval.

### Menüs

**Purpose / hierarchy:** choose one day's real food; understand pending versus placed. Tree: shell heading → `day_navigator` → neutral refresh OR order-progress banner → error/banner stack → swipe wrapper → scrolling category groups → mobile pending dock → platform tabs. Existing operator broadcast and new-menu toast remain shell overlays with their existing event precedence.

**360 / 390:** use the common widths. Day arrows 48×48 with 20 px Lucide `ChevronLeft`/`ChevronRight`; middle is a min-48 target with date headline and ordinal metadata, gap 4. Category label is body Semibold `TEXT_PRIMARY`, 8 above its cards; group gap 24. Each `menu_card`: padding 16, gap 8, auto height, 8 after card, existing `card_surface` radii (16 iOS /14 Android). Dish is **subtitle**, falling back to title only when subtitle is empty; body, maximum four lines as required. Reserve an inline 24 px check slot plus 8 gap when selected; never float a check over text. Footer: metadata allergens left, body Semibold unchanged price string right; if they cannot fit, allergens get their own line above right-aligned price. Do not silently remove allergen letters.

**768:** same hierarchy in 560 px; no extra day column, no Master/Detail. Dock matches column width. The existing swipe/today/arrow navigation remains.

**Tokens:** `BACKGROUND`, `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `BORDER`, `PRIMARY_SURFACE` / `GLASS_PRIMARY`, `SUCCESS_SURFACE` / `GLASS_SUCCESS`, `WARNING_SURFACE` / `GLASS_WARNING`, `ERROR_SURFACE` / `GLASS_ERROR`, semantic text/border pairs; `card_surface`, `tinted_banner`, `circle_button`. Use `mobile.selectionInk`, `mobile.selectionBoundary`, `mobile.actionFill/Ink` below, not desktop palette fields.

**Selection / actions:** pending selected = incumbent accent tint rather than full orange field, 1 px resolved boundary and inline Lucide `Check` 20. This is the signature, not a new saved status. Existing status precedence remains cancellation → ordered → unavailable → cutoff; badge metadata, r8, horizontal8/vertical4. Pending-cancel remains dashed + strike-through, but readable ink rather than blanket opacity. Ordered uses success tint + `Bestellt`; unavailable/cutoff use their existing labels and disabled actions without dimming readable dish text. Do not change ordered-item cancellation eligibility based on appearance.

Dock appears under exactly the existing pending-count/progress conditions, submits the same all-days set with exactly `model.fab_label()`. Full column minus the common gutters, min48, r14 iOS/r12 Android, horizontal16/vertical12, text centred/wrapping; reserve its measured height above navigation, separated by 8. During operation show the existing exact phase text in the dock region and remove repeat action; no new success dialog. No new current-day total/count or promise of server success. Cached-refresh retains cards and pending selection, uses neutral spinner16 + `Aktualisiere...`; initial no-data/auth-pending uses loading state. Empty/error/auth/offline and eligibility use the state matrix. **Acceptance:** actual dish visible in sample M2, badge never confused with check, mixed action at 360 does not clip, last card is scrollable fully above dock. **Non-goals:** new filtering, changed swipe thresholds, checkout, quantity selection, new order semantics. Domain strings, category order and `SUPPE & SALAT` heading suppression stay identical.

### Bestellungen

**Purpose / hierarchy:** distinguish coming/past and confirmed/outstanding; explicitly confirm or cancel. Tree: shell heading → `segment_tabs` → optional confirm banner → error → scrolling `order_item`s → existing selection controls/dock → tabs.

**360 / 390:** segments min48, two equal targets, body Semibold with wrapping count labels and 3 px selected rule reserved in both states; this equality is earned. Cards padding16/gap8, r16/14. Date and category share metadata role in `TEXT_SECONDARY`, separated by 4; dish body `TEXT_PRIMARY`. Top date row can wrap, do not abbreviate formatter output. Status badge r8 on its semantic surface, metadata. Cancellation sits in a separate trailing 44×44 target with Lucide `X`20, neutral rest surface and `ERROR_TEXT` glyph; do not mistake X for deleting a local list row. At narrow/large type, status and cancel move below dish rather than squeeze description. Existing description lookup/fallback stays unchanged.

Confirm banner stacks text above a full-width 48 px `Bestätigen` at 360/390; at 768 it may use one row, gap16, only if both fit. `Auswählen` is a 48 px text target outside list content. Existing selection mode retains whole-card toggle and check in a reserved 44 px area; the counted `Stornieren ({count})` / `Fertig` strip wraps if needed. It clears navigation like the menu dock. No new long-press or swipe-to-cancel gesture.

**768:** 560 px column, no table/detail preview. **Tokens:** `card_surface`, `tinted_banner(GLASS_WARNING/GLASS_ERROR)`, `SURFACE`, `BORDER`, semantic `SUCCESS_TEXT`, `WARNING_TEXT`, `ERROR_TEXT` with readable projection; mobile option/action/focus tokens. Lucide `Check`20 only for batch check, `X`20 for cancellation; no decorative leading icon.

**States / input:** initial loading only without cached records; cached refresh keeps segment/count/data; empty text remains segment-specific. Unauthenticated uses Anmeldung contract. Cutoff retains visible disabled cancel (no handler); during cancelling use spinner16 in the same 44 px slot, disable competing mutations with the existing gate and preserve existing hide-other-cancel rule. Progress does not create another cancellation path. Offline retains available cache and exact errors, no automatic retry of cancellation. **Acceptance:** longest dish/count and two-button strip fit at360; read-only past rows have no implied tap action; confirm dialog exactly preserves `Bestellung stornieren`, `"{order.title}" stornieren?`, `Stornieren`, `Behalten`. **Non-goals:** new batch semantics, re-sorting, different midnight rule. Existing counts, status words and action labels stay identical.

### Abrechnung

**Purpose / hierarchy:** see spending in the selected month/source, then read actual receipts. Tree: shell heading → three month tabs → source filters → error/refresh status → one summary surface → scrolling `bill_card`s → tabs. Keep filters present during loading so context does not jump.

**360 / 390:** month choices min48, body16; allow a second text line if a full existing month label needs it, never horizontally truncate or reformat it. Source choices `Alle`, `Kantine`, `Automaten`: mobile option recipe r8, min44, horizontal12, gap8; wrap the group when text scale demands. Selected choice gets a check20 and tint, so colour is not the only cue. Summary `banner_surface`: padding16; `Gesamt` metadata followed by body Semibold amount at the leading edge; below, one wrapping metadata line containing Belege and conditional Zuschuss, gap16 between pairs. No three equal metric columns or extra total heading.

Receipts: `card_surface`, padding16, gap8. Date body Semibold, time metadata; amount body Semibold `TEXT_PRIMARY`, right-aligned on top row. Source is neutral metadata `KANTINE`/`AUTOMATEN` as currently rendered (not new sentence-case copy). On narrow rows source/time can wrap underneath. Gourmet items: quantity metadata in 28 px column; description body flex; amount metadata right. Descriptions may wrap to avoid obscuring actual content; no extra restaurant or Ventopay items invented. Neutral hairline between header/body; no perforations, financial icons or trend charts. Receipt heights follow content, not a fixed card height.

**768:** same 560 px receipt column; no desktop table and no detail route. **Tokens:** `BACKGROUND`, `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `BORDER`, `banner_surface`, `card_surface`, `selected_chip`/`unselected_chip` bases overridden only by named mobile option geometry; `mobile.selection*`. No success tint for a service name; subsidy text uses readable `SUCCESS_TEXT` when possible, else `TEXT_PRIMARY`.

**States / input:** either service's authentication is enough; no demand to connect the other. Initial no-data spinner36; cached refresh keeps totals and cards for the current selection, never displays previous-month data under a new month label. Empty/error/auth/offline as matrix; cutoff/unavailable ordering states do not apply. Preserve pull refresh trigger, source/month actions and exact formatter outputs. **Acceptance:** totals equal existing filtered totals, conditional subsidy unchanged, both source variants readable at360 with long descriptions/large amounts; no accidental button semantics on read-only receipt cards. **Non-goals:** adding an export, payment, running balance, total-price calculation or transaction details not supplied by the model. All visible labels/values stay identical.

### Einstellungen

**Purpose / hierarchy:** locate each independent account and preferences. Tree: shell heading → two existing credential rows → separator → Darstellung and Benachrichtigungen rows → Datenschutz target. Keep current row order; no new section titles or profile card.

**360 / 390:** `settings_row` body Semibold title, metadata hint, gap4; 80 px minimum row, vertical16, trailing `ChevronRight`20 in reserved24 px slot plus8 gap. Whole row is the target, title/hint wrap; no new leading icons. Single divider with16 vertical separation **between accounts and preferences**, 8 between neighbouring rows, not48 around every row. Datenschutz is min48 and metadata `TEXT_SECONDARY`, still centred. **768:** 560 px single list; selecting a preference pushes the same existing surface, not desktop containment.

**Tokens:** `BACKGROUND`, `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `BORDER`/`divider`, `SURFACE_VARIANT` press, mobile focus geometry. No primary fill on settings chrome. **States:** no fetch/skeleton/empty/error screen of its own; auth appears only in existing row hints; local preferences remain usable offline. Loading/cached-refresh/cutoff/unavailable apply only to the destination or host capability, not this list. Desktop continues hiding unsupported notifications, including its narrow shell. **Acceptance:** long account name does not collide with chevron, both independent session hints remain truthful. **Non-goals:** account consolidation, logout shortcut, new privacy text. Preserve `Angemeldet als {username}`, `Sitzung aktiv`, `Nicht angemeldet`, theme preference labels, the approved Aptabase Datenschutz message and existing service routes byte-for-byte.

### Kantine-Zugangsdaten

**Purpose / hierarchy:** edit Gourmet credentials, save then attempt login; no SnackPilot registration. Tree: scroll/keyboard-safe container → back `Einstellungen` → title → username group → password group → Save → authenticated session and Logout.

**360 / 390:** common16 px gutters; title uses headline and wraps at the existing hyphen rather than shrinking. Field-group gap16, label-to-field8. Labels metadata Semibold; input body16, min48, horizontal16/vertical12, `input_field` r12. Save full-width min48, body Semibold, mobile primary r14/12. Session separated by24; metadata existing session text; `Abmelden` min48, outline/neutral fill with readable error ink, not another solid primary-sized colour block. **768:** form440, centred, same vertical structure and pushed back affordance.

**Tokens / icons:** `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `BORDER_INPUT`, `input_field`, `SURFACE`/`GLASS_SURFACE`, `mobile.action*`, `mobile.dangerInk`, `mobile.focus*`; back `ChevronLeft`24, no leading account/key/eye icons. Keyboard scrolls focused field and Save into view; do not place dock or tabs above the keyboard. Username keeps no autocorrect/no auto-capitalisation, password stays secure and non-copyable/non-cuttable. Native selection/paste/IME behaviour remains; preserve the same input entities across rerenders and any desktop reprojection. Do not turn Return into a new submit path as part of styling.

**States:** idle prefill identical; busy Save disabled with `Speichern...` and spinner16, same reserved height; blank-field, failure and success use exact existing dialogs (not optimistic inline success). Authenticated section appears only for this service; offline/expired-session errors use current model error/fallback; never clear saved fields to suggest logout. Empty/cached-refresh/cutoff/menu-unavailable do not apply. **Acceptance:** at360 with keyboard and long error, field and dialog action remain reachable; repeated activation is still claimed once. **Non-goals:** credential policy, show-password, validation timing, account creation, logging or storage changes. Placeholders, labels, alerts, service binding, and session text stay byte-identical.

### Automaten-Zugangsdaten

**Purpose / hierarchy / widths / input:** use the preceding credential contract exactly, but bind to Ventopay. Between title and field groups retain `Für Automaten und Kassenabrechnungen` as metadata, gap8 then24 before fields. At360/390 wrap subtitle/title without changing strings; at768 use the same440 px form. Same `input_field`, `BORDER_INPUT`, `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, mobile primary/danger/focus tokens; back `ChevronLeft`24 only, same r12 inputs/r14 or12 actions,16/8/24 spacing.

**States:** use `ventopay_busy`, not Gourmet busy; secure prefill and Save behaviour are identical in ordering, not in account identity. Authenticated text stays `Automaten-Sitzung aktiv`; no username is invented. Success stays `Automaten-Zugangsdaten sicher gespeichert`. All other dialogs and busy label unchanged. Offline/error remain model-owned; no cache-loading, order-cutoff or food-unavailable states. **Acceptance:** signing in/out of Ventopay changes only its session display, not the other credential form; full long title and Save fit at360. **Non-goals:** merging accounts, changing PIN/password semantics or pre-validating credentials. Everything except the approved presentation geometry/ink stays identical.

### Anmeldung — shared authentication gates

**Surface decision:** neither code nor §1 defines a standalone Anmeldung destination. Cover the existing signed-out/pending gates; do not add an onboarding/login-selection screen. This is an explicit human question, not missing code to fill speculatively.

**Tree / hierarchy / widths:** keep active tab heading and navigation; centre a text stack in remaining area, common328/358/560 px column with16 padding. Menüs retains `Nicht angemeldet` body Semibold and the existing instruction as readable metadata below8; Bestellungen/Abrechnung retain `Anmeldung erforderlich` body. Add a min48 neutral outlined `Einstellungen` target16 below that selects the existing settings tab (no direct credential deep link). This new affordance requires §1/§3 gate approval. No orange error headline unless there is an actual reported auth error, shown separately using the exact model message. `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `button_secondary` geometry r14/12, `BORDER`, mobile focus; no decorative icon.

**States:** auth idle/loading spinner without a false signed-out message; authenticated reveals existing screen; absent credentials shows gate; login error retains error feedback plus way to settings. Billing gate only when neither service is authenticated. Offline is not synonymous with logged out—do not discard an authenticated cached view. Cutoff/unavailable/empty-record/cached-refresh are not auth states and belong to the destination. **Acceptance:** a gate is navigable without guessing which icon is Settings; no credential entry occurs on it. **Non-goals / identical:** no SnackPilot account, new provider calls, new saved-state interpretation or route; existing auth-gate strings stay identical.

### Darstellung

**Purpose / hierarchy:** choose existing theme and accent. Tree: back → title → Design section → one three-option group → Akzentfarbe section → five labelled swatch targets. Remove nested card-within-card presentation. Sections gap24, title-to-options12. Do not add a preview mock device.

**360 / 390:** use common content width without the extra20 px inner card padding. `System`, `Hell`, `Dunkel` are three equal min48 options in a single r8 group; no icon above label and no equal square tiles. Keep existing `Smartphone`, `Sun`, `Moon` as20 px inline glyphs, gap8; each option body Semibold. If larger text cannot fit all three, stack the same options as full-width rows, maintaining order. Swatches: five equal columns in the available328/358, each min44 wide and min64 high;40 px colour disc centred above metadata label, gap4; group gap4. Thus five × (328−16)/5 fits without incidental wrap. At large text allow a deliberate3+2 row arrangement, never clip names. Each full cell is the target; swatch colour is always the existing **light-mode** primary, including dark appearance. Check20 readable black/white plus an independent2 px `TEXT_PRIMARY` outer ring with2 px background gap indicates selection; same-colour border alone is insufficient.

**768:** same group structure in560 px; do not make swatches larger or theme tiles taller to fill space. **Tokens:** `ACCENTS` unchanged, `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `SURFACE`, `BORDER`, `PRIMARY_SURFACE`/`GLASS_PRIMARY`, `selected_chip`/`unselected_chip` bases and mobile option/selection/focus tokens. Only the swatches show five colours. **States:** theme/accent selection immediate and persisted through existing setters; System follows OS; inactive/selected/pressed/focus independent. Offline/auth do not disable local preferences. No loading, cached-refresh, empty, cutoff or unavailable surface is invented; platform alternate-icon errors retain existing treatment. **Acceptance:** all five names visible at360 without default-size wrap; selected item identifiable in greyscale and all accents; existing app-icon coupling unchanged. **Non-goals / identical:** no sixth accent, renaming Design, palette value edits or icon-delivery changes; all German labels/order stay byte-identical.

### Mitteilungen — existing title Benachrichtigungen

**Surface decision:** `Mitteilungen` in the request maps to the existing notifications settings surface, not an inbox. Keep visible title/row `Benachrichtigungen`; renaming a precise current string is outside the copy rule.

**Tree / hierarchy:** back → title → Bestell-Erinnerung section → Standort-Benachrichtigungen section → Benachrichtigungs-Log section. **360 / 390:** common column; title wraps. Section gap24, heading body Semibold, hint metadata,8 to hint/16 to controls. Divider `divider` retained between sections. Reminder switch row min48, text body; track accent only when on. Twelve existing time options remain one horizontal scroll strip, min44 tall, r8, horizontal12/vertical12, gap8, selected check20 plus tint/visible boundary. Reveal selected time on entry without changing the saved time. No new time picker or 09:00-based reminder range.

Android background hint uses `SURFACE_VARIANT`, r12,padding12, leading `Info`20 neutral, metadata body/hint wrapping; whole target min44 in both axes, existing linked substring remains. Location setup is full-width min48 neutral outline, existing label wrapping at word boundaries, not ellipsized. Resolving location uses the exact busy string, spinner16 and disabled state, no label-string-based policy. Set location status metadata; remove target min48 outlined with readable error ink. Diagnostic duration choices are two min48 secondary actions (not two primary cards), equal flexible width, gap8; wrap into two rows if needed. Active/expired log status metadata; expired send is neutral outline with `Mail`20 and gap8; discard min48 text target. Location and diagnostics do not compete with menu submit's orange fill.

**768:** same560 px single column; keep times scrollable, no diagnostic side panel. **Tokens:** `BACKGROUND`, `TEXT_PRIMARY`, readable `TEXT_SECONDARY`, `BORDER`, `SURFACE_VARIANT`, `button_secondary` base, `input`-independent option recipe, `mobile.selection*`, `mobile.dangerInk`, `divider`; `Info`/`Mail`20 and back24, no bell illustration. **States:** reminder off hides time row; permission denial keeps switch off and existing alert/settings path; on shows persisted time. Location absent/resolving/set/error remain exact state machine; permissions/notification requirements unchanged. Log inactive/active/expired remain mutually exclusive with exact durations/status formatting/actions. Offline local settings/log remain available, while location/mail/settings failures use exact current dialogs; desktop unavailability stays truthful and has no functioning-looking controls. Cached-refresh/order-auth/cutoff/menu-empty do not apply. **Acceptance:** long location label and all dialog actions fit at360; off/on cannot be confused; expiry count and log discard/send semantics unchanged. **Non-goals / identical:** no notification inbox, schedule/geofence changes, email-template changes or permission pre-prompts; all copy identical except approved N4 table amendment.

## Shared components and tokens

### Named extension, not a second inventory

All proposed names below are **new Mobile Day Tray extension tokens**, not claims that these symbols already exist in Rust. Existing colour constants and `AppTheme` recipes remain pinned. These names describe a mobile presentation projection; do not rename public functions, add required arguments to shared APIs or silently reuse `theme.desktop` as a mobile theme. Approved durable decisions should later be recorded in `DESIGN.md` through the normal spec workflow; this brief does not edit it.

| Extension token | Exact contract |
|---|---|
| `mobile.space.1/.2/.3/.4/.5/.6/.8` | 4 / 8 / 12 / 16 / 20 / 24 / 32 px; all new layout gaps/padding use these |
| `mobile.contentMax`, `mobile.formMax` | 560 / 440 px |
| `mobile.target`, `mobile.actionHeight` | 44 px two-axis minimum / 48 px minimum; text may increase height |
| `mobile.type.headline` | bundled Inter Bold,24/32; screen titles and menu date, never a greeting |
| `mobile.type.body` | Inter Regular16/24; Semibold for controls, group headings, key amounts |
| `mobile.type.meta` | Inter Regular14/20; Semibold for status/labels when needed |
| `mobile.radius.option`, `mobile.radius.status` | 8 / 8 px; no new capsule badges |
| `mobile.focusWidth`, `mobile.focusGap` | 2 / 2 px; reserve4 px independent room without changing rest dimensions |
| `mobile.focusInk` | `TEXT_PRIMARY`, tested against actual adjacent fills at3:1 |
| `mobile.secondaryInk(fill)` | `TEXT_SECONDARY` when at least4.5:1 on the actual composited fill, otherwise `TEXT_PRIMARY` |
| `mobile.statusInk(fill, preferred)` | existing `SUCCESS_TEXT`, `WARNING_TEXT` or `ERROR_TEXT` when4.5:1, otherwise `TEXT_PRIMARY`; never invent a semantic hue |
| `mobile.selectionFill` | existing `GLASS_PRIMARY` iOS / accent's `PRIMARY_SURFACE` Android |
| `mobile.selectionInk` | `TEXT_PRIMARY`; readable metadata uses the projection above |
| `mobile.selectionBoundary` | accent `PRIMARY` if3:1 against selection fill and neighbouring fill, otherwise `TEXT_PRIMARY` |
| `mobile.actionFill` | selected accent's `PRIMARY_DARK` light / `PRIMARY` dark; primary brand token values unchanged |
| `mobile.actionInk(fill)` | whichever of existing white (`SURFACE.light`) and black (`BACKGROUND.dark`) gives higher contrast, minimum4.5:1 |
| `mobile.dangerInk(fill)` | `ERROR_TEXT` if4.5:1, otherwise `TEXT_PRIMARY`; danger meaning also carried by exact action label |
| `mobile.pressedFill(fill)` | mix14% `TEXT_PRIMARY` into the resting fill; recompute readable ink; no scale/bounce |
| `mobile.disabledFill`, `mobile.disabledInk` | `SURFACE_VARIANT`, readable secondary projection; no whole-content opacity reduction |

Existing radius exceptions intentionally retained: card16 iOS/14 Android; input12; button14/12; circle24 for48 px targets; iOS navigation28; modal14. These are **incumbent geometry**, not licence for new one-off spacing. Borders0.5/1, selection rule3 and navigation dot5 are optical/structural exceptions to the4 px spacing grid. Platform elevation and glass approximations are not new decoration: existing card/day/navigation material may remain, but the new mobile primary/secondary/dock variants have **no coloured shadows**; no new shadow, blur or glow anywhere.

Icon system: existing bundled Lucide via gpui-kit; no emoji, Unicode faux icons or new icon dependency. Content controls20 px; back24; navigation26 iOS/24 Android retained. `Utensils`, `Receipt`, `Wallet`, `Settings` tab map unchanged; active tint where readable plus incumbent dot/label; inactive `TEXT_SECONDARY` projection, not tertiary. Primary accent is concentrated in the pending dock/primary form action and actual selection; navigation/switch/option state are semantic exceptions, not a licence to colour headings, receipt sources or every amount.

### Shared interaction treatment

- Touch target44×44 minimum, primary actions48 high; target geometry never overlaps a neighbouring control. Decorative20/24 px glyphs are not the hit box.
- Keyboard/Switch Control focus independent of selection:2 px `mobile.focusInk` with2 px gap, no layout shift or clipping. Native text selection/caret and IME remain. Tab visits enabled controls in visual order; Enter/Space activate a focused non-input control once; no global arrow/Return interception. Assistive names use existing words: previous/next day, today, cancellation, settings; selected/busy/disabled are exposed as states, not inferred only from tint. Gate button labelled `Einstellungen` uses the actual destination name.
- Press changes fill only, synchronously with contact; cancel on release outside or scroll claim, no haptic or animation invented. Disabled controls have no mutation callback, do not become enabled merely by focus and do not lose readable context. Enforce existing model/shell mutation claims; styling is never the duplicate-submit protection.
- Read-only receipts/default order bodies are not buttons. Menu card is one selection target; no nested decorative check target. In order selection mode the whole card remains the existing check action; cancel is not simultaneously reachable through that same tap.
- On short screens and accessibility type, wrap before clipping and grow targets. Dialog body scrolls with actions always visible. Screen-reader reading order follows heading/context/status/records/action/navigation; announce existing status updates without repeatedly reading the full list on refresh.
- Mobile variants cover `card`, `chip`, `button`, `banner`, `broadcast_banner`, `text_field`, `day_navigator`, `menu_card`, `order_item`, `bill_card`, `segment_tabs`, `settings_row`, `time_chips`, `switch`, `fab`, `dialog`, `loading_overlay`, `tab_bar`, `toast`. Keep existing signatures and data/intent boundaries. Do not migrate desktop composition or native menu/focus logic into this extension.
- No new authored motion. Keep toast300/4000/300 ms, its event/dismissal policy, dialog's existing fade and gesture constants; no looping animation except a real-work spinner. Reduced-motion alternative: toast appears/disappears without travel but retains4000 ms lifetime, day navigation swaps without spring; this presentation accommodation needs the listed requirement amendment. No new timing constants are proposed because the external motion reference could not be loaded.

## States

This matrix is normative for every screen; “not applicable” is deliberate, not an excuse to fabricate a placeholder or claim that a state was runtime observed. The model remains the authority for network/auth/progress/eligibility; do not infer network status or auth failure from an empty array.

| State | Menüs | Bestellungen | Abrechnung | Settings / forms / appearance / notifications |
|---|---|---|---|---|
| Default | selected day's grouped dishes | selected segment and exact counts | exact month/source, filtered total and receipts | local values and destination-specific controls described above |
| Initial loading | spinner36 in empty content; auth pending distinct | spinner36 only without cached records | spinner36 only without selected-context cache; filters stay | Settings/appearance none; forms disabled `Speichern...`; notifications location busy only |
| Cached-refresh | retain cards/checks/day, neutral16 px spinner row | retain rows/counts/segment; same neutral refresh treatment | retain selected-context entries/totals; same treatment | no invented cache phase on local forms/settings |
| Empty | `Keine Menüs verfügbar` only after loading ends | exact coming/past empty string | `Keine Abrechnungsdaten für diesen Monat` | not applicable; empty credential fields are valid idle form state, not an empty screen |
| Error | inline exact model message; current `Erneut versuchen` min44 | exact existing inline error; no invented cancel retry | exact existing inline billing error, keep available source data | exact credential/permission/location/mail dialogs; no global error screen |
| Auth | Gourmet gate, pending spinner | Gourmet gate | gate only if both signed out | existing independent row hints/session sections; preferences remain available |
| Cutoff | exact model flags, existing `Geschlossen` badge and interactivity precedence | disable cancellation at model's cutoff and existing cadence | not applicable | not applicable |
| Unavailable | `Ausverkauft` for model unavailable menu, not same as cutoff | no fabricated food availability state | no fabricated missing-service failure if the other service works | unsupported desktop notifications retain current truthful sentence; platform permissions retain existing alerts |
| Offline | retain cache when existing auth allows; error model-owned | retain cache, no automatic mutation replay | retain valid selected-context cache and partial source data | local preferences/log continue; login/location/mail errors stay truthful; no new queue |

Empty-state text uses body16 `mobile.secondaryInk` with centred wrapping and16 px padding, no illustration. No spinner over nonempty cached content; network work does not erase context. A failing source is not a fabricated zero total from that source. Do not add `Offline`, “last updated” timestamps or retry buttons that the model cannot truthfully support. Initial auth flags in the current view API may not distinguish pending/no-credentials: expose the existing model status rather than inventing a timeout, and resolve the API/boundary question with spec before implementation.

## Copy table (before → after)

Only shouting or vague/misleading **current** strings are proposed for change. The literal `...` busy strings are progress indicators, not punctuation errors; preserve them. Source category caps (`MENÜ I` etc.), receipt source caps and complete explanatory sentences are not hype and remain unchanged. All rows here require approved amendments; until approval, exact current strings win.

| Location | Before → After | Reason / approval scope |
|---|---|---|
| New-menu toast, `04` §4.8 | `Neue Menüs verfügbar!` → `Neue Menüs verfügbar` | remove shouting; keep trigger, duration and notification-system copy outside this in-app toast unchanged |
| Notification log hint, `04` §3.8 | `Zeichnet 24 Stunden lang Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.` → `Zeichnet für 12 oder 24 Stunden Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.` | clarify the two existing choices; not a new duration |

New affordance, not a rewrite: Anmeldung gate target uses existing destination string `Einstellungen`; this is a navigation-recipe amendment. No rename from `Benachrichtigungen` to `Mitteilungen`, `Design` to another term, `Login fehlgeschlagen` to another title, `Wird storniert` to another semantic state, or `Speichern` to a promise of immediate login. Privacy follows the already approved Aptabase paragraph in code/release authority, not historical TelemetryDeck copy from04; no further rewrite is proposed.

## What stays identical

- Scraping sequences, methods, headers, encodings, parameters, request shapes, ban-sensitive anti-bot fields, service endpoints, caching/fingerprinting logic and order/cancel/confirm semantics. No changes to `src/core`.
- Pending composite identities, all-days submit scope, counts, category grouping/order, `SUPPE & SALAT` heading suppression, ordered-category union, swipe claim10/commit50/edge0.3 and current refresh triggers. No new fetch from repaint, resizing, selection-ring state or tab reselect.
- Device-local midnight upcoming/past split; Europe/Vienna same-day09:00 order/cancel rules and existing periodic eligibility recheck. Visual disabled state never changes eligibility. Any already selected/ordered edge case remains model-owned.
- All `de-AT` day/date/time/month/currency formatter outputs and raw menu price strings; menu/order descriptions and actual bill lines are data, never rewritten by the design. Correcting M2 displays the required existing subtitle, not invented food copy.
- Credential prefill, secure storage, persist-before-validate ordering, masked password clipboard restrictions, independent service statuses, logout retaining credentials/cookies, failed-save dialog semantics and error bodies. No new validation or logging policy is implied.
- Notification permission/geofence/background-sync logic, allowed times11:00–13:45/15 minutes, location reminder08:45, duration12/24 hours,200-entry cap, expiry formatting, recipient/subject/body export and retained log data behaviour. No notification inbox.
- `dev.radaiko.gourmetclient` bundle/package identities, SnackPilot name, alternate-icon coupling and five accent IDs/order/light swatch values. GourmetClient README credit/link stays unchanged.
- Shared public component API and data/intent types used by desktop; Desktop Ledger geometry, menus, keyboard/focus/containment, mutation dispatch and wide layouts are not redesigned. Narrow desktop must keep working with the shared compositions and desktop-only unsupported capabilities. Host-gated mobile visual variants must not silently restyle wide desktop callers.
- Existing German UI copy byte-for-byte except the two explicitly approved table substitutions. This includes busy phases, FAB labels, pluralisation, placeholders, alerts, source labels and external/operator-provided text. Do not trim punctuation from complete sentences or sanitise meaningful order titles as an aesthetic exercise.
- Preserve the incumbent iOS pill geometry, Android bar identity, system status-bar adaptation, dialog call shapes, one resolution per modal, backdrop action0 and current push/pop/back graph. `Anmeldung` remains gates, `Mitteilungen` remains the existing notifications surface unless the human decides otherwise.

## Required requirement amendments

These are proposed approvals, **not edits and not implementation tasks**. `04-ui-ux.md` currently says reproduce text verbatim and specifies dimensions/styles. A redesign brief cannot override it implicitly. If an amendment is declined, that portion must remain incumbent and the brief must be reconciled before coding rather than quietly mixing designs.

1. **§2, §3 common shell, §7–8:** approve readable mobile ink projections without changing pinned base values; three text roles,4 px spacing, touch/focus floors, measured safe-area/scroll clearance and mobile560/440 content caps. Keep navigation geometry but replace low-contrast inactive ink; keep optical border/dot exceptions. Approve token-derived action fill/ink and removal of coloured action shadows.
2. **§3.1, §4.4–5:** approve neutral category headings, Lucide chevrons/checks, larger/readable metadata/status, tinted pending selection instead of solid accent, no blanket dimming of readable disabled text, and reserved wrapping pending dock instead of absolute FAB. All status precedence, actions and counts remain. M2 subtitle rendering itself restores §4.5; confirm empty-subtitle fallback explicitly, not as an excuse to change dish data.
3. **§3.2, §4.6:** approve wrapping confirm/selection layouts,44 px cancel/check targets and consistent icon/status typography; document already-existing batch controls if no later requirement owns them. Preserve all mutation and cutoff semantics.
4. **§3.3, §4.7:** approve controls remaining in initial load, readable month/source targets, primary-total/secondary-detail summary hierarchy, neutral source/amount treatment and wrapping receipt descriptions instead of one-line clipping. No totals or formatter logic changes.
5. **§3.4–6:** approve shell heading, restrained settings row grouping/spacing and mobile form/input/action geometry, including outlined Logout; preserve existing credential and privacy behaviour. Reconcile historical04 provider paragraph to already-approved release privacy authority, without an independent copy edit here.
6. **§3.7:** approve removal of nested cards, compact inline theme group and deterministic five-column swatch layout/large-text3+2 fallback with independent selection ring; all preferences/accents unchanged.
7. **§3.8:** approve neutral local setup/diagnostic actions, larger/wrapping controls/time chips, explicit pressed/focus/disabled treatment and the exact log-hint copy-table substitution. Keep all permission/location/log semantics.
8. **§4.1–2, §4.8, §5:** approve mobile text/target/focus updates, quiet initial loading rather than an elevated ornamental spinner card, no blocking overlay on cache refresh, the exact toast substitution and reduced-motion presentation while preserving timing/events.
9. **§1, §3 auth gating:** approve `Einstellungen` target on existing gates, and distinguish pending authentication from signed-out without adding a route. If the existing model cannot expose that distinction via current bindings, spec must resolve its UI-only state projection first; no new auth pipeline is authorised.

No changes to scraping requirements, feature semantics, release identities or desktop addendum are requested. The768 mobile width is a responsive contract, not approval to change the iOS iPhone-only device-family setting in07.

## Open questions

1. **Approve the Day Tray presentation amendments above?** Recommendation: approve them as one coherent mobile extension while keeping all behavioural parity. Exact04 recipes otherwise prevent a lawful redesign. Copy rows can be separately declined, retaining those exact current strings.
2. **Does “Anmeldung” mean a new standalone surface?** No such route or screen was found. Recommendation: the shared gates + existing two credential forms satisfy this rework; no onboarding/provider chooser.
3. **Does “Mitteilungen” mean renaming or adding an inbox?** Recommendation: neither; keep `Benachrichtigungen` and its precise settings scope. An inbox needs a new product requirement, not a visual extrapolation.
4. **Should768 imply native iPad support?** `07-release.md` §1 specifies iPhone-only. Recommendation:768 is Android tablet/resizable/mobile-host layout evidence only; do not alter iOS device-family/build settings in this change. If iPad distribution is wanted, obtain separate release approval.
5. **Should narrow desktop adopt the new mobile geometry?** Recommendation: shared phone-column composition may adopt it only through explicit host-safe presentation selection; retain desktop keyboard/focus/async behaviour and unsupported-notifications hiding, preserve all APIs, and require before/after narrow-desktop evidence. Wide Desktop Ledger remains pixel-regression protected. Spec must settle host selection before shared mobile primitives are changed.
6. **How are auth-pending and background-refresh states exposed to mobile views?** Current screens often consume booleans or broad busy flags; the requirements distinguish pending versus signed out and cached refresh. Recommendation: project existing authoritative model state through the UI boundary, without extra network calls or timing heuristics. If that requires an API change conflicting with desktop consumers, pause for explicit approval rather than weaken this contract.
7. **Can current runtime/mobile captures be provided before implementation review?** Historical images do not prove present safe-area, input focus, populated orders or receipts. Recommendation: capture isolated synthetic fixtures on both hosts; treat capture/tool access as an evidence blocker, never as visual approval. Skill reference-file access was also denied; spec/reviewer should load those references when permissions allow and reconcile any stricter craft requirement before implementation.

## Evidence plan

Evidence is future work, not claimed present proof. Use only synthetic/sanitized data and the existing isolated evidence/demo facilities: never sign into a real account, inspect a credential store, record real usernames or commit live responses. All widths below are **logical viewport widths**, with physical image scale/platform/safe-area noted in an accompanying capture index. Keep baseline and after images at identical logical size, fixture, selected date and scroll position.

### Width and theme matrix

- **360 and390, iOS glass + Android flat, Orange light and Orange dark:** every tab, each credential form, all three auth gates, Darstellung, Benachrichtigungen. For each record screen show populated default, long content, empty, initial loading, cached-refresh and error with cache. Native iOS minimum test may require host viewport simulation; record simulation clearly, not as a claim of a360-point device model.
- **768 mobile host, both dialects, Orange light/dark:** same surfaces, prove560 column/440 form, bottom navigation rather than desktop rail, wrapping controls and no invented split view. On iOS use a viewport harness without changing iPhone-only shipping configuration.
- **All five accents × light/dark × glass/flat:** capture a component-state board at390 containing primary action, danger action, pending card/check, selected filter, status inks, inactive navigation and focus on each. Add real Menüs pending+mixed-change dock and Darstellung screenshots for Gold light and Beere dark (bright/fallback stress cases); capture appearance for every accent so swatch order/light-mode fill can be compared. Assert contrast numerically on actual composited surfaces:4.5:1 for all text,3:1 for control/selection/focus boundaries, for resting and pressed states. A screenshot alone cannot prove these ratios.
- **Narrow desktop360/390 and wide desktop768/1280, Orange light/dark:** before/after with same inputs/selections. Prove no broken APIs, input persistence, keyboard/focus or async regression; wide Desktop Ledger and capabilities remain unchanged. Existing `shell-390` gallery image is not this proof.

### Required state captures and interaction evidence

| Surface | Capture / interaction that proves the contract |
|---|---|
| Menüs | real synthetic subtitle rather than repeated category; four-line dish and long allergens; default, selected, ordered, pending-cancel, unavailable, cutoff; mixed all-days changes with longest dock label at360; last card fully revealed above pill/dock; operation phases with repeat taps rejected |
| Bestellungen | coming/past with long descriptions/counts; confirmed/unconfirmed; stacked confirm banner; cutoff cancel, in-flight cancel/hide-other-buttons, modal exact strings; existing selection mode zero/one/many; strip clears navigation; just before/at09:00 with model-controlled clock |
| Abrechnung | both-service/one-service auth; all three source filters/months; Gourmet multiline receipt, Ventopay without restaurant, large and negative values if supplied by fixture; subsidy present/absent; selected-context cache while refreshing and source error without fabricated totals |
| Einstellungen | both/one/neither authenticated; long synthetic account hint, all four mobile rows, privacy modal; desktop narrow still hides unsupported destination |
| Both forms | empty/prefilled masked, keyboard username/password focus, selected text/caret, Save busy, blank-input failure, login failure, success and session/logout; save visible with keyboard; no secret text captured or copied |
| Anmeldung | pending spinner versus signed-out versus failed auth; reachable `Einstellungen`; Billing with either service authenticated has no global gate |
| Darstellung | System/Hell/Dunkel checked, all accents,360 default five swatches and large-text3+2 fallback; OS appearance change; selected/focus/pressed clearly independent |
| Benachrichtigungen | reminder off/on/permission-denied, earliest/latest selected time, horizontally revealed saved selection; Android hint and system-settings return; location absent/busy/set/permission/failure; log inactive12/24, active with count, expired/send/discard and mail failure; no coordinates or real diagnostic contents |
| Shared overlays | toast at safe top without obscuring system status area, broadcast with44×44 dismiss, modal above navigation with readable long error and visible actions; background tap resolves once; reduced-motion alternative retains lifetime |

At360 additionally capture large text (at least150% effective scale) and a short usable height with keyboard; at390 verify normal and reduced motion; at768 verify rotation/resize preserves data, pending state and unsaved input. Record touch-target bounds, accessibility label/state inspection and keyboard/Switch Control traversal, not just screenshots. Contrast and hit-area assertions must exercise rendered/mobile presentation values rather than only legacy desktop tokens.

Before an implementation is called complete, the owning dispatcher must run the repository's exact `make check`, obtain the one review gate and quote actual output/verdict; this design brief supplies no substitute command or passing result. Review should compare captures against this brief and each approved requirement amendment, with remaining questions explicitly resolved—not assume approval from the existence of this file.
