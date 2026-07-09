# 06 — Testing strategy & fixtures

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This doc specifies the v1 test architecture — the record & replay fixture strategy, the
full fixture inventory, the recorder script, sanitization rules — and what the v2 Rust
core test suite must replicate. HTTP request specs themselves live in
`01-gourmet-scraping.md` and `02-ventopay-scraping.md`; feature behavior lives in
`03-features/*`. This doc only covers how that behavior is *verified*.

---

## 1. v1 test harness (v1 mechanism)

v1 runs Jest 29 (`jest ~29.7.0`, `ts-jest ^29.4.11`, `@types/jest 29.5.14` — v1:
src/app/package.json) in a plain Node environment — tests never touch a device,
simulator, or the network. Configuration (v1: src/app/jest.config.js:1-15):

- `testEnvironment: 'node'`
- `testMatch: ['**/src-rn/__tests__/**/*.test.ts']`
- `setupFilesAfterEnv: ['./src-rn/__tests__/setup.ts']`
- ts-jest transform for `^.+\.tsx?$` using `tsconfig.json`
- `moduleNameMapper` (module-level mocks applied to *all* suites):
  - `'.*/utils/analytics$'` → `src-rn/utils/__mocks__/analytics.ts` (exports jest-fn
    stubs `td.signal`, `setDefaultPayload`, `trackSignal` — v1:
    src/app/src-rn/utils/__mocks__/analytics.ts:1-3)
  - `'.*/utils/notificationLogStorage$'` → `src-rn/utils/__mocks__/notificationLogStorage.ts`
    (stubs `getLogActivatedUntil`→null, `activateLog`, `clearLog`, `isLogActive`→false,
    `getLogEntries`→[], `appendLogEntry`, `formatLogForEmail`→'' — v1:
    src/app/src-rn/utils/__mocks__/notificationLogStorage.ts:1-7)

Global setup (v1: src/app/src-rn/__tests__/setup.ts:1-36) mocks the RN runtime:

- `@react-native-async-storage/async-storage` → in-memory `Record<string,string>` with
  `getItem`/`setItem`/`removeItem`/`clear`/`getAllKeys`
- `expo-secure-store` → `getItemAsync` resolves `null`, `setItemAsync`/`deleteItemAsync` no-ops
- `react-native` → `{ Platform: { OS: 'ios', select: (spec) => spec.ios ?? spec.default } }`
  (i.e. the whole suite runs "as iOS")

Scripts (v1: src/app/package.json): `npm test` (= `jest`), `npm run test:watch`,
`npm run test:coverage`, `npm run record-fixtures` (= `tsx scripts/record-fixtures.ts`).

**Actual counts at 6997c44** (verified by running `npx jest`): **27 test suites,
435 tests, all passing**. Discrepancy: CLAUDE.md claims "178 tests" across "13 test
files" (v1: CLAUDE.md, Testing section) — that reflects the original test-infrastructure
commit (7955ab6) and is outdated; the code wins. (The number 13 survives accurately only
as the *fixture* count.)

### v2 equivalent

The v2 Rust core test suite replaces this harness: `cargo test` with fixtures loaded from
disk and HTTP mocked at the client boundary. The RN-runtime mocks above have no v2
equivalent (they exist only because v1 stores import RN modules); v2 core tests should
inject storage/clock/HTTP as traits instead.

---

## 2. Record & replay strategy

The strategy: **all parser and orchestration tests run against static HTML/JSON fixture
files checked into the repo; no test performs network I/O.** HTTP clients are mocked and
made to return fixture strings; assertions then check (a) the exact request shapes the
code produced and (b) the exact parsed values.

Two important facts an implementer must know:

1. **The committed fixtures are hand-authored synthetic replicas, not recorder output.**
   They contain sentinel values like `ufprt="CSRF-TOKEN-LOGIN-ABC123"` and
   `data-id="menu-001"`, are 250 B–4 KB each, and were created together with the tests
   (v1 commit 7955ab6 "Add test infrastructure with 178 tests across 13 suites"). The
   recorder script's sanitization (§4) only replaces usernames/IDs — it would never
   produce these token sentinels, and real pages would be far larger.
2. **Tests assert the sentinel values exactly** (e.g. login POST must carry
   `ufprt: 'CSRF-TOKEN-LOGIN-ABC123'` — v1: src/app/src-rn/__tests__/api/gourmetApi.test.ts:62-68).
   Therefore running `npm run record-fixtures` against the live sites *overwrites* the
   fixtures and breaks these assertions. In practice the recorder is a research tool: it
   captures fresh real HTML so a developer can check the live markup still matches, then
   manually folds any structural changes back into the synthetic fixtures.

