# Caching & offline behavior

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This doc specifies what data the app persists locally, under which keys, how staleness and
invalidation work, how screens boot from cache, and the exact merge semantics between cached
and live data. HTTP request specs for the fetches mentioned here (`getMenus`, `getOrders`,
`getBillings`, `getTransactions`, order mutation calls) are owned by
`01-gourmet-scraping.md` and `02-ventopay-scraping.md` — do not re-derive them from this doc.
Screen-level UI is owned by `03-features/menus.md`, `03-features/orders.md`,
`03-features/billing.md`; background notification tasks by the `03-features/notifications-*`
docs.

---

## 1. Storage mechanism

**v1 mechanism:** all caches live in React Native AsyncStorage
(`@react-native-async-storage/async-storage`) — an unencrypted, on-device, string key →
string value store that survives app restarts (v1: src/app/src-rn/store/menuStore.ts:2,
src/app/src-rn/store/orderStore.ts:3, src/app/src-rn/store/billingStore.ts:2). Values are
JSON strings produced by `JSON.stringify`.

v2 requirement: any durable local key-value/database store with equivalent semantics
(persists across app restarts; per-app private storage; no encryption required — cached data
is menu/order/billing content, never credentials). Credentials are stored separately via the
secure-storage mechanism (see `05-platform-services.md`); they are **not** part of this
caching layer.

There is no migration requirement to read v1's AsyncStorage entries in v2; the exact key
names below are documented for provenance and so behavior (e.g. per-month keying) can be
reproduced.

### Cache inventory

| Key | Payload | Written by | Read by |
|---|---|---|---|
| `menus_items` | Full menu item list (all days), JSON array | menu fetch success, availability-refresh success, order-submit failure revert (v1: src/app/src-rn/store/menuStore.ts:10,96,151,295) | `loadCachedMenus` (v1: src/app/src-rn/store/menuStore.ts:71-80) |
| `orders_list` | Full ordered-menus list, JSON array | order fetch success (v1: src/app/src-rn/store/orderStore.ts:9,67) | `loadCachedOrders` (v1: src/app/src-rn/store/orderStore.ts:48-57); also background tasks (§4.4) |
| `billing_{YYYY-MM}` | Gourmet bills for one month, JSON array | Gourmet billing fetch success (v1: src/app/src-rn/store/billingStore.ts:8,225) | `loadCachedMonths` (v1: src/app/src-rn/store/billingStore.ts:162) |
| `ventopay_billing_{YYYY-MM}` | Ventopay transactions for one month, JSON array | Ventopay billing fetch success (v1: src/app/src-rn/store/billingStore.ts:9,268-271) | `loadCachedMonths` (v1: src/app/src-rn/store/billingStore.ts:176) |

Other AsyncStorage keys exist in v1 but are owned by other docs, not this caching layer:
`known_menu_fingerprints`, `menu_notification_sent` (notifications-new-menu),
`daily_reminder_enabled`, `daily_reminder_time`, `daily_reminder_sent_date`
(notifications-daily-reminder / notifications-cancel-reminder),
`notification_debug_log_entries`, `notification_debug_log_activated_until`
(notification-log), `company-location` (zustand-persist, notifications-location),
`theme-preference` (zustand-persist, themes), and
`@secureStorage:migratedAfterFirstUnlock` (05-platform-services). The full key table
lives in 05-platform-services.md.

---

## 2. Serialization formats

All caches store the domain objects as-is except `Date` fields, which are converted to ISO
8601 UTC strings via `Date.prototype.toISOString()` (e.g. `"2026-02-10T00:00:00.000Z"`) on
write and revived with `new Date(isoString)` on read. Every other field round-trips through
JSON unchanged.

### 2.1 `menus_items`

JSON array of `GourmetMenuItem` with `day` as ISO string
(v1: src/app/src-rn/store/menuStore.ts:14-27). The item shape
(v1: src/app/src-rn/types/menu.ts):

```ts
{
  id: string;
  day: Date;            // serialized as ISO string
  title: string;
  subtitle: string;
  allergens: string[];
  available: boolean;
  ordered: boolean;
  category: string;     // one of 'MENÜ I' | 'MENÜ II' | 'MENÜ III' | 'SUPPE & SALAT' | 'UNKNOWN'
  price: string;
}
```

### 2.2 `orders_list`

