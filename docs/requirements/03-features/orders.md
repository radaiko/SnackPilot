# Ordering & cancellation feature

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

Covers the full order lifecycle: building a pending "cart" of menu selections, submitting
orders, confirming them, listing ordered menus with approval states, and cancelling orders
(both from the Menus screen batch flow and the Orders screen single-order flow).

**Related docs — do not duplicate:**

- `01-gourmet-scraping.md` — byte-level HTTP specs (base URL, headers, `multipart/form-data`
  encoding, cookie handling, CSRF token extraction, session re-login, HTML parsing selectors).
  This doc states operation *sequences* and *field names* because they are feature behavior;
  the wire encoding is specified there.
- `03-features/menus.md` — the Menus screen where selections are made (menu cards, checkboxes,
  day navigation).
- `03-features/caching.md` — AsyncStorage cache mechanics shared by menu/order/billing stores.
- `03-features/notifications-daily-reminder.md`, `notifications-cancel-reminder.md`,
  `notifications-location.md` — the reminder hooks triggered after an orders fetch.
- `03-features/analytics.md` — `trackSignal` semantics.
- `04-ui-ux.md` — theming, surfaces, dialog system, loading overlay.
- `03-features/demo-mode.md` — demo API that substitutes the real Gourmet API.

---

## 1. Domain model

An ordered menu ("order") as parsed from the Gourmet orders page
(v1: src/app/src-rn/types/order.ts:1-8):

| Field | Type | Meaning |
|---|---|---|
| `positionId` | string | Unique per order line; key for cancellation |
| `eatingCycleId` | string | Opaque server value; parsed for completeness but **unused** — cancellation re-extracts the fresh value from the cancel form (§6.1), never sends this stored field (v1: src/app/src-rn/api/gourmetApi.ts:274-285) |
| `date` | date-time | Day the menu is for (parsed from `dd.MM.yyyy HH:mm:ss`; time is `00:00:00` in practice) |
| `title` | string | Category label as shown on the orders page, e.g. `MENÜ I`, `SUPPE & SALAT` |
| `subtitle` | string | Dish description, e.g. `Wiener Schnitzel mit Kartoffelsalat` |
| `approved` | bool | Whether the order is confirmed (see §5) |

Note: v1 also declares `OrderDateGroup { date, orders[] }` (v1: src/app/src-rn/types/order.ts:10-13)
but it is referenced nowhere — dead code, do not carry to v2.

The order's `date` is parsed from the hidden input value in format `dd.MM.yyyy HH:mm:ss`;
a missing time part defaults to `00:00:00` (v1: src/app/src-rn/utils/dateUtils.ts:22-27).
If the date input is missing entirely, v1 falls back to "now"
(v1: src/app/src-rn/api/gourmetParser.ts:189).

---

## 2. Fetching the ordered-menus list

Operation `getOrders` (v1: src/app/src-rn/api/gourmetApi.ts:160-169):

1. **GET** `https://alaclickneu.gourmet.at/bestellungen/` (constant `GOURMET_ORDERS_URL`
   = base + `/bestellungen`, with a trailing `/` appended at the call site;
   v1: src/app/src-rn/utils/constants.ts:4).
2. Check the response HTML for an active session (login marker regex — see
   `01-gourmet-scraping.md`). If not logged in and credentials are saved, re-login and
   **GET** the same URL again; if no credentials, throw `SessionExpiredError`
   (v1: src/app/src-rn/api/gourmetApi.ts:42-51).
3. Parse order items from the HTML.

Parsing summary (authoritative selector spec in `01-gourmet-scraping.md`;
v1: src/app/src-rn/api/gourmetParser.ts:157-197):

- Item selector: `div.order-item, div[class*="order-item"]`.
- Per item: `positionId` from `input[name="cp_PositionId"]` value — **skip the item if
  absent**; `eatingCycleId` from `input[name^="cp_EatingCycleId_"]` value (default `''`);
  date string from `input[name^="cp_Date_"]` value (default `''`).
- `title` = the *direct text nodes only* of the `.title` element, trimmed (the subtitle is a
  nested `div` inside `.title` and must be excluded); `subtitle` = `.subtitle` text, trimmed.
