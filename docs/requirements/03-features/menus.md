# Menu browsing feature

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

The Menus tab is the app's primary screen: it shows the cafeteria menu for one selected
day at a time, lets the user navigate between days, select menus to order, mark ordered
menus for cancellation, and submit all changes with a single action.

Cross-references (do not restate here):

- HTTP request/response byte-level specs for `getMenus`, `addToCart`, `confirmOrders`,
  `cancelOrders` and HTML parsing (selectors, regexes, pagination): **01-gourmet-scraping**
  (v1: src/app/src-rn/api/gourmetApi.ts, gourmetClient.ts, gourmetParser.ts).
- Order list model and Orders tab: **03-features/orders** (v1: src/app/src-rn/store/orderStore.ts).
- Cache persistence conventions (AsyncStorage keys, serialization): **03-features/caching**.
- New-menu detection/fingerprints/toast internals: **03-features/notifications-new-menu**.
- Analytics event transport (`trackSignal`): **03-features/analytics**.
- Theming, colors, glass/flat styles, exact layout values: **04-ui-ux**, **03-features/themes**.

---

## 1. Data model

(v1: src/app/src-rn/types/menu.ts:1-24)

```
GourmetMenuItem {
  id: string            // menu ID from data-id; per-CATEGORY, not per-item (all MENÜ I items of a day share one ID)
  day: Date             // local calendar date (midnight local time), parsed from data-date "MM-dd-yyyy"
  title: string         // category headline text, e.g. "MENÜ I"
  subtitle: string      // dish description
  allergens: string[]   // allergen letters, e.g. ["A", "G", "L"]
  available: boolean    // true iff the availability checkbox exists in the HTML
  ordered: boolean      // true iff the availability checkbox exists AND is checked
  category: GourmetMenuCategory  // derived from title
  price: string         // opaque display string from the site ('' when absent)
}

enum GourmetMenuCategory {   // enum VALUES are the display strings
  Menu1        = 'MENÜ I',
  Menu2        = 'MENÜ II',
  Menu3        = 'MENÜ III',
  SoupAndSalad = 'SUPPE & SALAT',
  Unknown      = 'UNKNOWN',
}

GourmetDayMenu { date: Date; items: GourmetMenuItem[] }
```

Field extraction (selectors, category regexes) is owned by 01-gourmet-scraping
(v1: src/app/src-rn/api/gourmetParser.ts:94-144 `parseMenuItems`; `detectCategory` at
73-92 with `SOUP_SALAD_PATTERN` at line 8).

**Discrepancy — CLAUDE.md vs code (code wins):** CLAUDE.md's "Menu Data Extraction"
table lists only Menu ID / Day / Title / Subtitle / Allergens / Available. The code
additionally parses `ordered` (checkbox present AND checked, v1:
src/app/src-rn/api/gourmetParser.ts:125) and `price` (`.price span` text, v1:
src/app/src-rn/api/gourmetParser.ts:128). Both fields are required by this feature.

### Composite item key

Because menu IDs are per-category, items are identified everywhere by the composite key
`"{menuId}|{localDateKey(day)}"` where `localDateKey` formats the **local** date
components as `YYYY-MM-DD` (zero-padded, no timezone conversion — explicitly NOT
`toISOString()`, which would shift dates in CET/CEST) (v1:
src/app/src-rn/utils/dateUtils.ts:45-50, src/app/src-rn/store/menuStore.ts:56-58).

---

## 2. Menu state

(v1: src/app/src-rn/store/menuStore.ts:31-69)

| Field | Type | Initial value |
|---|---|---|
| `items` | `GourmetMenuItem[]` | `[]` |
| `lastFetched` | epoch-ms timestamp or null | `null` (in-memory only; NOT persisted) |
| `loading` | boolean | `false` |
| `refreshing` | boolean | `false` |
| `error` | string or null | `null` |
| `selectedDate` | Date | now (device local time) |
| `pendingOrders` | Set of composite keys | empty |
| `pendingCancellations` | Set of composite keys | empty |
| `orderProgress` | `'adding' \| 'confirming' \| 'cancelling' \| 'refreshing' \| null` | `null` |

