# Demo mode

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

Demo mode lets the app run fully offline with plausible canned data. Its stated purpose is
App Store review (v1: src/app/src-rn/utils/constants.ts:22, comment "Demo mode credentials
(for App Store review)"). It performs **zero network requests** — demo credentials must never
reach the live Gourmet or Ventopay servers (v1: src/app/src-rn/utils/backgroundMenuCheck.ts:44-45).

Related docs: the live client interfaces the demo implementations mirror are specified in
`01-gourmet-scraping.md` and `02-ventopay-scraping.md`. Consumers of the API objects
(menu/order/billing flows, caching) are in `03-features/menus.md`, `03-features/orders.md`,
`03-features/billing.md`, `03-features/caching.md`. CLAUDE.md does not mention demo mode;
this doc is derived entirely from code.

---

## 1. Activation trigger

Demo mode is activated purely by entering **magic credentials** on either login form. There is
no setting, build flag, or hidden gesture.

```
DEMO_USERNAME = 'demo'
DEMO_PASSWORD = 'demo1234!'
```

(v1: src/app/src-rn/utils/constants.ts:23-24)

Match rule (copy exactly — username is case-insensitive, password is case-sensitive):

```ts
function isDemoCredentials(username: string, password: string): boolean {
  return username.toLowerCase() === DEMO_USERNAME && password === DEMO_PASSWORD;
}
```

(v1: src/app/src-rn/utils/constants.ts:26-28)

The check happens at three call sites:

1. **Gourmet login** (v1: src/app/src-rn/store/authStore.ts:35-42). If credentials match:
   - Construct a **fresh** demo Gourmet API instance and "log in" to it (always succeeds,
     arguments ignored).
   - **Clear the menu cache**: the menu store is reset to `{ items: [], lastFetched: null }` so
     previously cached live menus do not leak into the demo session
     (v1: src/app/src-rn/store/authStore.ts:38). This clear happens only on the demo branch,
     not on live login.
   - Replace the auth store's `api` object with the demo instance. All downstream feature flows
     resolve the API through the auth store at call time
     (`useAuthStore.getState().api` — v1: src/app/src-rn/store/menuStore.ts:93,
     orderStore.ts:64, billingStore.ts:213), so menus, orders, and Gourmet billing transparently
     run against demo data.
   - Status becomes `authenticated`; the analytics signal `auth.loginSuccess` with
     `{ service: 'gourmet' }` fires exactly as for a live login — analytics does **not**
     distinguish demo from live (v1: src/app/src-rn/store/authStore.ts:40).
2. **Ventopay login** (v1: src/app/src-rn/store/ventopayAuthStore.ts:34-40). Analogous: a fresh
   demo Ventopay instance is swapped into the Ventopay auth store's `api`; `auth.loginSuccess`
   `{ service: 'ventopay' }` fires. No cache clearing here.
3. **Background menu check** (v1: src/app/src-rn/utils/backgroundMenuCheck.ts:46-49). Before the
   headless background task logs in with saved credentials, it checks `isDemoCredentials`; on
   match it appends a notification-log entry `('menu-check', 'guard', 'demo_credentials_skip')`
   and returns success **without any network activity** (see
   `03-features/notifications-new-menu.md`).

The two services activate independently: demo credentials on the Kantine form only demo the
Gourmet side; demo credentials on the Automaten form only demo the Ventopay side. Mixed
live/demo operation is possible.

### Persistence across restarts

The login screens save the entered credentials to secure storage *before* attempting login
(v1: src/app/app/kantine-login.tsx:61-62, src/app/app/automaten-login.tsx:60-61 — note: saved
even if the login subsequently fails). Demo credentials are therefore persisted like real ones,
and the app-start auto-login (`loginWithSaved`, v1: src/app/src-rn/store/authStore.ts:55-62,
invoked from the root layout on mount, v1: src/app/app/_layout.tsx:44-45) re-enters demo mode
automatically on every launch until the user replaces the credentials.

### Quirk: demo API is sticky for the rest of the process

Neither `logout` (v1: src/app/src-rn/store/authStore.ts:64-71) nor anything else ever restores
the auth store's `api` field to a live client — the live instance is created only once at store
construction (v1: src/app/src-rn/store/authStore.ts:30). Consequence: after a demo login, a
subsequent login **with real credentials in the same app session** takes the non-demo branch
(`isDemoCredentials` is false) but calls `login()` on the still-installed demo instance, which
ignores the arguments and succeeds — the app keeps serving demo data until it is fully
restarted. The same holds for Ventopay (v1: src/app/src-rn/store/ventopayAuthStore.ts:29,41).
This looks unintentional but is v1's actual behavior; see §8.

---

## 2. What is indicated to the user

There is **no explicit demo badge, banner, or watermark**. The only visible indication is the
fake account name: demo login returns `username: 'Demo User'`, and the Settings screen shows
`Angemeldet als Demo User` when authenticated (v1: src/app/app/(tabs)/settings.tsx:62-64).
The Ventopay side shows only its generic connected status `Sitzung aktiv`
(v1: src/app/app/(tabs)/settings.tsx:79-81) — no demo indication at all.

---

## 3. Demo Gourmet API — behavior contract

A stateful in-memory object with the same public surface as the live Gourmet API
(v1: src/app/src-rn/api/demoGourmetApi.ts). Per-instance state: `userInfo`, `orders[]`, and a
lazily generated, instance-cached `menus[]`. A fresh instance is created on every demo login.

| Operation | Behavior |
|---|---|
| `login(username, password)` | Ignores arguments; never fails. Sets and returns user info: `{ username: 'Demo User', shopModelId: 'demo-shop-1', eaterId: 'demo-eater-1', staffGroupId: 'demo-staff-1' }` (v1: demoGourmetApi.ts:19-27) |
| `getUserInfo()` | The stored user info, or null before login / after logout (v1: demoGourmetApi.ts:29-31) |
| `getMenus()` | Generates the menu set on first call and caches it on the instance; every call returns copies with `ordered` recomputed as: an order exists whose `date.getTime() === item.day.getTime()` **and** whose `subtitle === item.subtitle`. Matching is by subtitle+day, *not* by id, because `title` holds the category name (v1: demoGourmetApi.ts:33-46) |
| `getOrders()` | Shallow copy of the in-memory order list (v1: demoGourmetApi.ts:48-50) |
| `addToCart(items: {date, menuId}[])` | Generates menus first if not yet cached. For each entry, finds the cached menu item with `m.id === menuId && m.day.getTime() === date.getTime()`; creates an order with `title = menuItem?.title ?? 'Demo Menü'`, `subtitle = menuItem?.subtitle ?? ''`, `approved: false` (v1: demoGourmetApi.ts:52-70). The order does not retain the menuId (see §6) |
| `confirmOrders()` | Marks **all** orders `approved: true` (v1: demoGourmetApi.ts:72-74) |
| `cancelOrders(positionIds)` | Removes orders whose `positionId` is in the given set (v1: demoGourmetApi.ts:76-79) |
| `getBillings(checkLastMonthNumber)` | Regenerated on every call from the generator in §5.3 (deterministic per target month) (v1: demoGourmetApi.ts:81-83) |
| `logout()` | Clears user info, orders, and the cached menus (v1: demoGourmetApi.ts:85-89) |
| `isAuthenticated()` | `userInfo !== null` (v1: demoGourmetApi.ts:91-93) |

## 4. Demo Ventopay API — behavior contract

Stateless except for a `loggedIn` boolean (v1: src/app/src-rn/api/demoVentopayApi.ts):

| Operation | Behavior |
|---|---|
| `login(username, password)` | Ignores arguments; never fails; sets `loggedIn = true` (v1: demoVentopayApi.ts:11-13) |
| `getTransactions(fromDate, toDate)` | Generated per call from the generator in §5.4 (v1: demoVentopayApi.ts:15-17) |
| `logout()` | `loggedIn = false` (v1: demoVentopayApi.ts:19-21) |
| `isAuthenticated()` | Returns `loggedIn` (v1: demoVentopayApi.ts:23-25) |

---

## 5. Canned data generation (v1: src/app/src-rn/api/demoData.ts)

All date math uses the **local device clock/timezone** (`new Date()`).

### 5.1 Deterministic PRNG

A linear congruential generator, copied exactly (v1: demoData.ts:8-14):

```ts
function seededRandom(seed: number): () => number {
  let s = seed;
  return () => {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    return s / 0x7fffffff;
  };
}
```

Each generated value is in [0, 1). Draw *order* determines the data — it is specified per
generator below. Note the arithmetic must match JavaScript semantics: the multiply is done in
double-precision floating point and only the result of the addition is truncated by the 32-bit
`& 0x7fffffff`. Seeds:

| Generator | Seed | Effect |
|---|---|---|
| Menus | `year*10000 + (month+1)*100 + dayOfMonth` of today (v1: demoData.ts:16-19) | Data stable within one calendar day, rotates daily |
| Gourmet billing | `actualYear*100 + actualMonth` of the *target* month, where `actualMonth` is the 0-based month index (v1: demoData.ts:177) | Stable per billed month |
| Ventopay transactions | `now.getFullYear()*100 + now.getMonth()` (0-based, **current** month regardless of the requested range) (v1: demoData.ts:212) | Stable within the current calendar month |

### 5.2 Menus — `generateDemoMenus()` (v1: demoData.ts:118-162)

- Start day: the **Monday of the current week**. With `dow = today.getDay()`,
  `diff = dow === 0 ? -6 : 1 - dow` (Sunday counts as belonging to the week that started six
  days earlier) (v1: demoData.ts:124-127).
- Collect the next **10 weekdays** (Mon–Fri, skipping Sat/Sun) starting from that Monday,
  each normalized to local midnight (v1: demoData.ts:89-102,129). On weekends this means the
  first five of the ten days lie entirely in the past.
- For each day (outer loop, day index 0–9), for each of 4 pools **in this order** —
  Menu 1, Menu 2, Menu 3, Soup & Salad — draw one `rand()` and pick
  `dishIndex = (dayIndex + Math.floor(rand() * 3)) % 10` from the pool (v1: demoData.ts:139-144).
  That is 4 PRNG draws per day, 40 total.
- Emitted item fields (v1: demoData.ts:146-157):
  - `id`: `` `${idPrefix}-${dayIndex}` `` — e.g. `demo-m1-0` … `demo-ss-9`. IDs are unique per
    (category, day-index) pair, mirroring the live system's per-category ID semantics.
  - `day`: the weekday Date (local midnight).
  - `title`: the **category display string** — `'MENÜ I'`, `'MENÜ II'`, `'MENÜ III'`, or
    `'SUPPE & SALAT'` (the `GourmetMenuCategory` enum values, v1: src/app/src-rn/types/menu.ts:13-19).
    Title deliberately holds the category text so category detection works; the dish goes in
    the subtitle (v1: demoData.ts:149 comment).
  - `subtitle`: `` `${dish.title} ${dish.subtitle}` `` (single space join).
  - `allergens`: the dish's allergen letters array.
  - `available: true`, `ordered: false` for every item — including past days of the current week.
  - `category`: the pool's category enum value.
  - `price`: `'6,00 €'` for all three Menu pools, `'2,50 €'` for Soup & Salad (v1: demoData.ts:133-136).

Pool parameters (v1: demoData.ts:132-137):

| Pool | Category | `idPrefix` | `price` |
|---|---|---|---|
| MENU1_DISHES | `MENÜ I` | `demo-m1` | `6,00 €` |
| MENU2_DISHES | `MENÜ II` | `demo-m2` | `6,00 €` |
| MENU3_DISHES | `MENÜ III` | `demo-m3` | `6,00 €` |
| SOUP_SALAD_DISHES | `SUPPE & SALAT` | `demo-ss` | `2,50 €` |

Dish pools, copied verbatim (v1: demoData.ts:29-79). Each row: title / subtitle / allergens.

**MENU1_DISHES** (v1: demoData.ts:29-40):

| # | title | subtitle | allergens |
|---|---|---|---|
| 0 | Wiener Schnitzel | mit Petersilerdäpfel und Preiselbeeren | A, C, G |
| 1 | Schweinsbraten | mit Semmelknödel und Sauerkraut | A, C, G |
| 2 | Tafelspitz | mit Apfelkren und Schnittlauchsauce | A, G, L |
| 3 | Rindsgulasch | mit Nockerl und Essiggurkerl | A, C, G |
| 4 | Backhendl | mit Erdäpfelsalat | A, C, G |
| 5 | Cordon Bleu | mit Reis und Preiselbeeren | A, C, G |
| 6 | Zwiebelrostbraten | mit Bratkartoffeln und Röstzwiebeln | A, G, L |
| 7 | Faschierter Braten | mit Erdäpfelpüree und Bratensauce | A, C, G |
| 8 | Kalbsrahmgulasch | mit Butternockerl | A, C, G |
| 9 | Gebackene Leber | mit Erdäpfelsalat und Preiselbeeren | A, C, G |

**MENU2_DISHES** (v1: demoData.ts:42-53):

| # | title | subtitle | allergens |
|---|---|---|---|
| 0 | Gemüselasagne | mit Blattsalat | A, C, G |
| 1 | Spinatknödel | mit Parmesan und brauner Butter | A, C, G |
| 2 | Käsespätzle | mit Röstzwiebeln und grünem Salat | A, C, G |
| 3 | Pasta Primavera | mit Saisongemüse und Basilikum | A, C |
| 4 | Kartoffelgratin | mit buntem Gemüse | A, G |
| 5 | Topfenknödel | mit Butterbröseln und Apfelmus | A, C, G |
| 6 | Gemüse-Curry | mit Basmatireis und Naan-Brot | A, G |
| 7 | Flammkuchen | mit Sauerrahm, Zwiebeln und Speck | A, G |
| 8 | Eierschwammerlgulasch | mit Semmelknödel | A, C, G |
| 9 | Palatschinken | mit Topfenfülle und Vanillesauce | A, C, G |

**MENU3_DISHES** (v1: demoData.ts:55-66):

| # | title | subtitle | allergens |
|---|---|---|---|
| 0 | Grillhendl | mit Pommes frites und Cole Slaw | A, G, M |
| 1 | Fischfilet | mit Dillsauce und Petersilerdäpfel | A, C, D, G |
| 2 | Putengeschnetzeltes | mit Reis und Champignons | A, G |
| 3 | Cevapcici | mit Djuvec-Reis und Ajvar | A, C |
| 4 | Hühnercurry | mit Jasminreis und Mango-Chutney | A, G |
| 5 | Leberkäse | mit Spiegelei und Erdäpfelsalat | A, C, G |
| 6 | Bratwürstel | mit Senf und Sauerkraut | A, M |
| 7 | Puten-Wrap | mit Salat, Tomaten und Joghurt-Dressing | A, G |
| 8 | Lachs gegrillt | mit Zitronenbutter und Gemüsereis | D, G |
| 9 | Spaghetti Bolognese | mit Parmesan | A, C, G |

**SOUP_SALAD_DISHES** (v1: demoData.ts:68-79):

| # | title | subtitle | allergens |
|---|---|---|---|
| 0 | Frittatensuppe | Klare Rindsuppe mit Frittaten | A, C, G |
| 1 | Kürbiscremesuppe | mit Kürbiskernöl und Croutons | A, G |
| 2 | Gemischter Salat | mit Hausdressing | M |
| 3 | Grießnockerlsuppe | Klare Suppe mit Grießnockerl | A, C, G |
| 4 | Tomatencremesuppe | mit Basilikum und Croutons | A, G |
| 5 | Kartoffelsuppe | mit Einlage und Brot | A, G, L |
| 6 | Caesar Salad | mit Hühnerstreifen und Parmesan | A, C, G |
| 7 | Leberknödelsuppe | Klare Rindsuppe mit Leberknödel | A, C, G |
| 8 | Gemüsesuppe | mit frischem Saisongemüse | A, L |
| 9 | Blattsalat | mit Kernöl-Dressing und Kürbiskernen | H |

Expected shape (encoded in tests, v1: src/app/src-rn/__tests__/api/demoGourmetApi.test.ts:41-76):
exactly 40 items (10 weekdays × 4 categories), all `available`, all four categories present,
no Saturday/Sunday items, identical results across repeated `getMenus()` calls.

### 5.3 Gourmet billing — `generateDemoBillings(checkLastMonthNumber)` (v1: demoData.ts:164-208)

- `monthOffset = parseInt(checkLastMonthNumber, 10) || 0` — non-numeric input and `"0"` both
  yield 0 (v1: demoData.ts:165).
- Target month = current month minus `monthOffset` (Date-constructor normalization handles
  year underflow) (v1: demoData.ts:166-171).
- Bill days: all Mon–Fri weekdays of the target month that are `<=` today (today extended to
  local 23:59:59.999). Past months therefore include every weekday; the current month includes
  weekdays up to and including today; future target months produce an empty list
  (v1: demoData.ts:104-114,173-176).
- PRNG seeded per target month (§5.1). **Two draws per bill day, in order**: description index,
  then price index (v1: demoData.ts:182-183).
- One bill per weekday, `i` = index in the weekday list (v1: demoData.ts:187-204):
  - `billNr`: `100000 + i`
  - `billDate`: the weekday (local midnight)
  - `location`: `'Betriebsrestaurant'`
  - exactly one item: `id` = `` `demo-bill-item-${i}` ``, `articleId` = `` `demo-art-${descIndex}` ``,
    `count: 1`, `description` = `BILLING_DESCRIPTIONS[Math.floor(rand() * 4)]`,
    `total` = `BILLING_PRICES[Math.floor(rand() * 10)]`, `subsidy: 1.50`, `discountValue: 0`,
    `isCustomMenu: false`
  - `billing` = `total - 1.50` (price minus subsidy)

Constant pools, verbatim (v1: demoData.ts:81-85):

```
BILLING_DESCRIPTIONS = ['Menü I', 'Menü II', 'Menü III', 'Suppe & Salat']
BILLING_PRICES = [6.80, 5.90, 6.20, 4.20, 5.50, 6.50, 5.80, 6.00, 4.80, 5.20]
```

Note the descriptions use mixed case (`'Menü I'`), unlike the menu category strings (`'MENÜ I'`).

### 5.4 Ventopay transactions — `generateDemoTransactions(fromDate, toDate)` (v1: demoData.ts:210-247)

- Iterate day by day from `fromDate` (normalized to local 00:00:00.000) through `toDate`
  (normalized to local 23:59:59.999), weekdays (Mon–Fri) only.
- PRNG seeded on the **current** month (§5.1) — the same range re-requested within one calendar
  month yields identical results; the seed ignores the requested range entirely.
- Per weekday: **one draw** (`roll`); a transaction is emitted only if `roll < 0.4`
  (~40 % of weekdays). If emitted, **three more draws in order**: amount index, hour, minute
  (v1: demoData.ts:224-232).
- Emitted transaction (v1: demoData.ts:227-240):
  - `id`: `` `demo-vp-${id}` `` where `id` is a 0-based counter incremented only per emitted
    transaction within this call
  - `date`: the weekday at `hour = 7 + Math.floor(rand() * 4)` (07–10 local),
    `minute = Math.floor(rand() * 60)`, seconds/ms zero
  - `amount`: one of `[0.50, 1.00, 1.20, 1.50, 2.00, 2.50]`
  - `restaurant`: `'Kaffeeautomat'`
  - `location`: `'Kaffeeautomat EG'`

Because `restaurant` never contains "Gourmet", every demo transaction would pass the live-mode
Gourmet/Kaffeeautomat filter rule documented in `02-ventopay-scraping.md` (the demo path does
not run that filter — data is generated post-parse).

Determinism is asserted by tests (v1: src/app/src-rn/__tests__/api/demoVentopayApi.test.ts:41-52).

---

## 6. Demo order identifiers (v1: demoData.ts:251-268)

Orders are created with a **module-level** counter starting at 1:

- `positionId`: `` `demo-pos-${id}` ``
- `eatingCycleId`: `` `demo-cycle-${id}` ``
- `date`, `title`, `subtitle` as passed in; `approved: false`

The counter is never reset — it survives logout and new demo-API instances for the lifetime of
the process, so IDs keep incrementing across demo sessions within one app run. The `menuId`
argument is accepted but **not stored** on the order (v1: demoData.ts:253-268), which is why
ordered-state matching in `getMenus()` uses day + subtitle.

---

## 7. Behavioral differences vs live mode (summary)

| Aspect | Live | Demo |
|---|---|---|
| Network | Scrapes Gourmet/Ventopay | None whatsoever |
| Login | Real credential check, can fail | Always succeeds; args ignored |
| User info | Parsed from the site | Fixed `Demo User` / `demo-shop-1` / `demo-eater-1` / `demo-staff-1` |
| Menus | Parsed from paginated HTML | 40 generated items, rotate daily (seeded), all `available: true` (even past days of the current week) |
| Orders | Server-side, survive restarts | In-memory only; lost on logout and app restart |
| Confirm | Form POST per current cart | Sets `approved: true` on **all** in-memory orders |
| Cancel | Edit-mode form workflow | Filters the in-memory list by positionId |
| Billing | JSON API | Regenerated per call, deterministic per month, subsidy fixed at 1.50 |
| Transactions | Scraped + Gourmet/Kaffeeautomat filter | Generated coffee purchases only; filter not exercised |
| Background menu check | Logs in and fetches | Skipped with log entry `demo_credentials_skip` (v1: backgroundMenuCheck.ts:46-48) |
| Analytics | `auth.loginSuccess` etc. | Identical signals — demo is indistinguishable in analytics |
| Menu cache | Cached with `lastFetched` (see `03-features/caching.md`) | Cache is cleared on demo login; demo API keeps its own per-instance menu snapshot |

Other background tasks (geofence order sync, daily/cancel reminders) operate exclusively on
cached orders without network logins (v1: src/app/src-rn/utils/notificationTasks.ts:86-87), so
they need no demo guard.

**Dropped in v2:** none of the demo code has desktop/web branches; nothing to drop. The demo
credentials' persistence path uses the platform secure storage described in
`05-platform-services` (v1's web `localStorage` fallback is dropped with the web target).

---

## 8. Open questions

- Is the sticky-demo-API-until-restart quirk (§1) a bug to fix in v2 or behavior to replicate?
  Nothing in v1 source or docs states the intent.
- Is daily menu rotation (menu seed changes at local midnight, §5.1) intentional, or should v2
  pick a different stability window? v1 gives no rationale beyond the seed formula.
- Whether App Store review actually depends on the exact strings `demo` / `demo1234!` (e.g.
  submitted in review notes) cannot be determined from source; v2 must keep them identical
  unless the review notes are updated.