- `approved` — see §5.

> **Discrepancy (code wins):** CLAUDE.md documents the order item selector as plain
> `div.order-item`; the code additionally matches `div[class*="order-item"]`
> (v1: src/app/src-rn/api/gourmetParser.ts:161).

---

## 3. Cart building (pending selections)

There is no server-side "view cart" in v1. The cart is purely client state on the Menus
screen, held as two sets of keys (v1: src/app/src-rn/store/menuStore.ts:38-39):

- `pendingOrders` — menu selections to order. Key format: `"{menuId}|{YYYY-MM-DD}"` where
  the date part is the *local* date key (no UTC shift; v1: src/app/src-rn/utils/dateUtils.ts:45-50,
  key built at src/app/src-rn/store/menuStore.ts:56-58).
- `pendingCancellations` — already-ordered items toggled off, same key format.

Toggling a menu item (v1: src/app/src-rn/store/menuStore.ts:161-185): find the menu item by
`id` + local date key. If the item is currently `ordered`, the toggle adds/removes the key
in `pendingCancellations`; otherwise it adds/removes the key in `pendingOrders`. So a single
tap gesture means "order this" for unordered items and "cancel this" for ordered items.

`clearPendingChanges()` empties both sets (v1: src/app/src-rn/store/menuStore.ts:187).

Derived counts: `getPendingCount()` = size of both sets combined;
`getPendingCancellationCount()` = cancellations only
(v1: src/app/src-rn/store/menuStore.ts:326-328).

Submit affordance on the Menus screen: a floating action button shown only when
`pendingCount > 0` and no submission is in progress. Label logic
(v1: src/app/app/(tabs)/index.tsx:146-154):

- both new orders and cancellations pending → `Änderungen bestätigen ({pendingCount})`
- only cancellations → `Stornieren ({cancellationCount})`
- otherwise → `Bestellen ({newOrderCount})`

---

## 4. Order submission pipeline (`submitOrders`)

Triggered by the FAB on the Menus screen. Full behavior
(v1: src/app/src-rn/store/menuStore.ts:189-300):

### 4.1 Resolution

- No-op if both pending sets are empty.
- **Cancellations → positionIds:** for each `"{menuId}|{dateStr}"` key, find the menu item
  (by `id` + local date key), then find the order in the order store where
  `order.title === menuItem.category` **and** `localDateKey(order.date) === dateStr`.
  Collect its `positionId`. Unresolvable keys are silently skipped (a console warning
  `Could not resolve all cancellations: {resolved}/{total}` is logged)
  (v1: src/app/src-rn/store/menuStore.ts:196-219). This join works because the order's
  `title` field holds the category label (e.g. `MENÜ I`) — the same string as the menu
  item's `category`.
- **New orders:** each pending key is split back into `{ menuId, date }` (date reconstructed
  from `YYYY-MM-DD` as a local date) (v1: src/app/src-rn/store/menuStore.ts:222-226).

### 4.2 Ordering cutoff filter

New orders whose date is cutoff-blocked are filtered out; **cancellation submission is not
cutoff-filtered here** (the UI blocks cancellation via §7 instead)
(v1: src/app/src-rn/store/menuStore.ts:228-234).

Cutoff rule `isOrderingCutoff(menuDate)` (v1: src/app/src-rn/utils/dateUtils.ts:94-99):

- date before today (today computed in `Europe/Vienna`) → blocked
- date is today → blocked iff current Vienna time ≥ 09:00
- future date → never blocked

If *all* new orders were cutoff-blocked and **no cancellation resolved to a positionId**
(pending cancellations that failed §4.1 resolution do not count — the check is on the
resolved IDs, `cancellationPositionIds.length === 0`, v1: menuStore.ts:231), set the error
`Bestellung für heute geschlossen (Bestellschluss 9:00)` (exported constant
`ORDERING_CUTOFF_MESSAGE`, v1: src/app/src-rn/store/menuStore.ts:11) and abort — pending
sets are left untouched on this path (they are only cleared by the optimistic update,
§4.3).

### 4.3 Optimistic UI update

Before any network call (v1: src/app/src-rn/store/menuStore.ts:236-254):