Derived getters (v1: src/app/src-rn/store/menuStore.ts:302-328):

- `getAvailableDates()` — unique dates of `items`, deduplicated by `localDateKey`
  (the first-encountered `Date` object per day is kept), sorted ascending.
- `getMenusForDate(date)` — items whose `day` is the same local calendar day
  (year+month+day equality; v1: src/app/src-rn/utils/dateUtils.ts:55-61).
- `getDayMenus()` — all available dates each paired with their items. Exposed and
  unit-tested but has no production consumer in v1 (only `getMenusForDate` is used by
  the screen); optional in v2.
- `getPendingCount()` = `pendingOrders.size + pendingCancellations.size`.
- `getPendingCancellationCount()` = `pendingCancellations.size`.

---

## 3. Caching and refresh behavior

Two distinct fetch paths exist; which one runs depends on whether cached items are
already loaded (§8).

### 3.1 Persistent cache

- Storage key: `'menus_items'` (v1: src/app/src-rn/store/menuStore.ts:10). Items are
  serialized as JSON with `day` converted to an ISO-8601 string; deserialization
  restores `Date` objects (v1: src/app/src-rn/store/menuStore.ts:14-27).
- `loadCachedMenus()`: reads the key; missing entry is a no-op; a corrupt/unparsable
  entry is **deleted** from storage and ignored (v1: src/app/src-rn/store/menuStore.ts:71-80).
  Loading the cache does NOT set `lastFetched` — so a full fetch is never suppressed by
  cache-file age, only by an in-session fetch.

### 3.2 Full fetch — `fetchMenus(force = false)`

(v1: src/app/src-rn/store/menuStore.ts:82-110)

1. Abort if `loading` is already true (re-entrancy guard).
2. Abort if `!force && lastFetched != null && now - lastFetched < MENU_CACHE_VALIDITY_MS`,
   where `MENU_CACHE_VALIDITY_MS = 4 * 60 * 60 * 1000` (4 hours) (v1:
   src/app/src-rn/utils/constants.ts:9).
3. Set `loading = true`, `error = null`.
4. Fetch all menu items via the Gourmet API's paginated menus scrape
   (see 01-gourmet-scraping; v1: src/app/src-rn/api/gourmetApi.ts:121-155).
5. On success: replace `items` entirely, set `lastFetched = now`, `loading = false`,
   persist to cache.
6. **Auto-select nearest date**: after a successful fetch, if the current `selectedDate`
   has no menus (no available date with equal `toDateString()`, i.e. same local calendar
   day), select `findNearestDate(dates, selectedDate)`, falling back to the first
   available date (v1: src/app/src-rn/store/menuStore.ts:98-105). `findNearestDate`
   returns the closest date on-or-after the target day, else the closest date before it,
   else null for an empty list (v1: src/app/src-rn/utils/dateUtils.ts:119-148).
7. On failure: `error = err.message` if available, else the fallback
   `'Menüs konnten nicht geladen werden'`; `loading = false`; items unchanged.

### 3.3 Background availability refresh — `refreshAvailability()`

(v1: src/app/src-rn/store/menuStore.ts:112-157)

Used when cached items are already on screen: refetches everything but merges only the
volatile fields, so the visible list never flickers or loses static data.

1. Abort if `refreshing` is already true or `items` is empty.
2. Set `refreshing = true` (renders the non-blocking "Aktualisiere..." banner, §7).
   No `loading` spinner. `error` is NOT touched by this path.
3. Fetch fresh items (same API call as full fetch).
4. Merge by composite key `"{id}|{localDateKey(day)}"`:
   - For every cached item with a matching fresh item: keep all cached fields, overwrite
     only `available` and `ordered` from the fresh item.
   - Fresh items with no cached counterpart are **appended** to the list.
   - Cached items with no fresh counterpart are kept as-is (not removed).
   - **Key-collision semantics (v1 quirk, replicate exactly):** keys are per
     category+day (§1), so multiple items routinely share one key. The fresh map keeps
     only the LAST fresh item per key (`Map.set` overwrites); on merge, only the FIRST
     cached item per key receives that fresh item's `available`/`ordered` flags — the
     fresh entry is deleted from the map after the first match, so later cached
     duplicates keep their stale flags; and at most ONE fresh item per key can be
     appended (a brand-new day/category appearing mid-refresh contributes only its
     last-parsed dish until the next full fetch)
     (v1: src/app/src-rn/store/menuStore.ts:127-146).
