# Billing feature (both sources)

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

The Billing tab shows a unified, per-month list of cafeteria receipts from two independent
sources:

- **Gourmet ("Kantine")** bills — fetched via the Gourmet `GetMyBillings` JSON API.
  HTTP details (URL, headers, body, session handling) are owned by
  `01-gourmet-scraping.md`; this doc only specifies the parameters the billing feature
  passes in and what it does with the results.
- **Ventopay ("Automaten")** transactions — fetched via the Ventopay transaction pages.
  HTTP/parsing details are owned by `02-ventopay-scraping.md`.

Cache persistence mechanics are shared with the other features and are additionally
covered by `03-features/caching.md`; visual styling (colors, surfaces, spacing) is owned
by `04-ui-ux.md`. This doc specifies the behavior: month/date handling, fetch and refresh
rules, merging, filtering, totals, and display content.

---

## 1. Data model

### 1.1 Gourmet types (v1: src/app/src-rn/types/billing.ts:1-34)

```
GourmetBillingItem {
  id: string
  articleId: string
  count: number
  description: string
  total: number          // gross line total
  subsidy: number
  discountValue: number
  isCustomMenu: boolean
}

GourmetBill {
  billNr: number
  billDate: Date (full timestamp)
  location: string
  items: GourmetBillingItem[]
  billing: number        // total billed amount after subsidy/discount
}

GourmetMonthlyBilling {
  monthKey: string       // "YYYY-MM"
  label: string          // e.g. "Jänner 2026"
  bills: GourmetBill[]
  totalGross: number
  totalSubsidy: number
  totalDiscount: number
  totalBilling: number
  fetchedAt: number      // Unix ms timestamp; 0 = restored from cache
}
```

How `GourmetBill` is populated from the `GetMyBillings` JSON response is specified in
`01-gourmet-scraping.md` (v1: src/app/src-rn/api/gourmetApi.ts:308).

### 1.2 Ventopay types (v1: src/app/src-rn/types/ventopay.ts:1-17)

```
VentopayTransaction {
  id: string
  date: Date (full timestamp)
  amount: number
  restaurant: string
  location: string
}

VentopayMonthlyBilling {
  monthKey: string       // "YYYY-MM"
  label: string
  transactions: VentopayTransaction[]
  total: number
  fetchedAt: number      // Unix ms timestamp; 0 = restored from cache
}
```

How `VentopayTransaction` is populated (including the "Gourmet"/"Kaffeeautomat"
transaction filter rule, which is applied inside the scraping layer, NOT in the billing
feature) is specified in `02-ventopay-scraping.md`.

---

## 2. Month handling

### 2.1 Month options

Exactly **3 months** are offered: the current month plus the 2 previous months, as
offsets `0, 1, 2` (v1: src/app/src-rn/store/billingStore.ts:126-130). Verified by test
"returns 3 month options" (v1: src/app/src-rn/__tests__/store/billingStore.test.ts:97-103).

Each option is `{ key, label, offset }` where:

- **key** = `"YYYY-MM"`, computed in the device's local timezone as
  `new Date(now.getFullYear(), now.getMonth() - offset, 1)` then formatted with
  4-digit year, `-`, and zero-padded 2-digit month (v1:
  src/app/src-rn/store/billingStore.ts:12-18). JavaScript `Date` month rollover handles
  year boundaries (e.g. offset 1 in January 2026 → `"2025-12"`); v2 must reproduce this
  calendar-month arithmetic.
- **label** = `"{GermanMonthName} {YYYY}"` using the exact Austrian-German month names
  (v1: src/app/src-rn/store/billingStore.ts:21-28):

  ```
  Jänner, Februar, März, April, Mai, Juni,
  Juli, August, September, Oktober, November, Dezember
  ```

  Note `Jänner` (Austrian), not `Januar`.