JSON array of `GourmetOrderedMenu` with `date` as ISO string
(v1: src/app/src-rn/store/orderStore.ts:12-25). The order shape
(v1: src/app/src-rn/types/order.ts):

```ts
{
  positionId: string;
  eatingCycleId: string;
  date: Date;           // serialized as ISO string
  title: string;
  subtitle: string;
  approved: boolean;
}
```

### 2.3 `billing_{YYYY-MM}` (Gourmet)

JSON array of `GourmetBill` with `billDate` as ISO string
(v1: src/app/src-rn/store/billingStore.ts:51-64). Only the raw `bills` array is persisted —
the monthly aggregates (`totalGross`, `totalSubsidy`, `totalDiscount`, `totalBilling`), the
display `label`, and `fetchedAt` are **recomputed at load time**, not stored
(v1: src/app/src-rn/store/billingStore.ts:162-173). Totals formula
(v1: src/app/src-rn/store/billingStore.ts:30-44): sum `item.total`, `item.subsidy`,
`item.discountValue` over every item of every bill; sum `bill.billing` per bill. Bill shape:
see `03-features/billing.md` (v1: src/app/src-rn/types/billing.ts).

### 2.4 `ventopay_billing_{YYYY-MM}` (Ventopay)

JSON array of `VentopayTransaction` with `date` as ISO string
(v1: src/app/src-rn/store/billingStore.ts:67-80). Only the raw `transactions` array is
persisted; `total` is recomputed at load as the sum of `t.amount`
(v1: src/app/src-rn/store/billingStore.ts:46-48,176-186). Transaction shape
(v1: src/app/src-rn/types/ventopay.ts): `{ id, date, amount, restaurant, location }`.

### 2.5 Month keys

Billing caches are keyed per calendar month with key `{prefix}{YYYY-MM}`, where the `YYYY-MM`
part is computed from "today minus N months" in device-local time
(v1: src/app/src-rn/store/billingStore.ts:12-18):

```ts
function monthKeyFromOffset(offset: number): string {
  const now = new Date();
  const target = new Date(now.getFullYear(), now.getMonth() - offset, 1);
  const yyyy = target.getFullYear();
  const mm = String(target.getMonth() + 1).padStart(2, '0');
  return `${yyyy}-${mm}`;
}
```

The month window is always exactly offsets `[0, 1, 2]` — current month plus the two previous
months (v1: src/app/src-rn/store/billingStore.ts:126-130). Display labels are recomputed from
the key using the Austrian German month names, exactly:
`['Jänner', 'Februar', 'März', 'April', 'Mai', 'Juni', 'Juli', 'August', 'September',
'Oktober', 'November', 'Dezember']`, formatted `"{month} {yyyy}"`
(v1: src/app/src-rn/store/billingStore.ts:21-28). For Ventopay fetches the month key expands
to a date range = first day of month .. last day of month
(v1: src/app/src-rn/store/billingStore.ts:83-88).

---

## 3. Staleness & invalidation rules

### 3.1 Menus: 4-hour in-memory TTL

- `MENU_CACHE_VALIDITY_MS = 4 * 60 * 60 * 1000` (4 hours)
  (v1: src/app/src-rn/utils/constants.ts:9).
- A menu fetch (`fetchMenus(force?)`) is skipped when a fetch is already in flight, or when
  `!force && lastFetched != null && Date.now() - lastFetched < MENU_CACHE_VALIDITY_MS`
  (v1: src/app/src-rn/store/menuStore.ts:82-89). `force = true` bypasses the TTL
  (verified by tests, v1: src/app/src-rn/__tests__/store/menuStore.test.ts:130-145).
- **`lastFetched` is in-memory only — it is never persisted.** Loading the disk cache sets
  only `items`, leaving `lastFetched` null (v1: src/app/src-rn/store/menuStore.ts:71-80).
  Consequence: after a cold start the TTL never suppresses a network fetch; the TTL only
  matters within a warm session — the Orders tab's non-forced `fetchMenus()` call (§4.2)
  AND the Menus tab's fallback `fetchMenus()` (§4.1 step 2, reachable when the item list
  is empty but `lastFetched` is recent, e.g. the last fetch returned an empty list).
  `lastFetched` is set to `Date.now()` on full-fetch success, on merge-refresh
  success, and on the order-failure revert fetch
  (v1: src/app/src-rn/store/menuStore.ts:95,150,294).

### 3.2 Orders: no TTL