5. Minimum banner visibility: results are not applied before 800 ms have elapsed since
   the refresh started (v1: src/app/src-rn/store/menuStore.ts:120,148-149) — the banner
   must be noticeable.
6. On success: set merged `items`, `lastFetched = now`, `refreshing = false`, persist to
   cache. On failure: **silent** — after the same 800 ms minimum, set
   `refreshing = false`; cached items and `error` remain unchanged (v1:
   src/app/src-rn/store/menuStore.ts:152-156; test: src/app/src-rn/__tests__/store/menuStore.test.ts:611-621).

---

## 4. Day navigation

- Available days = `getAvailableDates()` (§2). The screen shows exactly one day at a time.
- Date equality throughout the UI is same-local-calendar-day (v1 uses `toDateString()`
  comparison on screen, `isSameDay` in the store — both mean local Y/M/D equality).
- The day navigation bar is hidden entirely when there are no available dates (v1:
  src/app/app/(tabs)/index.tsx:376).
- **`selectedDate` may legitimately be absent from the available dates.** Auto-selection
  of the nearest date happens ONLY inside `fetchMenus` (§3.2); `refreshAvailability`
  (§3.3) never adjusts `selectedDate`. So on a cached startup where "now" has no menus
  (e.g. weekend with cached weekday menus), the screen stays on an empty day with
  `currentIndex = -1`: the DayNavigator indicator renders `0 / {total}`, the back arrow
  is disabled, the forward arrow selects `dates[0]`, and in the swipe gesture a
  right-drag rubber-bands while a committed left swipe also jumps to `dates[0]`
  (v1: src/app/src-rn/store/menuStore.ts:98-105,116-157;
  src/app/app/(tabs)/index.tsx:195-197,216,240;
  src/app/src-rn/components/DayNavigator.tsx:17-31,44,54-56).

### 4.1 Top day-navigator bar (mobile)

(v1: src/app/src-rn/components/DayNavigator.tsx)

- Left/right arrow buttons step to the previous/next date in the sorted available-dates
  list; each arrow is disabled (and rendered at 0.3 opacity) at the corresponding end of
  the list (v1: DayNavigator.tsx:21-31,44,61).
- Center shows the selected date formatted with locale `de-AT` as
  `{ weekday: 'short', month: 'short', day: 'numeric' }` (e.g. "Mo., 10. Feb.") (v1:
  src/app/src-rn/utils/dateUtils.ts:32-38) and, below it, the position indicator
  `"{index+1} / {total}"` (v1: DayNavigator.tsx:50-57).
- Tapping the center jumps to `findNearestDate(dates, now)` — i.e. "go to today", or the
  nearest menu day if today has none (v1: DayNavigator.tsx:33-38).
- v1 mechanism: on iOS the bar sits on a blur background; on Android on a solid surface
  (v1: DayNavigator.tsx:69-81). Visual treatment is owned by 04-ui-ux.

### 4.2 Horizontal swipe between days (mobile)

(v1: src/app/app/(tabs)/index.tsx:192-266)

The menu list content pane responds to horizontal drags:

- Gesture activation: `|dx| > |dy| && |dx| > 10` (px).
- Commit threshold: `50` px (`SWIPE_THRESHOLD`, v1: index.tsx:204). Swipe right
  (dx > 50) goes to the previous day; swipe left (dx < −50) goes to the next day; a
  release below threshold springs the content back to rest.
- Edge resistance: when already at the first date and dragging right, or at the last
  date and dragging left, the drag translation is multiplied by `0.3` (rubber-band)
  and release cannot change the day.