Month options are recomputed on every access from the current date; they are not cached
across month boundaries (v1: src/app/src-rn/store/billingStore.ts:126-130).

### 2.2 Month → date range (Ventopay)

For a `"YYYY-MM"` key, the fetch range is (v1: src/app/src-rn/store/billingStore.ts:83-88):

- `from` = first day of that month, local midnight: `new Date(yyyy, mm - 1, 1)`
- `to` = last day of that month, local midnight: `new Date(yyyy, mm, 0)`

These two `Date` values are passed to the Ventopay API layer, which formats them as
`dd.MM.yyyy` request parameters (see `02-ventopay-scraping.md`). Verified by test:
`from.getDate()` is `1` (v1: src/app/src-rn/__tests__/store/billingStore.test.ts:186-197).

### 2.3 Month → offset (Gourmet)

The Gourmet fetch does not use a date range. The month offset is passed **as a string**
(`"0"`, `"1"`, or `"2"`) to the API layer, which sends it as the
`checkLastMonthNumber` field of the `GetMyBillings` request body (v1:
src/app/src-rn/store/billingStore.ts:214; parameter semantics documented at
src/app/src-rn/api/gourmetApi.ts:304-307: `"0"` = current month, `"1"` = last month,
`"2"` = 2 months ago). Verified by test: `getBillings` called with `'0'` and `'1'`
(v1: src/app/src-rn/__tests__/store/billingStore.test.ts:119-124, 270-280).

> Note vs. CLAUDE.md: the project CLAUDE.md shows `"checkLastMonthNumber": "0"` in its
> example payload. The code sends the selected month offset (`"0"`/`"1"`/`"2"`), not
> a constant `"0"`. The code wins.

The billing feature performs **no client-side date filtering** on Gourmet bills: whatever
the server returns for that offset is stored under the month key as-is (v1:
src/app/src-rn/store/billingStore.ts:214-228).

---

## 3. State

Billing state (v1: src/app/src-rn/store/billingStore.ts:92-124):

| Field | Type | Initial | Meaning |
|---|---|---|---|
| `gourmetMonths` | map `"YYYY-MM"` → `GourmetMonthlyBilling` | `{}` | Fetched/cached Gourmet data |
| `ventopayMonths` | map `"YYYY-MM"` → `VentopayMonthlyBilling` | `{}` | Fetched/cached Ventopay data |
| `selectedMonthIndex` | number (0–2) | `0` | Index into month options |
| `sourceFilter` | `'all' \| 'gourmet' \| 'ventopay'` | `'all'` | Active source filter |
| `loading` | boolean | `false` | Set only by the **Gourmet** fetch |
| `error` | string \| null | `null` | Set only by the **Gourmet** fetch |

Selected-month getters return the entry for `monthOptions[selectedMonthIndex].key`, or
`null` if there is no data for that key **or** the index is out of range (v1:
src/app/src-rn/store/billingStore.ts:132-142; out-of-range verified with index 7 at
src/app/src-rn/__tests__/store/billingStore.test.ts:257-261).

Neither `selectedMonthIndex` nor `sourceFilter` is persisted; both reset to defaults on
app restart.

---

## 4. Fetching

### 4.1 Gourmet fetch (`fetchBilling`) (v1: src/app/src-rn/store/billingStore.ts:195-235)

Given an optional month offset (defaults to `selectedMonthIndex`):

1. Resolve the month option; if the offset is not 0–2, **return silently** (test:
   billingStore.test.ts:265-268).
2. If `loading` is already `true`, **return silently** (concurrency guard; test:
   billingStore.test.ts:290-294).
3. **Past-month skip**: if offset ≠ 0 AND an entry for this month key already exists in
   memory with `bills.length > 0`, return without fetching (test:
   billingStore.test.ts:161-182). The current month (offset 0) is **always** refetched.
   An existing entry with an empty `bills` array does NOT suppress the fetch.