Every `fetchOrders()` call hits the network, guarded only by "skip if a fetch is already in
flight" (v1: src/app/src-rn/store/orderStore.ts:59-62). The disk cache exists purely for
instant display and offline/background reads.

### 3.3 Billing: past months are permanent, current month always refetches

For both sources, given the selected month offset (v1:
src/app/src-rn/store/billingStore.ts:203-209 for Gourmet, 243-249 for Ventopay):

- offset `0` (current month): always fetch from network.
- offset `1` or `2` (past months): if in-memory state already holds that month **and** its
  data array is non-empty (`bills.length > 0` / `transactions.length > 0`), skip the network
  entirely — past-month data is treated as immutable. An empty cached month is refetched
  every time.

Additional guards: Gourmet fetch is skipped while `loading` is true
(v1: src/app/src-rn/store/billingStore.ts:201); the Ventopay fetch has **no** in-flight
guard and no loading flag, is skipped unless the Ventopay session is authenticated, and its
failures are non-blocking (logged to console, no user-visible error, Gourmet errors never
overwritten) (v1: src/app/src-rn/store/billingStore.ts:251-253,276-279).

`fetchedAt` on the in-memory month record is `Date.now()` for live-fetched data and the
sentinel `0` for disk-loaded data (v1: src/app/src-rn/store/billingStore.ts:171,184,222,265).
**`fetchedAt` participates in no staleness decision in v1** — only presence + non-emptiness
matter. Preserve the field (it is part of the month record type) but do not invent TTL logic
around it.

### 3.4 Corrupt cache entries

- `loadCachedMenus` / `loadCachedOrders`: if the cache key is **absent**, the loader
  returns without touching in-memory state — a missing entry never clears existing
  items/orders (v1: src/app/src-rn/store/menuStore.ts:72-73,
  src/app/src-rn/store/orderStore.ts:49-50; pinned by "does nothing when cache is empty"
  tests, menuStore.test.ts:578-581 and orderStore.test.ts:305-308). `loadCachedMonths`
  likewise skips missing month keys. If deserialization throws (invalid JSON), the
  cache entry is **deleted** (`removeItem`) and the in-memory state is left unchanged
  (v1: src/app/src-rn/store/menuStore.ts:74-79, src/app/src-rn/store/orderStore.ts:51-56).
  Tests pin this: a corrupt `orders_list` entry must trigger `removeItem('orders_list')` and
  leave `orders` empty (v1: src/app/src-rn/__tests__/store/orderStore.test.ts:310-318);
  a corrupt `menus_items` entry leaves `items` empty
  (v1: src/app/src-rn/__tests__/store/menuStore.test.ts:583-590).
- `loadCachedMonths` (billing) has **no** corrupt-entry handling: a `JSON.parse` throw
  rejects the whole load promise mid-loop; no state is set and the corrupt key is not
  removed. Callers swallow the rejection (`.catch(() => {})`,
  v1: src/app/app/(tabs)/billing.tsx:114). This asymmetry is v1 behavior as shipped.

### 3.5 No eviction, no logout clearing

- Billing month keys accumulate indefinitely: keys older than the 3-month window are never
  read again and never deleted (v1: src/app/src-rn/store/billingStore.ts — no `removeItem`
  outside corrupt handling, which billing lacks).
- **Logout does not clear any cache.** Gourmet logout only resets auth state
  (v1: src/app/src-rn/store/authStore.ts:64-71); cached menus/orders/billing remain on disk
  and in memory.
- Demo-mode login resets the in-memory menu store (`items: [], lastFetched: null`) but does
  **not** remove the persisted `menus_items` entry
  (v1: src/app/src-rn/store/authStore.ts:38); a later cache load can re-surface real data.
  See `03-features/demo-mode.md`.

---

## 4. Startup-from-cache behavior

The pattern (from the v1 design doc, docs/plans/2026-02-21-cache-menus-orders-design.md, and
the shipped screens): **load cache first for instant display, then background-refresh from
the network; cache-load errors are always swallowed and never block the network refresh.**

### 4.1 Menus tab (on screen focus and on becoming authenticated)

Only runs when Gourmet auth status is `authenticated`
(v1: src/app/app/(tabs)/index.tsx:88-124):

1. Run `loadCachedMenus()` and `loadCachedOrders()` in parallel; ignore errors.
2. When both settle: if the menu store now has any items (`items.length > 0`), run the
   **merge refresh** `refreshAvailability()` (§5); otherwise run the full `fetchMenus()`.