Discrepancy: CLAUDE.md says "Tests use a record & replay strategy with sanitized HTML
fixtures", implying the fixtures are sanitized recordings. The code wins: fixtures are
synthetic, recorder output is a maintenance aid.

**v2 requirement**: keep the same strategy. The 13 fixtures were carried over verbatim to
`docs/fixtures/gourmet/` and `docs/fixtures/ventopay/` (commit 343ec9a); the v2 Rust core
must load these exact files in its tests and reproduce the assertions in §6.

---

## 3. Fixture inventory (13 files)

All under v1: src/app/src-rn/__tests__/fixtures/ (carried verbatim to `docs/fixtures/`).
Sentinel values listed here are asserted verbatim by tests — do not alter the files.

### Gourmet (9 files)

| File | Contents / what it covers |
|---|---|
| `gourmet/login-page.html` | Unauthenticated `/start/` page: one `<form method="post" action="/start/" enctype="multipart/form-data">` with inputs `Username`, `Password`, hidden `RememberMe="false"`, hidden `ufprt="CSRF-TOKEN-LOGIN-ABC123"`, hidden `__ncforminfo="NCFORM-TOKEN-LOGIN-XYZ789"`. Covers login-token extraction and `isLoggedIn()===false`. |
| `gourmet/login-failed.html` | Same login form (`ufprt="CSRF-TOKEN-LOGIN-FAILED-111"`, `__ncforminfo="NCFORM-TOKEN-LOGIN-FAILED-222"` — these particular sentinels are not asserted by any test) plus `<div class="alert alert-danger">Benutzername oder Passwort falsch.</div>`. Covers failed-login detection (no `/einstellungen/` link) and session-expiry detection in orchestration tests. **Not produced by the recorder** — hand-authored only. |
| `gourmet/login-success.html` | Authenticated page: `<span class="loginname">TestUser</span>`, the login-verification anchor `<a href="https://alaclickneu.gourmet.at/einstellungen/" class="navbar-link">`, logout form (`ufprt="CSRF-TOKEN-LOGOUT-DEF456"`, `__ncforminfo="NCFORM-TOKEN-LOGOUT-UVW012"`, button `id="btnHeaderLogout"`), and hidden user-info inputs `#shopModel="SM-TEST-123"`, `#eater="EATER-TEST-456"`, `#staffGroup="SG-TEST-789"`. Covers `isLoggedIn()===true`, `extractUserInfo`, `extractLogoutFormTokens`. |
| `gourmet/menus-page-0.html` | Menus page with next-page link `<a class="menues-next" href="/menus/?page=1">`. Desktop layout `<div class="row hide-sm-down">` holds **7** `div.meal` items across two days (`data-date="02-10-2026"` / `"02-11-2026"`, ids `menu-001`…`menu-004`), covering: all four categories (MENÜ I/II/III, SUPPE & SALAT), allergens `A, C, G`, an *ordered* item (`<input type="checkbox" class="menu-clicked" checked />`, subtitle "Schweinsbraten mit Knödel"), an *unavailable* item (no checkbox, subtitle "Gebratener Lachs mit Dillsauce"), an *empty-allergen* item (`<li class="allergen"></li>`, subtitle "Reis mit Gemüse"), price `€ 5,50`. Also contains a mobile-layout block `<div class="row hide-sm-up">` with a decoy item `data-id="menu-mobile-dup"` that the parser **must ignore** (expected count is 7, not 8). |
| `gourmet/menus-page-1.html` | Last menus page: 2 `div.meal` items (`data-date="02-12-2026"`), **no** `menues-next` link. Covers pagination termination. |
| `gourmet/orders-page.html` | `/bestellungen/` NOT in edit mode: logout form, main order form (`ufprt="CSRF-TOKEN-ORDERS-MAIN-333"`), edit-mode toggle `<form class="form-toggleEditMode">` with `editMode="True"`, `ufprt="CSRF-TOKEN-EDITMODE-555"`, `__ncforminfo="NCFORM-TOKEN-EDITMODE-666"`; 3 `div.order-item`s: POS-001 (approved via `<span class="fa fa-check">`), POS-002 (not approved), POS-003 (approved via `<span class="checkmark">`), each with `cp_PositionId`, `cp_EatingCycleId_{pos}` (EC-001…EC-003), `cp_Date_{pos}` (`10.02.2026 00:00:00` etc.). |
| `gourmet/orders-page-edit-mode.html` | `/bestellungen/` IN edit mode: toggle form has `editMode="False"`, `ufprt="CSRF-TOKEN-EDITMODE-EXIT-777"`, `__ncforminfo="NCFORM-TOKEN-EDITMODE-EXIT-888"`; per-order cancel forms `id="form_POS-001_cp"` (`ufprt="CSRF-TOKEN-CANCEL-POS001-AAA"`, `__ncforminfo="NCFORM-TOKEN-CANCEL-POS001-BBB"`) and `id="form_POS-002_cp"` (`…-CCC` / `…-DDD`). Covers cancel-form extraction and edit-mode state detection. **Not produced by the recorder.** |
| `gourmet/billing-current.json` | `GetMyBillings` response, `Billings` wrapper array with 2 bills: `BillNr` 10001 (2 `BillingItemInfo` items, `Billing: 4.50`; item fields `Id`, `ArticleId`, `Count`, `Description`, `Total`, `Subsidy`, `DiscountValue`, `IsCustomMenu`) and 10002 (1 item, `Billing: 3.00`); `BillDate` ISO local (`2026-02-10T12:00:00`), `Location: "Betriebsrestaurant Wien"`. |
| `gourmet/billing-last-month.json` | Same shape, 1 bill (`BillNr: 9501`, January date). Recorded/used for the `checkLastMonthNumber: '1'` variant. |