- Every menu item matching a pending-cancellation key gets `ordered: false`; every item
  matching a pending-order key gets `ordered: true` (including cutoff-blocked ones — they
  are flipped optimistically too, then corrected by the refresh in step 4.4/3).
- Both pending sets are cleared.
- `error` is set to `ORDERING_CUTOFF_MESSAGE` if any new order was cutoff-blocked, else `null`.

The **orders list itself is not optimistically updated** — only menu items' `ordered` flags.

### 4.4 Network sequence

A non-blocking progress state `orderProgress` is exposed with values
`'cancelling' | 'adding' | 'confirming' | 'refreshing' | null`
(v1: src/app/src-rn/store/menuStore.ts:29). The Menus screen shows a banner with these exact
labels (v1: src/app/app/(tabs)/index.tsx:40-45):

| progress | banner text |
|---|---|
| `adding` | `Wird in den Warenkorb gelegt...` |
| `confirming` | `Bestellung wird bestätigt...` |
| `cancelling` | `Bestellung wird storniert...` |
| `refreshing` | `Menüs werden aktualisiert...` |

Steps, strictly in this order (v1: src/app/src-rn/store/menuStore.ts:256-286):

1. If there are cancellation positionIds: progress `cancelling`, run the cancellation
   operation (§6) with **all** positionIds in one batch (single edit-mode round trip).
2. If there are allowed new orders: progress `adding`, run add-to-cart (§4.5); then
   progress `confirming`, run confirm (§5.2). Note: after `AddToMenuesCart`, orders exist
   but are **unconfirmed** until the confirm step runs.
3. Progress `refreshing`: re-fetch the orders list (§2), then force-refresh menus.
4. Emit analytics `order.submitted` with properties
   `{ orderedCount: "<n>", cancelledCount: "<n>" }` (values are decimal strings).
5. Clear `orderProgress`; re-set `error` to `ORDERING_CUTOFF_MESSAGE` if step 4.2 had
   blocked items (the menu refresh clears `error`, so it must be restored), else leave `null`.

### 4.5 Add to cart operation

(v1: src/app/src-rn/api/gourmetApi.ts:176-208)

- Requires user info from login (`eaterId`, `shopModelId`, `staffGroupId` — extraction spec
  in `01-gourmet-scraping.md`); throws `Not logged in` otherwise.
- Group requested items by date, formatting each date as `MM-dd-yyyy`
  (v1: src/app/src-rn/utils/dateUtils.ts:4-9).
- **POST** JSON to `https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart`
  (v1: src/app/src-rn/utils/constants.ts:5) with body:

  ```json
  {
    "eaterId": "...",
    "shopModelId": "...",
    "staffgroupId": "...",
    "dates": [ { "date": "MM-dd-yyyy", "menuIds": ["..."] } ]
  }
  ```

  **The key is `staffgroupId` with a lowercase `g`** (it carries the `staffGroupId` value)
  (v1: src/app/src-rn/api/gourmetApi.ts:190-198). Headers/encoding per `01-gourmet-scraping.md`.
- Response shape `{ success: boolean, message?: string }`. If `success` is not true, throw
  `` `Add to cart failed: ${message || 'unknown error'}` ``
  (v1: src/app/src-rn/api/gourmetApi.ts:205-207).

### 4.6 Failure handling & revert

If any step of §4.4 throws (v1: src/app/src-rn/store/menuStore.ts:287-299):

- Set `error` to the thrown message, or fallback `Bestellung konnte nicht aufgegeben werden`
  for non-Error throws; clear `orderProgress`.
- **Revert the optimistic update** by re-fetching menus from the server and replacing the
  item list + persisted menu cache with the fresh data. If that revert fetch also fails,
  keep the optimistic state silently.
- Pending sets are NOT restored — the user's selections are lost on failure.

---

## 5. Approval states & confirmation

### 5.1 Approval detection

An order is `approved` (confirmed) iff its order-item element contains a descendant matching
`.fa-check` **or** `.checkmark` (v1: src/app/src-rn/api/gourmetParser.ts:180-184).