- v1 mechanism (animation fidelity, optional in v2): on commit, content animates off-screen
  over 180 ms in the swipe direction, the date switches, then content springs in from the
  opposite side (spring tension 65, friction 11) (v1: index.tsx:224-262).
- Pending selections (§6) are keyed by menu+date, so they persist across day navigation.

### 4.3 Wide-layout date sidebar — **Dropped in v2**

On desktop-wide layouts v1 replaces the DayNavigator with a left sidebar
(`DateListPanel`, header text "Termine") listing all dates as tappable rows (v1:
src/app/src-rn/components/DateListPanel.tsx; layout switch v1:
src/app/app/(tabs)/index.tsx:346-371 via `useDesktopLayout`). Desktop/web targets are
dropped in v2; only the mobile DayNavigator + swipe behavior carries over. Likewise all
`isCompactDesktop` style variants in these components are dropped.

---

## 5. Category grouping and display order

(v1: src/app/app/(tabs)/index.tsx:47-53,171-174,315-334)

- The selected day's items are grouped by `category` and rendered in this fixed order:

  1. `MENÜ I`
  2. `MENÜ II`
  3. `MENÜ III`
  4. `SUPPE & SALAT`
  5. `UNKNOWN`

- Empty groups are omitted.
- Each group renders a heading with the category's display string, **except**
  `SUPPE & SALAT`, whose heading is suppressed (v1: index.tsx:317-319). Note the
  `UNKNOWN` group does render a literal "UNKNOWN" heading in v1.
- Within a group, items keep the order in which the parser produced them (no re-sorting).
- Card list key: `"{item.id}-{formatGourmetDate(item.day)}"` where `formatGourmetDate`
  is `MM-dd-yyyy` (v1: index.tsx:324; src/app/src-rn/utils/dateUtils.ts:4-9).

### Ordered-state display is per category

A card displays as "ordered" if `item.ordered` is true **or** the item's category is in
the ordered-categories set for the selected day. That set contains (a) the category of
every displayed item with `ordered === true` and (b) `order.title` (cast to a category)
of every entry in the order store whose date is the same local calendar day as the
selected date (v1: index.tsx:157-169,321). Because menu IDs — and Gourmet orders — are
per category, ordering "MENÜ I" marks every MENÜ I card of that day as ordered.

---

## 6. Selecting, cancelling, and submitting (cart interaction)

### 6.1 Menu card states and interactivity

(v1: src/app/src-rn/components/MenuCard.tsx:17-98; design intent:
docs/plans/2026-02-21-menu-reordering-design.md)

Inputs per card: `item`, `isSelected` (in `pendingOrders`), `ordered` (per §5),
`isPendingCancel` (in `pendingCancellations`), plus the cutoff flag
`cutoff = isOrderingCutoff(item.day)` (§6.2).

Tappability: `canInteract = ordered || (item.available && !cutoff)`
(v1: MenuCard.tsx:25). Non-interactive cards are disabled.

| State (evaluation order for style) | Visual | Badge text |
|---|---|---|
| Ordered, not pending cancel | success/green surface | `Bestellt` |
| Pending cancel | 0.55 opacity, dashed border, strikethrough text | `Wird storniert` |
| Selected (pending new order) | primary/blue surface, white text, white circular checkmark (✓) top-right | — |
| Not ordered and not available | 0.5 opacity, disabled | `Ausverkauft` |
| Available, not ordered, past cutoff | 0.5 opacity, disabled | `Geschlossen` |
| Available, orderable | normal | — |

Badge display conditions exactly (v1: MenuCard.tsx:39-60):
`Wird storniert` iff `isPendingCancel`; `Bestellt` iff `ordered && !isPendingCancel`;
`Ausverkauft` iff `!ordered && !item.available`; `Geschlossen` iff
`cutoff && !ordered && item.available`.

Card content (v1: MenuCard.tsx:61-96):
- `item.subtitle`, max 4 lines (the `title` is NOT shown on the card — the group heading
  serves that purpose).
- Bottom row, left: allergens as `Allergene: {allergens.join(', ')}`, single line;
  rendered as empty string when the list is empty.