3. After the chosen menu refresh resolves, run the new-menu fingerprint/toast check (owned
   by `03-features/notifications-new-menu.md`).
4. Independently kick off `fetchOrders()` (not awaited).

### 4.2 Orders tab (on screen focus)

Only when authenticated (v1: src/app/app/(tabs)/orders.tsx:52-63):
`loadCachedOrders()` → ignore errors → then `fetchOrders()` and a non-forced `fetchMenus()`
(the latter may be suppressed by the 4-hour TTL, §3.1; menu subtitles are used to enrich
order rows — see `03-features/orders.md`).

### 4.3 Billing tab (on screen focus)

When at least one of the two sources is authenticated
(v1: src/app/app/(tabs)/billing.tsx:111-119): `loadCachedMonths()` (loads all cached entries
for the current 3-month window, both sources, into memory) → ignore errors → then
`fetchBilling()` if Gourmet is authenticated and `fetchVentopayBilling()` if Ventopay is
authenticated (both for the currently selected month; subject to §3.3 skip rules). Selecting
a month re-triggers both fetches for that offset
(v1: src/app/src-rn/store/billingStore.ts:144-148).

Note: `loadCachedMonths` merges disk data **over** in-memory months
(`{ ...current, ...fromDisk }`, v1: src/app/src-rn/store/billingStore.ts:189-192); with the
screen ordering above this is benign, but a v2 implementation must not let a late-resolving
cache load clobber a fresher live result for the same month key — or must reproduce the same
call ordering (cache load strictly before fetches).

### 4.4 Background tasks read the order cache without network

Background/geofence tasks must populate order state from the `orders_list` cache **only** —
deliberately no network call, to avoid concurrent scraping sessions
(v1: src/app/src-rn/utils/notificationTasks.ts:40,69,87 — comment: "Load cached orders (no
network calls to avoid concurrent scraping)"). Used to decide "has order today" for the
geofence notification and for the daily/cancel reminder checks. Details owned by the
`03-features/notifications-*` docs; the caching contract here is: cached orders must be
readable outside the UI process/foreground lifecycle.

### 4.5 What is NOT cached

Session-only state that must reset on app restart: pending order selections and pending
cancellations (in-memory `Set`s of `"{menuId}|{YYYY-MM-DD}"` keys), selected date, selected
billing month index, source filter, loading/refreshing/error/progress flags
(v1: src/app/src-rn/store/menuStore.ts:31-53, src/app/src-rn/store/billingStore.ts:92-116).
Auth sessions/cookies are also not part of this layer (see 01-/02- scraping docs).

---

## 5. Cache vs live merge semantics (`refreshAvailability`)

The menus screen's background refresh does **not** replace the cached list; it merges only
volatile fields into it (v1: src/app/src-rn/store/menuStore.ts:112-157). Exact algorithm:

1. Skip entirely if a refresh is already running or the in-memory item list is empty.
2. Start an 800 ms minimum-visibility timer (`new Promise((r) => setTimeout(r, 800))`)
   **before** the network call, and set `refreshing: true`. Both the success and the failure
   paths await this timer before clearing `refreshing`, so the refresh indicator is visible
   for `max(networkTime, 800ms)` (v1: src/app/src-rn/store/menuStore.ts:120,148-155).
3. Fetch the full fresh menu list from the network.
4. Build a lookup map from fresh items keyed by `` `${item.id}|${localDateKey(item.day)}` ``,
   where `localDateKey` formats the `Date` in **device-local time** as zero-padded
   `YYYY-MM-DD` (v1: src/app/src-rn/utils/dateUtils.ts:45-50):

   ```ts
   const y = date.getFullYear();
   const m = String(date.getMonth() + 1).padStart(2, '0');
   const d = String(date.getDate()).padStart(2, '0');
   return `${y}-${m}-${d}`;
   ```

5. Map over the **cached** items in their existing order. For each cached item whose key has
   a fresh match: keep every cached field and overwrite **only** `available` and `ordered`
   from the fresh item; remove the match from the map. Cached items with no fresh match are
   kept unchanged — the merge never deletes items; stale days disappear only when a full
   `fetchMenus` replaces the whole list.
6. Append all remaining (brand-new) fresh items to the end of the merged list.
7. On success: set the merged list, `lastFetched = Date.now()`, `refreshing: false`, and
   write the merged list through to the `menus_items` cache.