> **Discrepancy (code wins):** CLAUDE.md says approval is "Presence of `.confirmed` class or
> `fa fa-check` icon". The code comment states "no .confirmed class exists on the site" and
> instead checks `.fa-check` OR `.checkmark`. The recorded fixture shows both variants in use
> (`<span class="fa fa-check">` and `<span class="checkmark">`;
> v1: src/app/src-rn/__tests__/fixtures/gourmet/orders-page.html:42,65).

### 5.2 Confirm operation and edit-mode semantics

The orders page contains an edit-mode toggle form `form.form-toggleEditMode` with hidden
inputs `editMode`, `ufprt`, `__ncforminfo` (fixture:
v1: src/app/src-rn/__tests__/fixtures/gourmet/orders-page.html:26-31). Extraction defaults
`editMode` to `'True'` when the input is missing; missing `ufprt`/`__ncforminfo` is an error
(v1: src/app/src-rn/api/gourmetParser.ts:202-215).

**Inverted semantics:** the hidden `editMode` value `"False"` means the page currently IS in
edit mode; `"True"` means it is NOT (v1 comments: src/app/src-rn/api/gourmetApi.ts:221-222,
249). Posting the toggle form always sends back the **extracted current value** of
`editMode`, never a hardcoded literal.

> **Discrepancy (code wins):** CLAUDE.md describes the edit-mode toggle form data as
> `editMode=True + ufprt + __ncforminfo`. The code posts whatever value was extracted from
> the hidden input — `True` when entering edit mode, `False` when exiting
> (v1: src/app/src-rn/api/gourmetApi.ts:226-231, 253-259, 294-300).

Confirm operation `confirmOrders` (v1: src/app/src-rn/api/gourmetApi.ts:214-233):

1. **GET** `https://alaclickneu.gourmet.at/bestellungen/` (with session check / re-login
   re-fetch as in §2).
2. Extract the edit-mode form. If `editMode === 'False'` (page is in edit mode — i.e. there
   are unconfirmed orders), **POST** form data to `/bestellungen/` with exactly:
   `editMode={extracted value}`, `ufprt={extracted}`, `__ncforminfo={extracted}`.
   Exiting edit mode is what confirms the pending orders.
3. If `editMode === 'True'`, everything is already confirmed — do nothing.

### 5.3 Confirmation UX

Store action `confirmOrders` (v1: src/app/src-rn/store/orderStore.ts:93-106): sets
`loading` **and clears `error`** (`set({ loading: true, error: null })` at line 94 — a
stale error banner from a prior failed cancel is dismissed when the user taps
Bestätigen), calls the API confirm, emits analytics `order.confirmed`, then re-fetches
the orders list to reflect the confirmed state. On failure: `error` = thrown message or fallback
`Bestellungen konnten nicht bestätigt werden`; no orders re-fetch happens
(verified by test, v1: src/app/src-rn/__tests__/store/orderStore.test.ts:170-178).

Orders screen: when `getUnconfirmedCount() > 0` (count of orders with `approved === false`)
and the "upcoming" tab is active, show a warning banner:
`{n} unbestätigte Bestellung` with plural suffix `en` when n > 1, plus a `Bestätigen` button
(disabled while loading) that triggers the confirm action
(v1: src/app/app/(tabs)/orders.tsx:176-189).

---

## 6. Cancellation

### 6.1 Cancellation operation (edit-mode flow)

`cancelOrders(positionIds[])` — batch-capable; exact sequence
(v1: src/app/src-rn/api/gourmetApi.ts:240-301):

1. **GET** `https://alaclickneu.gourmet.at/bestellungen/` (session check / re-login re-fetch
   as in §2).
2. Extract the edit-mode form. If `editMode !== 'False'` (not currently in edit mode):
   - **POST** to `/bestellungen/`: `editMode={extracted}`, `ufprt={extracted}`,
     `__ncforminfo={extracted}` (enters edit mode).
   - **GET** `/bestellungen/` again — the POST response may not reflect the new state.
   - Re-extract; if `editMode` is still not `'False'`, throw `Failed to enter edit mode`.