4. Set `loading = true`, `error = null`.
5. Call the Gourmet API `getBillings(String(offset))` (requires an authenticated Gourmet
   session; see `01-gourmet-scraping.md` for the session re-validation and the
   `GetMyBillings` request).
6. Compute totals over the returned bills (v1: src/app/src-rn/store/billingStore.ts:30-44):
   - `totalGross` = Σ over all bills, all items of `item.total`
   - `totalSubsidy` = Σ `item.subsidy`
   - `totalDiscount` = Σ `item.discountValue`
   - `totalBilling` = Σ over all bills of `bill.billing`
7. Persist the raw bills to cache (section 5).
8. Store `{ monthKey, label, bills, …totals, fetchedAt: Date.now() }` under the month key
   and set `loading = false`.
9. On failure: set `error` to `err.message` if the rejection is an `Error`, otherwise to
   the literal fallback `Abrechnung konnte nicht geladen werden`; set `loading = false`.
   Previously stored month data is left untouched (tests: billingStore.test.ts:152-159,
   282-288).

### 4.2 Ventopay fetch (`fetchVentopayBilling`) (v1: src/app/src-rn/store/billingStore.ts:237-280)

Given an optional month offset (defaults to `selectedMonthIndex`):

1. Resolve the month option; if invalid, return silently (test: billingStore.test.ts:298-301).
2. **Past-month skip**: if offset ≠ 0 AND an in-memory entry exists with
   `transactions.length > 0`, return without fetching (test: billingStore.test.ts:303-321).
   Current month always refetches.
3. If the Ventopay auth status is not `'authenticated'`, return silently (test:
   billingStore.test.ts:323-329). (Gourmet fetch has no such internal check; callers gate
   it — see section 7.)
4. Compute the month date range (section 2.2) and call the Ventopay API
   `getTransactions(from, to)` (see `02-ventopay-scraping.md`).
5. Compute `total` = Σ `transaction.amount` (v1: src/app/src-rn/store/billingStore.ts:46-48).
6. Persist the raw transactions to cache (section 5).
7. Store `{ monthKey, label, transactions, total, fetchedAt: Date.now() }` under the
   month key.
8. On failure: log a warning and do **nothing else** — the shared `error` field is NOT
   set and no loading flag is involved; Ventopay failures are non-blocking so they never
   mask Gourmet data or errors (v1: src/app/src-rn/store/billingStore.ts:276-279; test:
   billingStore.test.ts:331-340).

Note the deliberate asymmetry: `loading`/`error` reflect **only** the Gourmet fetch. The
Ventopay fetch has no loading indicator and swallows errors.

### 4.3 Month selection (`selectMonth`) (v1: src/app/src-rn/store/billingStore.ts:144-148)

Selecting a month sets `selectedMonthIndex` and immediately triggers **both** fetches for
that index (each still subject to its own skip rules above; test:
billingStore.test.ts:213-231). It is fire-and-forget; the UI does not await it.

---

## 5. Caching (persistence)

Shared conventions are in `03-features/caching.md`; billing-specific facts:

- Storage: device-local key-value store (v1 mechanism: `@react-native-async-storage/async-storage`).
- Keys (v1: src/app/src-rn/store/billingStore.ts:8-9):
  - Gourmet: `billing_` + monthKey → e.g. `billing_2026-07`
  - Ventopay: `ventopay_billing_` + monthKey → e.g. `ventopay_billing_2026-07`
- Value: JSON array of the raw records only (bills / transactions), NOT the monthly
  aggregate. `Date` fields (`billDate` / `date`) are serialized as ISO-8601 strings via
  `toISOString()` and revived to dates on load (v1: src/app/src-rn/store/billingStore.ts:51-80;
  test: billingStore.test.ts:354-376).
- Totals are **recomputed from the raw records** on cache restore, never stored (v1:
  src/app/src-rn/store/billingStore.ts:165, 183).