### Ventopay (4 files)

| File | Contents / what it covers |
|---|---|
| `ventopay/login-page.html` | `Login.aspx` form with the six ASP.NET hidden inputs: `__LASTFOCUS=""`, `__EVENTTARGET=""`, `__EVENTARGUMENT=""`, `__VIEWSTATE="VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123"`, `__VIEWSTATEGENERATOR="ABCD1234"`, `__EVENTVALIDATION="EVENTVALIDATION-TOKEN-XYZ789"`, plus `select name="DropDownList1"` (option value = the hardcoded company UUID `0da8d3ec-0178-47d5-9ccd-a996f04acb61`), `TxtUsername`, `TxtPassword`, `BtnLogin`, `languageRadio value="DE"`. |
| `ventopay/login-success.html` | Authenticated page containing the verification anchor `<a href="Ausloggen.aspx">Ausloggen</a>`. |
| `ventopay/transactions-page.html` | `Transaktionen.aspx` list with **6** `div.transact` entries inside `div.content`, ids are the base64-looking transaction IDs `dHhuLTAwMQ==`…`dHhuLTAwNg==`. Each entry: `<div class="transact_title">€ 1,80 (Café + Co. Automaten)</div>` and `<div class="transact_timestamp">09. Feb 2026 - 11:49 Uhr</div>`. Covers: German amount parsing (`1,80`→1.8), restaurant extraction from parentheses, timestamp parsing incl. Austrian month abbreviations `Mrz` (March) and `Jän` (January), and the Gourmet filter — the entry `€ 5,50 (Gourmet Betriebsrestaurant)` must be dropped (6→5) while both `Kaffeeautomat EG` entries are kept. |
| `ventopay/transactions-empty.html` | List page with no `div.transact` (text "Keine Transaktionen in diesem Zeitraum.") → parser must return `[]`. |

**Coverage gap to note**: there is no fixture for `Rechnung.aspx` (transaction detail),
and no v1 code fetches or navigates to it. The only references are a lowercase
`rechnung.aspx?id=...` URL in a JSDoc comment documenting the list-page markup
(ventopayParser.ts:116) and the anchor hrefs inside the transaction fixtures — nothing
scrapes it. The only Ventopay page URLs *defined* besides `Login.aspx` are
`VENTOPAY_TRANSACTIONS_URL` and `VENTOPAY_LOGOUT_URL`, alongside `VENTOPAY_BASE_URL`,
`VENTOPAY_LOGIN_URL`, and `VENTOPAY_COMPANY_ID` (v1: src/app/src-rn/utils/constants.ts:16-20;
no `Rechnung.aspx` constant exists). CLAUDE.md's "Transaction Details" section
(Rechnung.aspx, item-row columns) describes behavior that does not exist in v1 code —
code wins; v1 parses amount/restaurant/date directly from the list page. See
`02-ventopay-scraping.md` for the authoritative spec.

---

## 4. Recorder script (`npm run record-fixtures`)