- Bottom row, right: `item.price` verbatim.

Tapping a card calls `togglePendingOrder(item.id, item.day)` (§6.3).

### 6.2 Ordering cutoff rule

(v1: src/app/src-rn/utils/dateUtils.ts:66-99)

`isOrderingCutoff(menuDate)`:
- "today" = the current date in the **Europe/Vienna** timezone (v1: dateUtils.ts:80-86).
- `menuDate < today` (past day) → blocked (`true`).
- `menuDate` after today → never blocked (`false`).
- `menuDate` is today → blocked iff current Vienna wall-clock time ≥ **09:00**
  (`viennaMinutes() >= 9 * 60`, v1: dateUtils.ts:98).

User-facing cutoff message (exact string, v1: src/app/src-rn/store/menuStore.ts:11):

```
Bestellung für heute geschlossen (Bestellschluss 9:00)
```

**Discrepancy — design doc vs code (code wins):** the reordering design doc states the
cutoff as 12:30 (docs/plans/2026-02-21-menu-reordering-design.md:54) and the impl plan
shows a 12:30 message (docs/plans/2026-02-21-menu-reordering-impl.md:377). The shipped
code uses 09:00 Europe/Vienna everywhere.

### 6.3 Pending-change model — `togglePendingOrder(menuId, date)`

(v1: src/app/src-rn/store/menuStore.ts:161-185; tests: menuStore.test.ts:186-263)

- Key = `"{menuId}|{localDateKey(date)}"`.
- Look up the item by `id === menuId && localDateKey(day) === localDateKey(date)`.
- If the item is currently `ordered` (default `false` when not found): toggle the key in
  `pendingCancellations`. Otherwise: toggle the key in `pendingOrders`.
- Toggling means add-if-absent / remove-if-present. Multiple selections per day are
  allowed (e.g. MENÜ I + SUPPE & SALAT, or several categories at once).
- `clearPendingChanges()` empties both sets (v1: menuStore.ts:187). (The design doc
  calls this `clearPendingOrders`; code name wins.)

### 6.4 Submit action button (FAB)

(v1: src/app/app/(tabs)/index.tsx:146-154,391-395)

- Visible iff `getPendingCount() > 0 && orderProgress === null`; pressing it calls
  `submitOrders()`.
- Label (exact strings), with `pendingCount = orders+cancels`,
  `cancellationCount = cancels`, `newOrderCount = pendingCount - cancellationCount`:
  - both kinds pending → `Änderungen bestätigen ({pendingCount})`
  - only cancellations → `Stornieren ({cancellationCount})`
  - otherwise → `Bestellen ({newOrderCount})`

### 6.5 Submit flow — `submitOrders()`

(v1: src/app/src-rn/store/menuStore.ts:189-300; tests: menuStore.test.ts:265-473)

1. **No-op** if both pending sets are empty.
2. **Resolve cancellations → positionIds.** For each key in `pendingCancellations`:
   find the menu item (by id + date key) to get its `category`; then find the order in
   the order store with `order.title === menuItem.category` and
   `localDateKey(order.date) === dateStr`; collect `order.positionId`. Unresolvable
   entries are skipped with a console warning
   `"Could not resolve all cancellations: {resolved}/{total}"` (v1: menuStore.ts:196-219).
3. **Resolve new orders.** Each `pendingOrders` key is parsed back into
   `{ menuId, date }`, the date reconstructed from the `YYYY-MM-DD` key parts as a local
   Date (v1: menuStore.ts:222-226).
4. **Cutoff filter.** New orders whose date fails `isOrderingCutoff` are dropped from
   the submission; cancellations are never cutoff-filtered. If at least one new order
   was blocked AND no new orders remain AND no cancellation resolved to a positionId:
   set `error` to the cutoff message and return **without clearing the pending sets**
   (v1: menuStore.ts:228-234).

   **Discrepancy — design/impl doc vs code (code wins):** the plan documents block the
   entire submit if any item is past cutoff (design.md:54, impl.md:374-379); the code
   instead filters blocked items and proceeds with the rest.