- Restored entries get `fetchedAt: 0` to mark them as cache-derived (v1:
  src/app/src-rn/store/billingStore.ts:171, 184; test asserts `fetchedAt === 0`).
- A fetch that returns an empty list still overwrites the cache entry with `[]`.

**Restore (`loadCachedMonths`)** (v1: src/app/src-rn/store/billingStore.ts:155-193):
for each of the 3 current month options, read both cache keys; for each hit, build the
monthly aggregate and merge into state. The merge spreads cached entries **over** the
in-memory maps (`{ ...inMemory, ...cached }`), so for a given month key a cache entry
replaces whatever is in memory at that moment. Months with no cache entry are simply
absent (test: billingStore.test.ts:378-382). Cache entries for months that have fallen
out of the 3-month window are never read (and are not proactively deleted).

---

## 6. Merging, filtering, totals (unified view)

### 6.1 Source filter

Three values (v1: src/app/src-rn/store/billingStore.ts:90; labels at
src/app/app/(tabs)/billing.tsx:24-28):

| Value | UI label |
|---|---|
| `all` | `Alle` |
| `gourmet` | `Kantine` |
| `ventopay` | `Automaten` |

Default is `all`. The filter only affects presentation; it never suppresses fetching or
caching of either source.

### 6.2 Unified entry list (v1: src/app/app/(tabs)/billing.tsx:63-85)

For the selected month:

1. If `sourceFilter !== 'ventopay'` and Gourmet data exists, include every `GourmetBill`
   as an entry tagged `gourmet`.
2. If `sourceFilter !== 'gourmet'` and Ventopay data exists, include every
   `VentopayTransaction` as an entry tagged `ventopay`.
3. Sort the combined list by timestamp **descending** (newest first), comparing
   `billDate` (Gourmet) against `date` (Ventopay) as epoch milliseconds. No secondary
   sort key; ties keep the JS `Array.prototype.sort` order (Gourmet entries were appended
   before Ventopay entries).

Entry identity for list rendering: `g-{billNr}` for Gourmet, `v-{id}` for Ventopay
(v1: src/app/app/(tabs)/billing.tsx:149-153) — i.e. bill numbers and transaction IDs are
assumed unique within a month.

### 6.3 Totals (v1: src/app/app/(tabs)/billing.tsx:88-106)

Computed for the selected month, respecting the source filter:

- `gourmetTotal` = the month's `totalBilling` (Σ of per-bill `billing`, i.e. after
  subsidy/discount), or 0 if filtered out / no data.
- `ventopayTotal` = the month's `total` (Σ of `amount`), or 0 if filtered out / no data.
- `total` = `gourmetTotal + ventopayTotal`.
- `subsidy` = the month's Gourmet `totalSubsidy` when `sourceFilter !== 'ventopay'`,
  else 0.
- `count` = number of entries in the filtered unified list (bills + transactions).

There is no per-day or per-category grouping — the list is flat, one card per
bill/transaction.

---

## 7. Refresh behavior (screen lifecycle)

(v1: src/app/app/(tabs)/billing.tsx:110-120)

Every time the Billing screen gains focus (including the first display and every return
to the tab), **and additionally whenever an auth status changes while the screen stays
focused** — the focus effect's callback depends on `hasAnyAuth` and both auth statuses,
and `useFocusEffect` re-executes when the callback identity changes while focused, so a
login/session-restore completing with the Billing tab open immediately triggers the
sequence below without a refocus (v1: src/app/app/(tabs)/billing.tsx:110-120 —
deps `[hasAnyAuth, gourmetAuthStatus, ventopayAuthStatus, …]`):

1. Emit the analytics signal `screen.viewed` with `{ screen: 'billing' }` (see
   `03-features/analytics.md`).
2. If at least one source is authenticated:
   a. Run `loadCachedMonths()` (errors ignored), then — regardless of whether it
      succeeded —
   b. `fetchBilling()` if the Gourmet auth status is `'authenticated'`, and
   c. `fetchVentopayBilling()` if the Ventopay auth status is `'authenticated'`.