8. On failure: **silent** — after the 800 ms timer, set `refreshing: false` only. No error is
   surfaced, the cached items stay visible, and nothing is written to disk.

Pinned by tests (v1: src/app/src-rn/__tests__/store/menuStore.test.ts:157-176,593-621):
merging updates `available`/`ordered` on matched items; brand-new fresh items are appended;
on refresh error the state keeps the cached items with `error` still null.

### 5.1 Write-through points (complete list)

| Trigger | Cache written | Provenance |
|---|---|---|
| `fetchMenus` success | `menus_items` ← fetched list | v1: src/app/src-rn/store/menuStore.ts:96 |
| `refreshAvailability` success | `menus_items` ← merged list | v1: src/app/src-rn/store/menuStore.ts:151 |
| `submitOrders` failure revert (re-fetch after a failed order mutation) | `menus_items` ← fresh list; if this revert fetch itself fails, nothing is written and the optimistic in-memory state remains | v1: src/app/src-rn/store/menuStore.ts:290-298 |
| `fetchOrders` success | `orders_list` ← fetched list | v1: src/app/src-rn/store/orderStore.ts:67 |
| `fetchBilling` success | `billing_{YYYY-MM}` ← fetched bills | v1: src/app/src-rn/store/billingStore.ts:225 |
| `fetchVentopayBilling` success | `ventopay_billing_{YYYY-MM}` ← fetched transactions | v1: src/app/src-rn/store/billingStore.ts:268-271 |

The optimistic UI update during order submission (flipping `ordered` flags before the network
calls, v1: src/app/src-rn/store/menuStore.ts:236-254) mutates in-memory state only — it is
**never** written to the cache; the post-submit `fetchMenus(true)` write-through is what
persists the final state. Order submission flow itself is owned by `03-features/orders.md` /
`03-features/menus.md`.

**Write-failure semantics and set()/write ordering** (differ per store; replicate as-is):

| Write point | Ordering | On write failure |
|---|---|---|
| `fetchMenus` | state committed first, awaited write inside the same try (v1: menuStore.ts:95-96) | fetch error path fires: user-visible `error` set, though the fetched items stay in state; the auto-select-nearest-date step (menuStore.ts:98-105, owned by 03-features/menus.md §3.2) is also skipped (v1: menuStore.ts:106-108) |
| `fetchOrders` | state committed first, awaited write inside the same try (v1: orderStore.ts:66-67) | `error` set AND the notification side-effects are skipped; fetched orders stay in state (v1: orderStore.ts:87-90) |
| `fetchBilling` (Gourmet) | write happens BEFORE `set()` (v1: billingStore.ts:225-230) | fetched month is discarded entirely; `error` set (v1: billingStore.ts:231-234) |
| `fetchVentopayBilling` | write BEFORE `set()` (v1: billingStore.ts:268-271) | fetched month discarded, silent (`console.warn` only) (v1: billingStore.ts:276-279) |
| `refreshAvailability` | after merge | silent — swallowed (v1: menuStore.ts:151-156) |
| `submitOrders` revert | state committed BEFORE write (v1: menuStore.ts:294-295) | silent; two sub-cases — write-only failure: revert-fetched fresh items stay in state, nothing persisted; revert FETCH failure: optimistic state kept (v1: menuStore.ts:294-298) |

---

## 6. Dropped in v2

- `orderStore.fetchOrders` gates its notification side-effects behind
  `Platform.OS !== 'web'` (v1: src/app/src-rn/store/orderStore.ts:70). Web is dropped in v2,
  so the notification refresh after an order fetch is unconditional on mobile (the
  side-effects themselves are owned by the notifications docs).
- No other caching code has desktop/web branches; AsyncStorage backed onto localStorage on
  web in v1, which is moot in v2.

## 7. Discrepancies: code vs design doc

The implementation plan (v1: docs/plans/2026-02-21-cache-menus-orders-design.md) differs
from shipped code in two places; **the code wins**:

1. The plan's `loadCachedMenus`/`loadCachedOrders` snippets have no corrupt-entry handling;
   shipped code wraps deserialization in try/catch and deletes the corrupt key (§3.4).
2. The plan's Menus-tab snippet fires `refreshAvailability()`/`fetchMenus()` without using
   the returned promise; shipped code captures it and chains the new-menu fingerprint check
   (§4.1).

No conflicts with CLAUDE.md were found (CLAUDE.md does not describe the caching layer).