3. For each `positionId`, in order:
   - Locate that order's cancel form in the current HTML: primary selector
     `form#form_{positionId}_cp`; fallback
     `form:has(input[name="cp_PositionId"][value="{positionId}"])`
     (v1: src/app/src-rn/api/gourmetParser.ts:221-250). Cancel forms only exist while in
     edit mode.
   - Extract from that form: `cp_EatingCycleId_{positionId}` value (default `''`),
     `cp_Date_{positionId}` value (default `''`), `ufprt`, `__ncforminfo` (both required,
     else throw `` `Could not extract cancel form data for position: {positionId}` ``).
   - **POST** form data to `/bestellungen/` with exactly these fields:
     ```
     cp_PositionId: {positionId}
     cp_EatingCycleId_{positionId}: {extracted eatingCycleId}
     cp_Date_{positionId}: {extracted date string, dd.MM.yyyy HH:mm:ss}
     ufprt: {extracted}
     __ncforminfo: {extracted}
     ```
     (Note the two dynamic field *names* embed the positionId.)
   - **GET** `/bestellungen/` again to obtain fresh tokens for the next cancellation.
4. After the loop, extract the edit-mode form from the latest HTML; if `editMode === 'False'`
   (still in edit mode), POST the toggle once more to exit edit mode and return the page to
   its normal state.

All form POSTs are `multipart/form-data` to the `/bestellungen/` path — wire details in
`01-gourmet-scraping.md`.

### 6.2 Single-order cancellation UX (Orders screen)

Store action `cancelOrder(positionId)` (v1: src/app/src-rn/store/orderStore.ts:108-122):

- Guard: no-op if another cancellation is in flight (`cancellingId !== null`).
- Set `cancellingId = positionId`, clear `error`.
- Call the cancellation operation with a single-element array `[positionId]`.
- On success: emit analytics `order.cancelled`, clear `cancellingId`, re-fetch orders.
- On failure: `error` = thrown message or fallback
  `Bestellung konnte nicht storniert werden`; clear `cancellingId`; **no** orders re-fetch
  (verified by test, v1: src/app/src-rn/__tests__/store/orderStore.test.ts:221-229).

There is no optimistic removal — the row stays until the post-cancel re-fetch.

UI flow (v1: src/app/app/(tabs)/orders.tsx:82-92): tapping an order's cancel button opens a
destructive confirmation dialog with title `Bestellung stornieren`, message
`"{order title}" stornieren?`, confirm label `Stornieren` (destructive style), cancel label
`Behalten`. Only on confirm is `cancelOrder` invoked.

Per-row cancel button (v1: src/app/src-rn/components/OrderItem.tsx):

- Rendered only when `canCancel` is true — the screen passes
  `activeTab === 'upcoming' && cancellingId === null`
  (v1: src/app/app/(tabs)/orders.tsx:135, 209). So past orders never show it, and all cancel
  buttons disappear while any cancellation is in flight.
- The button is a circular "✕" (`✕`) glyph.
- While this row is being cancelled (`cancellingId === positionId`): the whole row is
  dimmed to 60% opacity and the cancel button is hidden — like every other row's, since
  `canCancel` is false for all rows while a cancellation is in flight (see above).
  **Note:** OrderItem contains an `ActivityIndicator` spinner branch for
  `canCancel && isCancelling` (v1: OrderItem.tsx:53-55), but it is unreachable dead code
  given the `canCancel` prop the screen passes (`isCancelling` requires a non-null
  `cancellingId`, `canCancel` requires it null). v2 must decide deliberately whether to
  show a spinner on the cancelling row; v1's observable behavior is dim-only.

### 6.3 Batch cancellation (Menus screen)

Covered by §3/§4: toggling ordered items on the Menus screen accumulates
`pendingCancellations`; submission resolves them to positionIds and runs one batched
`cancelOrders` call (single edit-mode enter/exit for all of them), before any new orders
are added.

### 6.4 Cancellation cutoff

`isCancellationCutoff(orderDate)` — identical rule to the ordering cutoff
(v1: src/app/src-rn/utils/dateUtils.ts:107-112): past dates always blocked; today blocked
from 09:00 Europe/Vienna; future dates never blocked.

Enforcement is **client-side UI only** (v1: src/app/src-rn/components/OrderItem.tsx:21-32):