Both fetches target the currently selected month (no argument → `selectedMonthIndex`).
Combined with the skip rules in section 4, the effective policy is
**stale-while-revalidate**: cached data appears immediately; the current month is
re-fetched on every focus; past months are re-fetched only if they have no non-empty
in-memory data yet.

There is no pull-to-refresh gesture and no timed auto-refresh on this screen in v1.

Authentication state itself (login, demo mode, credential storage) is owned by
`03-features/settings.md`; note that in demo mode the auth stores expose demo API
implementations behind the same `getBillings`/`getTransactions` interfaces (see
`03-features/demo-mode.md`), and the billing feature is agnostic to which is active.

---

## 8. Display rules

Visual styling (colors, surfaces, blur, typography) is owned by `04-ui-ux.md`. Behavioral
display rules for billing:

### 8.1 Screen structure (mobile) (v1: src/app/app/(tabs)/billing.tsx:188-284)

Top to bottom:

1. **Month selector**: 3 equal-width tabs labeled with the German month labels
   (section 2.1), selected tab highlighted. Tapping calls `selectMonth(index)`.
2. **Source filter**: 3 pill buttons `Alle` / `Kantine` / `Automaten`.
3. **Error banner**: shown whenever `error != null`, displaying the error string.
   It coexists with data (data stays visible below it).
4. **Summary bar** (only when the filtered list is non-empty):
   - `Gesamt` → `total` formatted as currency
   - `Belege` → `count` (integer)
   - `Zuschuss` → `subsidy` formatted as currency, rendered in the success color —
     shown **only when subsidy > 0** (v1: src/app/app/(tabs)/billing.tsx:255-262)
5. **Entry list**: one card per entry (section 8.3), newest first.

### 8.2 Screen states

- **Neither source authenticated**: only the centered text `Anmeldung erforderlich`
  (v1: src/app/app/(tabs)/billing.tsx:122-128). Month selector/filters are not shown.
- **Loading with no data yet**: centered large activity spinner — shown only while
  `loading && entries.length === 0`; once any data exists, refreshes happen without a
  spinner (v1: src/app/app/(tabs)/billing.tsx:140-144, 237-241).
- **Not loading, no data**: centered text `Keine Abrechnungsdaten für diesen Monat`
  (v1: src/app/app/(tabs)/billing.tsx:159-163, 278-282).

### 8.3 Cards (v1: src/app/src-rn/components/BillCard.tsx)

Common header for both card types: date on the left as two lines — formatted date above,
time below — and on the right a source badge above the amount.

- **Date format**: locale `de-AT` with `{ weekday: 'short', day: 'numeric', month:
  'short', year: 'numeric' }` (v1: BillCard.tsx:22-29), e.g. "Mi., 8. Juli 2026"-style
  short forms per platform ICU.
- **Time format**: locale `de-AT` with `{ hour: '2-digit', minute: '2-digit' }`
  (v1: BillCard.tsx:31-36).
- **Currency format** (used everywhere in this feature): locale `de-AT`,
  `{ style: 'currency', currency: 'EUR' }` (v1: BillCard.tsx:18-20 and
  billing.tsx:30-32).

**Gourmet card** (v1: BillCard.tsx:38-69):
- Badge text `Kantine` (rendered uppercase via styling).
- Amount = `bill.billing` (post-subsidy total), highlighted in the primary color.
- Body: one row per item: `{count}x` | `description` (truncated to 1 line) |
  `formatCurrency(item.total)`. Per-item subsidy/discount are NOT displayed.
- `location` is not displayed.

**Ventopay card** (v1: BillCard.tsx:71-96):
- Badge text `Automaten` (rendered uppercase via the same shared `badgeText` styling as
  the Gourmet badge — `textTransform: 'uppercase'`, v1: BillCard.tsx:150-156).