5. **Optimistic update.** Every item whose key is in `pendingCancellations` gets
   `ordered = false`; every item whose key is in `pendingOrders` gets `ordered = true`
   (note: this includes cutoff-blocked and unresolvable entries — the forced refresh in
   step 8 corrects them). Both pending sets are cleared. `error` is set to the cutoff
   message if any new order was blocked, else `null` (v1: menuStore.ts:236-254).
6. **Cancel** (only if any positionIds resolved): `orderProgress = 'cancelling'`, then
   one batched `cancelOrders(positionIds)` API call (single edit-mode enter/exit; see
   01-gourmet-scraping; v1: src/app/src-rn/api/gourmetApi.ts:240-301).

   **Discrepancy — impl doc vs code (code wins):** the impl plan loops
   `orderStore.cancelOrder(positionId)` per item (impl.md:402-408); shipped code batches
   all IDs into one `api.cancelOrders([...])` call (v1: menuStore.ts:257-261; test:
   menuStore.test.ts:354 expects `cancelOrders(['P1'])`).
7. **Add + confirm** (only if allowed new orders remain): `orderProgress = 'adding'`,
   call `addToCart(allowedNewOrders)` (list of `{ menuId, date }`; grouping by date and
   the JSON body are owned by 01-gourmet-scraping); then `orderProgress = 'confirming'`,
   call `confirmOrders()` (v1: menuStore.ts:263-270).
8. **Refresh:** `orderProgress = 'refreshing'`; await the order store's `fetchOrders()`,
   then `fetchMenus(true)` (force) (v1: menuStore.ts:272-275).
9. Emit analytics event `order.submitted` with string properties
   `orderedCount` = `String(allowedNewOrders.length)` — the cutoff-FILTERED count
   actually submitted, not the raw pending size — and `cancelledCount` =
   `String(positionIds.length)` — RESOLVED cancellations only. The event fires on every
   non-aborted submit, **including when both counts are `"0"`** (e.g. all cancellations
   unresolvable — the flow still runs the refresh and emits `0`/`0`)
   (v1: menuStore.ts:277-280; filtering at :229-234, resolution at :196-219).
10. Finish: `orderProgress = null`; `error` is **always overwritten** here — the cutoff
    message if step 4 blocked anything, otherwise explicitly `null`. Note this also
    wipes any error message the step-8 refresh may have set on failure (`fetchMenus`
    and `fetchOrders` swallow their own errors and never throw, so a failed
    post-submit refresh ends with `error = null`)
    (v1: menuStore.ts:282-286 — unconditional `error: hasCutoffBlocked ? … : null`;
    menuStore.ts:106-109; orderStore.ts:87-90).
11. **On any failure in steps 6-7** (cancel, add, confirm — step 8 cannot throw, see
    step 10): `error = err.message` (fallback
    `'Bestellung konnte nicht aufgegeben werden'`), `orderProgress = null`, and the
    optimistic update is reverted by fetching fresh items (`getMenus`), replacing
    `items`, updating `lastFetched`, and persisting the cache; if that revert fetch also
    fails, the optimistic state is silently kept (v1: menuStore.ts:287-299).
    Note the accepted risk from the design doc: if cancellation succeeds but the new
    order fails, the old order is lost — there is no atomic swap in the external API
    (docs/plans/2026-02-21-menu-reordering-design.md:65-67).

### 6.6 Order progress banner

While `orderProgress` is non-null the FAB is hidden and a progress banner shows a
spinner plus the step label (exact strings, v1: src/app/app/(tabs)/index.tsx:40-45):

| `orderProgress` | Label |
|---|---|
| `adding` | `Wird in den Warenkorb gelegt...` |
| `confirming` | `Bestellung wird bestätigt...` |
| `cancelling` | `Bestellung wird storniert...` |
| `refreshing` | `Menüs werden aktualisiert...` |

The flow is intentionally non-blocking: the list stays interactive underneath.

---

## 7. Loading, error, and empty states

(v1: src/app/app/(tabs)/index.tsx:268-343)