- The cancel button is disabled and visually dimmed when the cutoff applies.
- While not yet cut off, the component re-evaluates the cutoff every 30 seconds
  (interval `30_000` ms) so the button locks live at 09:00 without a re-render trigger.
  Once cut off, the interval stops (it never unlocks).
- The API-level `cancelOrders` performs no cutoff check, and `submitOrders` deliberately
  does not filter cancellations by cutoff (v1: src/app/src-rn/store/menuStore.ts:228 comment
  "cancellations are always allowed").

---

## 7. Orders screen (list UX)

(v1: src/app/app/(tabs)/orders.tsx)

**Auth gate:** if the Gourmet auth status is not `authenticated`, render only the centered
hint `Anmeldung erforderlich`.

**On screen focus** (every time the tab gains focus): emit analytics
`screen.viewed` with `{ screen: 'orders' }`; if authenticated, load cached orders first
(errors swallowed), then always trigger `fetchOrders()` (always a network fetch) and a
**non-forced** `fetchMenus()` (may no-op within the 4-hour menu cache validity window —
see 03-features/caching.md §3.1; menus are needed for description lookup, below)
(v1: src/app/app/(tabs)/orders.tsx:52-63; menuStore.ts:87-89).

**Tabs:** two tabs, `Kommende ({count})` and `Vergangene ({count})`; default `upcoming`.
Split rule (v1: src/app/src-rn/store/orderStore.ts:124-134): compute `now` = device-local
today at 00:00:00.000; upcoming = orders with `date >= now`, past = `date < now`. (Note this
split uses device-local midnight, unlike the cutoffs which use Vienna time.)

**Unconfirmed banner:** see §5.3.

**List:** one row per order, keyed by `positionId`. Empty states (only when not loading):
`Keine kommenden Bestellungen` / `Keine vergangenen Bestellungen`. A loading overlay shows
while fetching; a dismissable-by-refresh error banner shows `error` text when set.

**Row content** (v1: src/app/src-rn/components/OrderItem.tsx:34-69):

- Line 1: date formatted for display — `toLocaleDateString('de-AT', { weekday: 'short',
  month: 'short', day: 'numeric' })`, e.g. `Mo., 10. Feb.`
  (v1: src/app/src-rn/utils/dateUtils.ts:32-38).
- Line 2: category label = `order.title` (accent-colored, small caps style).
- Line 3 (max 2 lines): `menuDescription || order.subtitle || order.title`.
- Right side: status badge — `Bestätigt` (success style) when `approved`, otherwise
  `Ausstehend` (warning style) — plus the cancel button per §6.2.

**Menu description lookup** (v1: src/app/app/(tabs)/orders.tsx:70-80): build a map from
menu-store items with non-empty `subtitle`: key `"{day.toDateString()}|{title}"` → subtitle,
where `title` is the menu item's title (the category heading text, e.g. `MENÜ I`). Lookup
key for an order: `"{order.date.toDateString()}|{order.title}"`. This enriches the order row
with the dish description from the menus data when the orders-page subtitle is missing or
less specific. Later menu items overwrite earlier ones on key collision.

---

## 8. Store behavior, caching, and side effects

(v1: src/app/src-rn/store/orderStore.ts)

State: `orders[]`, `loading`, `cancellingId` (positionId being cancelled or null), `error`.

**`fetchOrders`** (lines 59-91):

- Guard: no-op while `loading` is already true (verified by test,
  v1: src/app/src-rn/__tests__/store/orderStore.test.ts:111-117).
- On success: replace `orders`, persist to cache key `orders_list` (dates serialized as ISO
  strings; see `03-features/caching.md`). **The cache write runs inside the same try
  block as the fetch, after `orders`/`loading` are already committed to state** — if the
  cache write throws, the failure path below fires (`error` is set, the notification
  block is skipped) even though the fetch succeeded and the fresh orders remain in state
  (v1: orderStore.ts:66-67, catch at 87-90).