- Amount = `transaction.amount`, highlighted in the primary color — the **same**
  `billing` text style as the Gourmet card amount (v1: BillCard.tsx:52,85,134-138);
  only the badge accent colors differ between the two card types.
- Body: single line with `transaction.restaurant` (truncated to 1 line), omitted when
  empty. Individual purchased items are NOT displayed (the scraper aggregates only the
  transaction total; see `02-ventopay-scraping.md`). `location` is not displayed.

The two badges use different accent colors (Gourmet = primary tint, Ventopay = success
tint; exact colors in `04-ui-ux.md`) so sources are distinguishable in the merged list.

---

## 9. Dropped in v2

- **Wide/desktop layout**: v1 renders an alternative two-pane layout (left filter panel
  `BillingFiltersPanel` with month list, source list, and an `Übersicht` summary showing
  the same `Gesamt`/`Belege`/`Zuschuss` values; bill list on the right) when
  `isDesktop() && windowWidth >= 700` (v1: src/app/app/(tabs)/billing.tsx:167-186,
  src/app/src-rn/components/BillingFiltersPanel.tsx, src/app/src-rn/hooks/useDesktopLayout.ts:24).
  The condition can only be true on the Tauri desktop target — never on iOS/Android —
  so the entire wide-layout branch and `BillingFiltersPanel` are dropped. Only the mobile
  layout (section 8.1) carries to v2.
- **Compact desktop card metrics**: `isCompactDesktop` (smaller paddings/fonts in
  `BillCard`) is desktop-only (v1: src/app/src-rn/utils/platform.ts:23); v2 uses the
  non-compact values.
- The Android/desktop "flat style" vs iOS "glass style" split (`useFlatStyle`, v1:
  src/app/src-rn/utils/platform.ts:20) remains relevant for iOS vs Android and is owned
  by `04-ui-ux.md` / `03-features/themes.md`.

---

## 10. Test-encoded edge cases (summary)

From v1: src/app/src-rn/__tests__/store/billingStore.test.ts —

| Behavior | Test lines |
|---|---|
| Exactly 3 month options, offsets 0/1/2, German labels | 96-116 |
| Offset passed to Gourmet API as string (`'0'`, `'1'`) | 119-124, 270-280 |
| Totals computed and stored per month (`totalBilling`) | 127-138 |
| Cache write under `billing_*` / `ventopay_billing_*` keys | 140-150, 342-350 |
| Gourmet error → `error` set to message, `loading` false | 152-159 |
| Non-`Error` rejection → fallback `Abrechnung konnte nicht geladen werden` | 282-288 |
| Past month with non-empty data → fetch skipped (both sources) | 161-182, 303-321 |
| In-flight `loading` → Gourmet fetch skipped | 290-294 |
| Ventopay date range starts on day 1 of month | 186-197 |
| Ventopay unauthenticated → fetch skipped silently | 323-329 |
| Ventopay failure → warn only, `error` stays null | 331-340 |
| Out-of-range month index → getters null, fetches no-op | 257-268, 298-301 |
| `selectMonth` triggers both fetches | 223-231 |
| Cache restore revives `Date` objects, recomputes totals, `fetchedAt` = 0 | 353-382 |

---

## Open questions

- Exact rendered strings for currency/date/time depend on the platform's `de-AT` ICU
  data (v1 runs on Hermes' Intl). Source pins only the locale and format options, not
  byte-exact output (e.g. `€ 3,00` vs `3,00 €`); v2 should use native `de-AT` locale
  formatting with the same options rather than hardcoded strings.
- Whether the Gourmet server scopes `GetMyBillings` responses strictly to the requested
  `checkLastMonthNumber` month (the store assumes yes and does no client-side date
  filtering) cannot be confirmed from the billing-layer source; server behavior belongs
  to `01-gourmet-scraping.md`.