Auth gating (auth statuses owned by 03-features/settings; v1:
src/app/src-rn/store/authStore.ts:10):

- `idle` or `loading` → full-screen loading indicator only.
- `error` or `no_credentials` → centered message `Nicht angemeldet` with hint
  `Gehe zu Einstellungen, um Zugangsdaten einzugeben`; no menu UI.
- `authenticated` → normal screen.

Within the normal screen (top-to-bottom, all may coexist):

- Refresh banner: spinner + `Aktualisiere...`, shown iff `refreshing && !orderProgress`.
- Order progress banner (§6.6), shown iff `orderProgress != null`.
- Full loading overlay, shown iff `loading && !orderProgress` (i.e. only during a full
  fetch, never during background refresh or submit).
- Error banner showing the store's `error` string verbatim, shown iff `error != null`.
  It does not auto-dismiss; it is cleared by the next `fetchMenus` (which sets
  `error = null` on start).
- Empty state: `Keine Menüs verfügbar`, shown iff the grouped list is empty and
  `!loading`.

---

## 8. Screen refresh lifecycle

(v1: src/app/app/(tabs)/index.tsx:87-138)

A refresh is triggered (a) every time the Menus tab gains focus and (b) whenever auth
status becomes `authenticated`. Each trigger:

1. On the focus trigger, emit analytics `screen.viewed` with `{ screen: 'menus' }`
   **before any auth check — the event fires on every tab focus, even unauthenticated**
   (v1: index.tsx:127-129 — `trackSignal` precedes `triggerRefresh`).
2. Abort the remaining steps unless auth status is `authenticated` (the gate lives
   inside the refresh helper, v1: index.tsx:88-89).
3. Load cached menus AND cached orders in parallel (instant display; failures ignored).
4. Then: if the menu store now has any items → `refreshAvailability()` (§3.3, banner,
   merge); otherwise → `fetchMenus()` (§3.2, spinner, subject to the 4-hour validity
   guard).
5. Concurrently start the order store's `fetchOrders()` (needed for the per-category
   ordered display, §5, and cancellation resolution, §6.5).
6. After the menu refresh resolves, run the new-menu detection hook: compute
   fingerprints of current items, compare against stored known menus, and if new/changed
   menus are detected and the notification flag is unset, show the in-app "new menus"
   toast; then persist current fingerprints as known and reset the flag. All errors here
   are swallowed. **The entire detection/acknowledge sequence is skipped when the store
   has zero items after the refresh** — this guard prevents a failed/empty refresh from
   persisting an empty fingerprint set, which would make every menu look "new" on the
   next refresh (v1: index.tsx:102-103). (Detection algorithm, storage keys, and toast
   component are owned by **03-features/notifications-new-menu**; trigger sequence
   v1: index.tsx:100-120.)

---

## 9. Dropped in v2

- Wide/desktop layout branch of the Menus screen: `DateListPanel` sidebar, desktop FAB
  variant, `useDesktopLayout`, and all `isCompactDesktop` style forks
  (v1: src/app/app/(tabs)/index.tsx:346-371, src/app/src-rn/components/DateListPanel.tsx,
  src/app/src-rn/hooks/useDesktopLayout.ts).
- Web-specific blur fallback in the day navigator (`AdaptiveBlurView.web`); keep the
  native iOS blur / Android solid-surface split or the v2 platform-native equivalent.

---

## 10. Summary of code-vs-docs discrepancies (code wins)

1. Ordering cutoff is **09:00 Europe/Vienna**, not 12:30 as in the reordering design/impl
   docs (§6.2).
2. `submitOrders` filters cutoff-blocked new orders and proceeds with the rest
   (cancellations always proceed); the plans described blocking the whole submit (§6.5
   step 4).
3. Cancellations are submitted as **one batched** `cancelOrders(positionIds)` call, not a
   per-position loop through the order store (§6.5 step 6).
4. The store method is named `clearPendingChanges`, not `clearPendingOrders` (§6.3).
5. CLAUDE.md's menu extraction table omits the `ordered` and `price` fields that the
   code parses and this feature requires (§1).