- Then run notification updates (v1 gated these with `Platform.OS !== 'web'`; v2 is
  mobile-only so they always run):
  - If any order's date is the same calendar day as today in Vienna
    (`isSameDay(o.date, viennaToday())`): cancel the geofence "no order" notification.
  - Then run the daily-reminder check and the cancel-reminder check.
  - **All three helpers execute sequentially inside ONE shared try/catch**
    (v1: orderStore.ts:73-85): a failure in an earlier helper skips the remaining checks
    for that fetch — e.g. if cancelling the geofence notification rejects, neither
    reminder check runs. Only the combined failure is swallowed; it must not affect
    `orders`/`error` (verified by test,
    v1: src/app/src-rn/__tests__/store/orderStore.test.ts:140-148 — which pins the
    orders/error outcome, not that later helpers still run).
  See the notifications feature docs for what these checks do.
- On failure: `error` = thrown message or fallback
  `Bestellungen konnten nicht geladen werden`; `loading` false; existing `orders` kept.

**`loadCachedOrders`** (lines 48-57): read cache key `orders_list`; absent → no-op;
parse and revive ISO date strings to dates; on parse failure, delete the cache entry and
keep current state.

**Error message catalog** (exact strings, all German):

| Context | Message |
|---|---|
| Orders fetch failed (non-Error throw) | `Bestellungen konnten nicht geladen werden` |
| Confirm failed (non-Error throw) | `Bestellungen konnten nicht bestätigt werden` |
| Cancel failed (non-Error throw) | `Bestellung konnte nicht storniert werden` |
| Submit pipeline failed (non-Error throw) | `Bestellung konnte nicht aufgegeben werden` |
| All new orders past cutoff | `Bestellung für heute geschlossen (Bestellschluss 9:00)` |
| Add-to-cart rejected by server | `Add to cart failed: {message \| 'unknown error'}` |
| Edit mode could not be entered | `Failed to enter edit mode` |
| Add-to-cart without login info | `Not logged in` |

When the throw *is* an `Error`, its own `message` is displayed instead of the fallback.

**Analytics signals** (see `03-features/analytics.md`): `order.confirmed`,
`order.cancelled`, `order.submitted` (props `orderedCount`, `cancelledCount` as strings),
`screen.viewed` (prop `screen: 'orders'`).

---

## 9. Dropped in v2

- **Wide/desktop layout** of the Orders screen: v1 renders a sidebar
  (`OrdersPanel`, v1: src/app/src-rn/components/OrdersPanel.tsx) with view selector
  (`Ansicht`: `Kommende`/`Vergangene` with counts), a summary block (`Übersicht`:
  `Kommende`/`Vergangene`/`Unbestätigt` counts, unconfirmed count highlighted in warning
  color when > 0), and an `Alle bestätigen` confirm button — shown only when
  `useDesktopLayout()` reports a wide layout. Desktop is dropped in v2; implement only the
  phone layout (tabs + banner, §7).
- `DesktopContentWrapper` (max-width 700 centering wrapper around the phone layout,
  v1: src/app/app/(tabs)/orders.tsx:155) — desktop-only concern.
- The `Platform.OS !== 'web'` guard around the notification hooks in `fetchOrders`
  (v1: src/app/src-rn/store/orderStore.ts:70) — v2 has no web target; the hooks always run.
- `isCompactDesktop` / `useFlatStyle` style variants in OrderItem/OrdersPanel/orders screen
  styles — desktop styling switches; see `04-ui-ux.md` for the retained mobile styling.

---

## 10. Open questions

- Whether the Gourmet server enforces the 09:00 cutoff server-side. v1 enforces it purely
  client-side (button disable + submit filter); server behavior on a post-09:00
  order/cancellation attempt is not determinable from source.
- Which approval marker (`.fa-check` vs `.checkmark`) the live site currently emits and
  under what conditions each appears — the sanitized fixture contains both, and the code
  accepts either.
- Whether cancelling an *unconfirmed* order behaves differently server-side from cancelling
  a confirmed one (v1 treats them identically).
- Semantics of `eatingCycleId` and `cp_Date_{positionId}` on the server (v1 only echoes the
  extracted values back).
- Why `getUpcomingOrders`/`getPastOrders` use device-local midnight while cutoffs use
  Europe/Vienna — likely an oversight, but the intended behavior for users outside Vienna's
  timezone is not documented anywhere in v1.