v1: src/app/scripts/record-fixtures.ts, executed with `tsx` (v1: src/app/package.json,
`"record-fixtures": "tsx scripts/record-fixtures.ts"`). Node-only tool; it uses its own
axios instances with a `tough-cookie` `CookieJar` via `axios-cookiejar-support` (NOT the
app's clients), configured `maxRedirects: 5`, `validateStatus: (s) => s >= 200 && s < 400`
(v1: scripts/record-fixtures.ts:66-71, 197-202).

### .env requirements

Loaded via `dotenv` from the **project root** (`path.resolve(__dirname, '../../../.env')`,
i.e. two levels above `src/app/scripts/` — v1: scripts/record-fixtures.ts:18). Required
variables (script throws `Missing environment variable: {name}. Add it to .env` if absent
— v1: scripts/record-fixtures.ts:51-57):

```
KANTINE_USERNAME=
KANTINE_PASSWORD=
AUTOMATEN_USERNAME=
AUTOMATEN_PASSWORD=
```

(v1: .env.example:1-4; `.env` is gitignored. `.env.example` also defines `GITHUB_TOKEN`
for Velopack uploads — desktop-only, **Dropped in v2**.)

### Gourmet recording sequence (v1: scripts/record-fixtures.ts:63-188)

Base `https://alaclickneu.gourmet.at`; output dir `src-rn/__tests__/fixtures/`.

1. `GET {base}/start/` → save `gourmet/login-page.html`.
2. Extract `ufprt` + `__ncforminfo` from the first form (`form:first-of-type`,
   `input[name="ufprt"]` / `input[name="__ncforminfo"]` value attrs); throw if missing.
3. `POST {base}/start/` via axios `postForm` (multipart/form-data) with fields
   `Username`, `Password`, `RememberMe: 'false'`, `ufprt`, `__ncforminfo`.
   Success check: response HTML contains `/einstellungen/` **or** `loginname`
   (looser than the app's regex — recorder-only). Save `gourmet/login-success.html`.
4. Extract `#shopModel`, `#eater`, `#staffGroup` values and `span.loginname` text; if any
   ID is missing from the POST response, re-`GET {base}/start/` and extract from there.
5. `GET {base}/menus/` → save `gourmet/menus-page-0.html`.
6. While the last page contains `a[class*="menues-next"]` and page < 10:
   `GET {base}/menus/` with `params: { page: String(n) }` → save
   `gourmet/menus-page-{n}.html`. (Committed set stops at page 1.)
7. `GET {base}/bestellungen/` → save `gourmet/orders-page.html`.
8. `POST {base}/umbraco/api/AlaMyBillingApi/GetMyBillings` with JSON body
   `{ eaterId, shopModelId, checkLastMonthNumber: '0' }`, header
   `Content-Type: application/json` → save `gourmet/billing-current.json`.
9. Same POST with `checkLastMonthNumber: '1'` → save `gourmet/billing-last-month.json`.

Note the recorder does **not** record `gourmet/login-failed.html` or
`gourmet/orders-page-edit-mode.html` (those exist only as hand-authored fixtures), and it
never POSTs orders/cancellations (safe against real account state).

### Ventopay recording sequence (v1: scripts/record-fixtures.ts:194-284)

Base `https://my.ventopay.com/mocca.website`.

1. `GET {base}/Login.aspx` → save `ventopay/login-page.html`.
2. Extract `#__VIEWSTATE`, `#__VIEWSTATEGENERATOR`, `#__EVENTVALIDATION` (throw if any
   missing) plus `#__LASTFOCUS`, `#__EVENTTARGET`, `#__EVENTARGUMENT` (default `''`).
3. `POST {base}/Login.aspx` with `Content-Type: application/x-www-form-urlencoded`, body
   = `URLSearchParams` of `__LASTFOCUS, __EVENTTARGET, __EVENTARGUMENT, __VIEWSTATE,
   __VIEWSTATEGENERATOR, __EVENTVALIDATION, DropDownList1:
   '0da8d3ec-0178-47d5-9ccd-a996f04acb61', TxtUsername, TxtPassword, BtnLogin: 'Login',
   languageRadio: 'DE'`. Success check: regex `/href="Ausloggen\.aspx"/i`. Save
   `ventopay/login-success.html`.
4. `GET {base}/Transaktionen.aspx` with `params: { fromDate, untilDate }` where
   `fromDate = 01.{MM}.{yyyy}` (first of current month) and `untilDate` = today, both
   `dd.MM.yyyy` → save `ventopay/transactions-page.html`.
5. Same GET with `fromDate: '01.01.2099'`, `untilDate: '31.12.2099'` → save
   `ventopay/transactions-empty.html` (guaranteed-empty range).

Gourmet and Ventopay recording each run in a try/catch; sanitization runs on whatever was
recorded (v1: scripts/record-fixtures.ts:355-383).

### Sanitization rules (v1: scripts/record-fixtures.ts:290-349)

After recording, the script walks **every file** under the fixtures dir and applies plain
string `replaceAll` with this table (empty-string sources are filtered out to avoid
infinite loops):

| Recorded value | Replaced with |
|---|---|
| Gourmet username (`KANTINE_USERNAME`) | `TestUser` |
| `shopModelId` | `SM-TEST-123` |
| `eaterId` | `EATER-TEST-456` |
| `staffGroupId` | `SG-TEST-789` |
| Display name (`.loginname` text), only if ≠ username | `Test User` |
| Ventopay username (`AUTOMATEN_USERNAME`), only if ≠ Gourmet username | `TestUser` |

**Caveat**: nothing else is sanitized — live `ufprt`/`__ncforminfo`/`__VIEWSTATE`/
`__EVENTVALIDATION` values, transaction IDs, amounts, and menu texts pass through
unchanged. Raw recorder output is therefore not automatically publication-safe beyond the
listed identity fields; review before committing. (This is consistent with §2: recorder
output is an inspection aid, not the committed fixture set.)

### v2 requirement

Port the recorder as a dev-only tool (Rust binary or script) preserving: the exact
request sequences above (they follow the account-safety rules of 01-/02-), root `.env`
credential sourcing with the same four variable names, the same output file names, and
the same sanitization table (including the same sentinel replacement values, so recorded
pages can be diffed against the committed fixtures).

---

## 5. v1 suite inventory (27 test files)

All under v1: src/app/src-rn/__tests__/.

### api/ — parser, client, and orchestration tests (fixture-driven)

| Suite | Covers |
|---|---|
| `api/gourmetParser.test.ts` | All parser functions against gourmet fixtures: `extractLoginFormTokens`/`extractFormTokens`, `isLoggedIn`, `extractUserInfo`, `parseMenuItems`, `hasNextMenuPage`, `parseOrderedMenus`, `extractEditModeFormData`, `extractCancelOrderFormData`, `extractLogoutFormTokens`, plus edge cases (Unknown category, missing-token errors, cancel-form fallback matching). |
| `api/gourmetClient.test.ts` | HTTP-client shape with axios fully mocked: constructor config, `get`/`postForm`/`postJson` argument shapes, Origin/Referer header behavior, `resetClient()` semantics. |
| `api/gourmetApi.test.ts` | Orchestration with the client class mocked: login sequence (incl. stale-session logout and user-info re-fetch), menus pagination, session-expiry re-login/retry, orders parsing, `addToCart` JSON shape, `confirmOrders`/`cancelOrders` edit-mode state machine, `getBillings` request + response mapping, logout. |
| `api/ventopayParser.test.ts` | `extractAspNetState`, `isVentopayLoggedIn`, `parseTransactions` (Gourmet filter, German amounts, `Mrz`/`Jän` month parsing, empty page) + fallback edge cases. |
| `api/ventopayClient.test.ts` | Constructor config (`withCredentials: false`), URL-encoded `postForm`, Origin/Referer, manual cookie interceptors (capture array or single-string `Set-Cookie`; inject joined `Cookie` header), `getCookieDebug()` (names only, never values), `resetClient()`. |
| `api/ventopayApi.test.ts` | Login sequence with full ASP.NET field set, transaction fetching with `dd.MM.yyyy` params, session-expiry re-login/retry, logout via `GET Ausloggen.aspx`. |
| `api/demoGourmetApi.test.ts`, `api/demoVentopayApi.test.ts` | Demo-mode fakes (no fixtures): generated menus (10 weekdays × 4 categories = 40 items), orders workflow, billing, transactions. Behavior spec: `03-features/demo-mode.md`. |

### store/ — Zustand orchestration & caching (API layer mocked)

`store/authStore.test.ts`, `ventopayAuthStore.test.ts` (login/loginWithSaved/logout,
credential persistence, demo-mode switch), `menuStore.test.ts` (fetch/caching with
`MENU_CACHE_VALIDITY_MS`, pending orders/cancellations, ordering-cutoff guard, submit
flow, availability merge), `orderStore.test.ts` (fetch/confirm/cancel, caching, getters),
`billingStore.test.ts` (month options, dual-source fetch, filters, cached months),
`locationStore.test.ts`, `themeStore.test.ts`. Behavior specs: `03-features/*` and
`03-features/caching.md` — v2 must re-test the equivalent logic wherever it lands in the
Rust core.

### utils/ + theme/

`dateUtils.test.ts` (formatGourmetDate, parseGourmetDate, parseGourmetOrderDate,
localDateKey, isSameDay, isOrderingCutoff, findNearestDate, isCancellationCutoff — date
format contracts shared with 01-/02-), `menuFingerprint.test.ts`,
`menuChangeStorage.test.ts`, `dailyReminderCheck.test.ts`, `cancelReminderCheck.test.ts`,
`reminderStorage.test.ts`, `notificationService.test.ts`,
`notificationLogStorage.test.ts`, `platform.test.ts`, `theme/colors.test.ts`.

**Dropped in v2**: `utils/desktopUpdater.web.test.ts` and `utils/tauriHttp.web.test.ts`
test desktop/web-only modules (Velopack updater, Tauri HTTP proxy) that do not exist in
v2.

---

## 6. What the v2 Rust core test suite must replicate

The v2 core owns everything the v1 `api/` suites cover; store-level behavior moves to
wherever the equivalent logic lives (core vs. Swift/Kotlin shell) but the assertions
below concerning HTTP and parsing are **mandatory core tests**. Load the fixtures from
`docs/fixtures/` verbatim.

### 6.1 Request-shape assertions (HTTP client mocked/recorded at the boundary)

Full request specs are in 01-/02-; the tests must pin at minimum these exact shapes:

**Gourmet** (v1: __tests__/api/gourmetApi.test.ts, gourmetClient.test.ts):

- Client config: base URL `https://alaclickneu.gourmet.at`, redirects followed (v1 axios
  `maxRedirects: 5`), platform-native cookie handling (v1 mechanism: `withCredentials:
  true`; see 01-gourmet-scraping for the v2 cookie requirement).
- Every form POST carries header `Origin: https://alaclickneu.gourmet.at` and a `Referer`
  equal to the absolute URL of the last GET page; after a client reset with no prior GET,
  the Referer falls back to the request URL itself.
- Login: `GET https://alaclickneu.gourmet.at/start/`, then multipart POST to `/start/`
  with exactly `{ Username, Password, RememberMe: 'false', ufprt:
  'CSRF-TOKEN-LOGIN-ABC123', __ncforminfo: 'NCFORM-TOKEN-LOGIN-XYZ789' }` when fed
  `login-page.html`.
- Stale-session login: if the initial `GET /start/` already shows a logged-in page, first
  POST the logout form (`{ ufprt: 'CSRF-TOKEN-LOGOUT-DEF456', __ncforminfo:
  'NCFORM-TOKEN-LOGOUT-UVW012' }` from `login-success.html`), re-GET, then do the normal
  login POST.
- Failed login: a login POST whose response lacks the `/einstellungen/` anchor (fed
  `login-failed.html`) rejects with error `Login failed: invalid credentials or account
  blocked` (v1: __tests__/api/gourmetApi.test.ts:91-93).
- Login-response fallback: if the successful login response lacks
  `#shopModel`/`#eater`/`#staffGroup`, re-GET `/start/` and extract from there.
- Menus pagination: first `GET https://alaclickneu.gourmet.at/menus/` with **no** page
  param, then `{ page: '1' }`, `{ page: '2' }`… while `hasNextMenuPage` is true; item
  lists from all pages are concatenated.
- Session expiry: any data fetch that lands on an unauthenticated page triggers re-login
  with stored credentials and a single retry of the original request; with no stored
  credentials it fails with a session-expired error (v1 message contains
  `Session expired`).
- User-info recovery: `getMenus` on a valid session without prior login must recover
  user info from the menus page's `#shopModel`/`#eater`/`#staffGroup`; a logged-in page
  *without* those fields must not crash (menus still parse, user info stays null).
- `addToCart`: JSON POST to
  `https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart` with
  `Content-Type: application/json` and body exactly
  `{ eaterId, shopModelId, staffgroupId, dates: [{ date: 'MM-dd-yyyy', menuIds: [...] }] }`
  — entries grouped per date preserving order (e.g. two items on `02-10-2026` →
  `menuIds: ['menu-001','menu-004']`). Note lowercase `staffgroupId`. Response
  `{ success: false, message }` → error `Add to cart failed: {message}`; not logged in →
  error `Not logged in`.
- `confirmOrders`: GET orders page; if the `form-toggleEditMode` form has
  `editMode="False"` (currently IN edit mode) POST `/bestellungen/` with
  `{ editMode: 'False', ufprt, __ncforminfo }` (fixture values
  `CSRF-TOKEN-EDITMODE-EXIT-777`/`NCFORM-TOKEN-EDITMODE-EXIT-888`); if `editMode="True"`,
  POST nothing.
- `cancelOrders` state machine (exact POST order asserted): (1) if not in edit mode, POST
  the toggle `{ editMode: 'True', … }` and re-GET — if the page still shows
  `editMode="True"`, fail with `Failed to enter edit mode`; (2) POST the per-position
  cancel form data — must include `cp_PositionId` and that form's own `__ncforminfo`
  (fixture: `NCFORM-TOKEN-CANCEL-POS001-BBB`); (3) re-GET and POST the exit toggle
  `{ editMode: 'False', … }`.
- `getBillings`: after a session check (GET of the start page), JSON POST to
  `https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings` with exactly
  `{ eaterId, shopModelId, checkLastMonthNumber: '0' }` (string values). Response
  mapping: `Billings[]` wrapper → bills with `billNr`, `billDate` (parsed Date),
  `location`, `billing`, `items[]` (`id`, `articleId`, `count`, `description`, `total`,
  `subsidy`, `discountValue`, `isCustomMenu`); a raw top-level array (no `Billings` key)
  must also parse; a response with neither → `[]`; not logged in → `Not logged in`.
- Logout: GET a page, POST `/start/` with the logout form tokens, then reset the client
  (cookies/Referer state cleared) and report unauthenticated.

**Ventopay** (v1: __tests__/api/ventopayApi.test.ts, ventopayClient.test.ts):

- Client config: base `https://my.ventopay.com/mocca.website`, `withCredentials: false`
  (v1 mechanism), `maxRedirects: 5`, POSTs `Content-Type:
  application/x-www-form-urlencoded` with a `URLSearchParams`-encoded body, headers
  `Origin: https://my.ventopay.com` + Referer.
- Manual cookie management: capture `Set-Cookie` response headers (both array and single
  string forms), store `name=value` up to the first `;`, and inject
  `Cookie: name1=value1; name2=value2` on subsequent requests; no header when the jar is
  empty; a debug accessor exposes cookie **names only** (never values); reset clears all.
- Login: `GET https://my.ventopay.com/mocca.website/Login.aspx`, then POST the same URL
  with exactly `{ __LASTFOCUS: '', __EVENTTARGET: '', __EVENTARGUMENT: '', __VIEWSTATE:
  'VIEWSTATE-TOKEN-LONG-BASE64-STRING-ABC123', __VIEWSTATEGENERATOR: 'ABCD1234',
  __EVENTVALIDATION: 'EVENTVALIDATION-TOKEN-XYZ789', DropDownList1:
  '0da8d3ec-0178-47d5-9ccd-a996f04acb61', TxtUsername, TxtPassword, BtnLogin: 'Login',
  languageRadio: 'DE' }` when fed `login-page.html`. Failure (no `Ausloggen.aspx` link)
  → error `Ventopay login failed: invalid credentials or account blocked`.
- Transactions: `GET https://my.ventopay.com/mocca.website/Transaktionen.aspx` with
  params `{ fromDate: '01.02.2026', untilDate: '28.02.2026' }` (`dd.MM.yyyy`,
  zero-padded). Session expiry (response without `Ausloggen.aspx`) → re-login + one
  retry (3 GETs total in the test scenario).
- Logout: `GET https://my.ventopay.com/mocca.website/Ausloggen.aspx` + client reset.

### 6.2 Parser assertions (fixture-exact values)

From v1: __tests__/api/gourmetParser.test.ts and ventopayParser.test.ts — v2 parsers must
reproduce every one of these against the carried-over fixtures:

Gourmet:
- `extractLoginFormTokens(login-page.html)` → `ufprt='CSRF-TOKEN-LOGIN-ABC123'`,
  `ncforminfo='NCFORM-TOKEN-LOGIN-XYZ789'`; missing `ufprt` → error mentioning `ufprt`;
  present `ufprt` but missing `__ncforminfo` → error mentioning `__ncforminfo`.
- `isLoggedIn`: true only for `login-success.html`; false for `login-page.html` and
  `login-failed.html`.
- `extractUserInfo(login-success.html)` → `{ username: 'TestUser', shopModelId:
  'SM-TEST-123', eaterId: 'EATER-TEST-456', staffGroupId: 'SG-TEST-789' }`; empty page →
  error `Could not extract user info…`.
- `parseMenuItems(menus-page-0.html)` → exactly **7** items (mobile duplicate in
  `.row.hide-sm-up` ignored). Item 0: `id='menu-001'`, `title='MENÜ I'`,
  `subtitle='Wiener Schnitzel mit Kartoffelsalat'`, `allergens=['A','C','G']`
  (split from the single comma-separated `li.allergen`), category Menu1,
  `available=true`, `price='€ 5,50'`. Category detection: all four categories present;
  unmatched title (e.g. `WOCHENANGEBOT`) → Unknown. Checked checkbox → `ordered=true`
  ('Schweinsbraten mit Knödel'); no checkbox → `available=false` ('Gebratener Lachs mit
  Dillsauce'); empty `li.allergen` → `allergens=[]` ('Reis mit Gemüse').
- `hasNextMenuPage`: true for page-0, false for page-1.
- `parseOrderedMenus(orders-page.html)` → 3 orders; POS-001 `eatingCycleId='EC-001'`,
  `approved=true` (fa-check); POS-002 `approved=false`; POS-003 `approved=true`
  (`checkmark` class).
- `extractEditModeFormData`: `editMode='True'` from `orders-page.html`,
  `editMode='False'` from `orders-page-edit-mode.html`, with the fixture ufprt/ncforminfo
  values; token-less form → error `Could not extract edit mode form data`.
- `extractCancelOrderFormData(orders-page-edit-mode.html, 'POS-001')` →
  `{ positionId: 'POS-001', eatingCycleId: 'EC-001', date: '10.02.2026 00:00:00'(from
  cp_Date), ufprt: 'CSRF-TOKEN-CANCEL-POS001-AAA', ncforminfo:
  'NCFORM-TOKEN-CANCEL-POS001-BBB' }`; must also find the form by its
  `cp_PositionId` input when the `form_{pos}_cp` id is absent; missing tokens → error
  `Could not extract cancel form data for position: {pos}`.
- `extractLogoutFormTokens(login-success.html)` → the LOGOUT token pair; page without a
  logout form → `Could not find logout form…`; form present but token-less →
  `Could not extract logout form tokens`.

Ventopay:
- `extractAspNetState(login-page.html)` → the three token sentinels above plus
  `lastFocus`/`eventTarget`/`eventArgument` = `''`; missing state → error matching
  `Could not extract ASP.NET state`.
- `isVentopayLoggedIn`: true for `login-success.html`, false for `login-page.html`.
- `parseTransactions(transactions-page.html)` → **5** of 6 (Gourmet entry filtered).
  First: `id='dHhuLTAwMQ=='`, `amount=1.8`, `restaurant='Café + Co. Automaten'`,
  date = 2026-02-09 11:49 local. Amounts `[1.8, 3.2, 0.5, 2.4, 1.5]`. Month
  abbreviations: `Mrz` → March, `Jän` → January. Empty page → `[]`. Fallbacks: an
  ISO-format timestamp string still parses; a title without `(…)` yields the raw title
  as both amount source and `restaurant`; entries missing `id` or title are skipped.

### 6.3 Orchestration / feature-level tests

The store suites (§5) encode caching, pending-change, cutoff, and dual-source billing
behavior whose specs live in `03-features/*`. v2 must provide equivalent tests at the
layer that owns each behavior; the harness pattern to preserve is: fake the API layer,
fake storage (in-memory), fake the clock where cutoffs are involved (v1 uses dates
"+14 days" to dodge cutoffs — v1: __tests__/store/menuStore.test.ts:57-64), and assert
state transitions (loading flags, error strings, cache reuse within
`MENU_CACHE_VALIDITY_MS`). Demo-mode fakes get their own deterministic tests
(40 generated menu items = 10 weekdays × 4 categories; no weekend days — v1:
__tests__/api/demoGourmetApi.test.ts:41-60; spec in `03-features/demo-mode.md`).

### 6.4 Explicitly not replicated

- `desktopUpdater.web.test.ts`, `tauriHttp.web.test.ts` — desktop/web dropped in v2.
- The RN-runtime mocks in setup.ts (AsyncStorage/expo-secure-store/Platform) — replaced
  by injected traits in the Rust core; native-shell storage is covered by
  `05-platform-services.md`.

---

## Open questions

- None blocking. (The Rechnung.aspx discrepancy between CLAUDE.md and code, and the
  outdated "178 tests / 13 test files" CLAUDE.md claim, are resolved above in favor of
  the code.)
