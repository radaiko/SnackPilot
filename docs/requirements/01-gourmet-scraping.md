# 01 — Gourmet (Kantine) scraping specification

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

**SAFETY-CRITICAL.** The Gourmet backend detects "incomplete" or malformed form
submissions as bot behavior and **bans accounts**. Every deviation listed in
[§13 Things that ban accounts](#13-things-that-ban-accounts) has been observed (or strongly
suspected) to cause bans. Implement this spec byte-for-byte; do not "improve" the request
sequence.

This doc owns `src/app/src-rn/api/gourmetClient.ts`, `gourmetParser.ts`, `gourmetApi.ts`,
plus the Gourmet parts of `api/types.ts`, `utils/constants.ts`, `utils/dateUtils.ts`, and
`analysis/playwright-findings.md` (see `appendix-source-map.md`). Feature-level behavior
(stores, caching, UI, ordering cutoff policy, demo mode) lives in `03-features/*`; credential
storage lives in `05-platform-services`. Where those docs need HTTP details, they reference
this doc — this doc is the single source of truth for every byte sent to
`alaclickneu.gourmet.at`.

---

## 1. Endpoints and constants

All URLs (v1: src/app/src-rn/utils/constants.ts:1-7):

| Constant | Exact value |
|---|---|
| `GOURMET_BASE_URL` | `https://alaclickneu.gourmet.at` |
| `GOURMET_LOGIN_URL` | `https://alaclickneu.gourmet.at/start/` |
| `GOURMET_MENUS_URL` | `https://alaclickneu.gourmet.at/menus` |
| `GOURMET_ORDERS_URL` | `https://alaclickneu.gourmet.at/bestellungen` |
| `GOURMET_ADD_TO_CART_URL` | `https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart` |
| `GOURMET_BILLING_URL` | `https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings` |
| `GOURMET_SETTINGS_URL` | `https://alaclickneu.gourmet.at/einstellungen/` (defined but unused in v1 code; never requested — login detection uses the literal substring `/einstellungen/` in `isLoggedIn`, see §4, v1: src/app/src-rn/api/gourmetParser.ts:45) |

Related constants owned by other docs but defined alongside (v1: src/app/src-rn/utils/constants.ts:9-42):
`MENU_CACHE_VALIDITY_MS = 4 * 60 * 60 * 1000` (→ 03-features/caching), credential storage keys
`gourmet_username` / `gourmet_password` (→ 05-platform-services), demo credentials
`demo` / `demo1234!` (→ 03-features/demo-mode), 09:00 ordering/cancellation cutoff
(→ 03-features/menus, 03-features/orders; helpers in §12).

Origin header value (v1: src/app/src-rn/api/gourmetClient.ts:4):

```
GOURMET_ORIGIN = 'https://alaclickneu.gourmet.at'
```

---

## 2. HTTP client requirements

(v1: src/app/src-rn/api/gourmetClient.ts:19-77)

The client is a single long-lived HTTP session against `GOURMET_BASE_URL` with:

- **Cookie handling delegated to the platform's native HTTP stack.**
  *v1 mechanism:* axios with `withCredentials: true` so iOS's NSURLSession (Android: OkHttp)
  stores and replays cookies automatically; the server's `Set-Cookie` headers are consumed
  by the native layer and never exposed to application code
  (v1: src/app/src-rn/api/gourmetClient.ts:9-11,26). v2 must equivalently use a native
  cookie-jar-backed session (e.g. `URLSession` with default cookie storage / OkHttp with a
  CookieJar) so cookies set during redirects are captured. Cookies persist for the process
  lifetime and — on iOS/Android native stores — across app restarts (see §6 stale-session
  handling).
- **Redirects followed automatically**, up to 5 (`maxRedirects: 5`,
  v1: src/app/src-rn/api/gourmetClient.ts:27). The login POST answers with a 302 to
  `/menus/` and cookies are set during the redirect (v1: analysis/playwright-findings.md:36-38);
  the client only ever sees the final page.
- **Status validation**: statuses `200-399` are success; `>= 400` raises an error
  (`validateStatus: (status) => status >= 200 && status < 400`,
  v1: src/app/src-rn/api/gourmetClient.ts:28).
- **CRITICAL: no custom `User-Agent` header** — the platform default is used
  (v1: src/app/src-rn/api/gourmetClient.ts:16).
- **CRITICAL: every request (GET, form POST, JSON POST) carries
  `Accept: application/json, text/plain, */*`.** This is NOT a platform default — it is an
  app-level default v1 inherits from axios (`headers.common.Accept`,
  v1: src/app/node_modules/axios/lib/defaults/index.js:167; gourmetClient never overrides
  it). A v2 client on URLSession/OkHttp/reqwest would otherwise send a different Accept
  value (e.g. `*/*`) on every request. Header ownership summary — app-set: `Accept` (all
  requests), `Content-Type`/`Origin`/`Referer` (POSTs only, §2.2/§2.3); left to the
  platform stack: `User-Agent`, `Accept-Encoding`, `Accept-Language`, `Connection`.
- **CRITICAL: no request throttling or artificial delays** — delays may cause session
  timeout (v1: src/app/src-rn/api/gourmetClient.ts:17; CLAUDE.md "Things That Will Break
  Accounts" #8).

Three request primitives:

### 2.1 `get(url, params?)` → HTML string

Plain GET, `responseType: 'text'`, optional query params
(v1: src/app/src-rn/api/gourmetClient.ts:33-42). No Origin/Referer headers are set
explicitly on GETs. After every GET the client records `lastPageUrl` = the absolute request
URL **without query params** (if the url was relative, it is prefixed with
`GOURMET_BASE_URL` plus a `/` if needed; v1: src/app/src-rn/api/gourmetClient.ts:38-40).

### 2.2 `postForm(url, data)` → HTML string

**CRITICAL: every Gourmet form POST must be `multipart/form-data`** (every form on the site
declares `enctype="multipart/form-data"`; sending `application/x-www-form-urlencoded`
makes the server silently reject the login — HTTP 200 with the login page instead of the
302 redirect) (v1: src/app/src-rn/api/gourmetClient.ts:13-14,44-59;
analysis/playwright-findings.md:12-14).

*v1 mechanism:* `axios.postForm()` builds the multipart body and boundary automatically
from a flat `Record<string, string>`; parts are emitted in object-key insertion order.

Headers set on every form POST (v1: src/app/src-rn/api/gourmetClient.ts:51-54):

```
Origin: https://alaclickneu.gourmet.at
Referer: {lastPageUrl, or the request url itself if no GET happened yet}
Content-Type: multipart/form-data; boundary={auto-generated}
```

The Referer is therefore always the URL of the page the form was scraped from (e.g. after
`GET https://alaclickneu.gourmet.at/start/`, a login POST carries
`Referer: https://alaclickneu.gourmet.at/start/`)
(v1: src/app/src-rn/__tests__/api/gourmetClient.test.ts:100-114).

Fallback-Referer caveat: when no GET has happened yet, the fallback value is the URL string
*exactly as passed* to the POST helper — in v1 that is a relative path such as
`/bestellungen/`, not an absolute URL (pinned by
src/app/src-rn/__tests__/api/gourmetClient.test.ts:157-178, which asserts
`Referer: /bestellungen/` after `resetClient()`). This branch is unreachable in every
documented flow (each POST follows a GET that set `lastPageUrl`), so v2 only needs it for
behavioral completeness.

### 2.3 `postJson(url, body)` → parsed JSON

Headers (v1: src/app/src-rn/api/gourmetClient.ts:62-71):

```
Content-Type: application/json
Origin: https://alaclickneu.gourmet.at
Referer: {lastPageUrl, or the request url itself if no GET happened yet}
```

### 2.4 `resetClient()`

Clears only `lastPageUrl` (v1: src/app/src-rn/api/gourmetClient.ts:74-76). Despite the
code comment, it does **not** clear native cookies; session termination relies on the
server-side logout POST (§11), and any cookies that survive are handled by the
stale-session logic at the next login (§6.1).

---

## 3. Form-token rules (apply to EVERY form POST)

Every form on the Gourmet site contains two hidden CSRF/anti-bot fields, and **both must be
extracted fresh from the page and sent with every form POST**
(v1: src/app/src-rn/api/gourmetParser.ts:10-29; analysis/playwright-findings.md:6-10):

- `ufprt` — CSRF token, changes per page load
- `__ncforminfo` — anti-bot token, changes per page load. **The original .NET MAUI app was
  banned precisely because it did not send this field**
  (v1: analysis/playwright-findings.md:6-10).

Token extraction (v1: src/app/src-rn/api/gourmetParser.ts:14-29): within the target form,
read `input[name="ufprt"]` `value` and `input[name="__ncforminfo"]` `value`. If either is
missing, **abort with an error — never POST without both**
(errors: `Could not find ufprt in form: {selector}` /
`Could not find __ncforminfo in form: {selector}`).

Tokens are single-use per page load: after any state-changing POST, re-fetch the page
before extracting tokens for the next POST (see §9.4 cancel loop).

---

## 4. Login-state detection (`isLoggedIn`)

A page is considered authenticated if the raw HTML contains **any** of these four
substrings (plain substring checks, not selectors)
(v1: src/app/src-rn/api/gourmetParser.ts:43-50):

1. `/einstellungen/` — settings link (absolute or relative)
2. `btnHeaderLogout` — logout button id
3. `class="loginname"` — username display
4. `id="eater"` — eater hidden input (present on menus/orders pages)

> **Discrepancy vs CLAUDE.md (code wins):** CLAUDE.md specifies verifying login with the
> regex `<a href="https://alaclickneu.gourmet.at/einstellungen/" class="navbar-link">`.
> The code uses the four substring indicators above because different pages use different
> layouts (v1: src/app/src-rn/api/gourmetParser.ts:40-42). Implement the code's behavior.

---

## 5. User-info extraction

(v1: src/app/src-rn/api/gourmetParser.ts:55-68)

From any authenticated page containing the hidden inputs (menus, orders, start-after-login):

| Field | Extraction (CSS selector on parsed HTML) |
|---|---|
| `shopModelId` | `#shopModel` — `value` attribute |
| `eaterId` | `#eater` — `value` attribute |
| `staffGroupId` | `#staffGroup` — `value` attribute |
| `username` | `span.loginname` — text content, trimmed |

If any of `shopModelId`, `eaterId`, `staffGroupId` is missing → error
`Could not extract user info from page`. A missing/empty `username` is tolerated.

Resulting record (`GourmetUserInfo`, v1: src/app/src-rn/types/menu.ts:26-31):
`{ username, shopModelId, eaterId, staffGroupId }` — all strings. These IDs are required
by both JSON APIs (§10).

---

## 6. Authentication flow

(v1: src/app/src-rn/api/gourmetApi.ts:62-109) — **DO NOT MODIFY THIS SEQUENCE.**

### 6.1 Step 0 — stale-session pre-logout

Native cookie stores persist across app restarts, so `GET /start/` may return an
authenticated page instead of the login form (v1: src/app/src-rn/api/gourmetApi.ts:66-80):

1. `GET https://alaclickneu.gourmet.at/start/`
2. If the response `isLoggedIn()` (§4):
   a. Extract logout-form tokens (§11) from that page.
   b. `POST /start/` (multipart, §2.2) with exactly two fields:
      `ufprt={token}`, `__ncforminfo={token}`.
   c. Any failure in a-b is swallowed (session may have expired mid-check).
   d. Re-`GET https://alaclickneu.gourmet.at/start/` — this response replaces the login
      page HTML for the next step.

### 6.2 Steps 1-4 — login

1. From the login page HTML, extract tokens from the **first form on the page**
   (selector `form:first-of-type`; the page has two forms, login and forgot-password,
   both posting to `/start/`) (v1: src/app/src-rn/api/gourmetParser.ts:35-37;
   analysis/playwright-findings.md:20-23).
2. `POST /start/` as `multipart/form-data` with **exactly** these five fields, in this
   order (v1: src/app/src-rn/api/gourmetApi.ts:85-91):

   ```
   Username:     {username}
   Password:     {password}
   RememberMe:   false          ← literal string "false", never boolean/true
   ufprt:        {extracted}
   __ncforminfo: {extracted}
   ```

   On success the server responds 302 → `/menus/` (followed transparently; cookies are set
   during the redirect) (v1: analysis/playwright-findings.md:36-38).
3. Verify: run `isLoggedIn()` (§4) on the final response HTML. If false → error
   `Login failed: invalid credentials or account blocked`
   (v1: src/app/src-rn/api/gourmetApi.ts:93-96).
4. Extract user info (§5) from the response HTML. If extraction throws (the redirect
   target may lack the hidden inputs), `GET https://alaclickneu.gourmet.at/start/` once
   more and extract from that page instead (v1: src/app/src-rn/api/gourmetApi.ts:98-106).

After success, the credentials are retained in memory for automatic re-login (§7)
(v1: src/app/src-rn/api/gourmetApi.ts:107).

On login **failure** the error propagates and any previously cached user info/credentials
are left **unchanged** — the throw at step 3 happens before the assignments, and there is
no catch/reset path in `login()`; only `logout()` (§11) clears them
(v1: src/app/src-rn/api/gourmetApi.ts:94-107). A v2 implementation must not clear cached
credentials on a failed login attempt (§7's automatic re-login deliberately retries with
whatever credentials are cached).

---

## 7. Session-expiry handling (`ensureSession`)

(v1: src/app/src-rn/api/gourmetApi.ts:42-51)

Before parsing any scraped HTML page, check `isLoggedIn(html)`:

- Logged in → proceed with that HTML.
- Not logged in, credentials cached → run the full login flow (§6); the **caller** then
  re-fetches the original page and proceeds with the fresh HTML — but ONLY at the
  HTML-parsing call sites (see below).
- Not logged in, no credentials → raise `SessionExpiredError` (name
  `SessionExpiredError`, message `Session expired`; v1: src/app/src-rn/api/types.ts:68-73).

Applied on: menus page 0 (§8.1), orders page in `getOrders`/`confirmOrders`/`cancelOrders`
(§9), and a probe `GET /start/` before `getBillings` (§10.2). **Not** applied on menus
pages ≥ 1 or before `addToCart` (§10.1) — those trust the session established earlier.

**Re-fetch exception (exact request sequence matters):** the re-fetch after a re-login is
performed by `getMenus`, `getOrders`, `confirmOrders`, and `cancelOrders` (they check
`ensureSession`'s empty-string return and re-GET the page they need to parse;
v1: src/app/src-rn/api/gourmetApi.ts:131-135, 163-166, 216-219, 243-246). `getBillings`
**ignores** the re-fetch signal: after a re-login triggered by its probe, it proceeds
directly to the `GetMyBillings` POST with NO second `GET /start/`
(v1: src/app/src-rn/api/gourmetApi.ts:313-315 — return value discarded). A v2
implementation must not insert an extra GET here.

---

## 8. Menus

### 8.1 Pagination

(v1: src/app/src-rn/api/gourmetApi.ts:26,121-155)

- Loop `page = 0 .. 9` (`MAX_MENU_PAGES = 10`, hard cap even if more pages are advertised).
- Every page: `GET https://alaclickneu.gourmet.at/menus/` (note trailing slash).
  - Page 0: **no query parameters at all**.
  - Page N ≥ 1: query param `page=N` (i.e. `/menus/?page=1`, `/menus/?page=2`, …).
- On page 0 only: run `ensureSession` (§7; re-fetch page 0 after a re-login), and if user
  info is not yet cached, try to extract it (§5) from the page, ignoring failures
  (v1: src/app/src-rn/api/gourmetApi.ts:131-144).
- Parse the page's menu items (§8.2), append to the result.
- **Stop after any page that has no next-page link.** Next-page detection: any `<a>`
  element whose `class` attribute contains the substring `menues-next` — CSS selector
  `a[class*="menues-next"]` (v1: src/app/src-rn/api/gourmetParser.ts:149-152).

> **Discrepancy vs CLAUDE.md (code wins):** CLAUDE.md writes the endpoint as
> `/menus?page={0-9}` and the selector as `a.menues-next`. The code GETs `/menus/` (with
> trailing slash), omits the `page` param entirely on page 0, and matches the class by
> substring.

### 8.2 Menu-item parsing

(v1: src/app/src-rn/api/gourmetParser.ts:94-144)

The menus page renders every meal twice — a desktop layout under `div.row.hide-sm-down`
and a mobile layout under `div.row.hide-sm-up` (v1: analysis/playwright-findings.md:59-67).
**Parse only the desktop layout** to avoid duplicates; item selector:

```
div.row.hide-sm-down .meal
```

> **Discrepancy vs CLAUDE.md (code wins):** CLAUDE.md gives the selector as `div.meal`
> (global). That would double-parse every meal.

Per `.meal` element:

| Field | Extraction |
|---|---|
| `id` | `.open_info.menu-article-detail` → `data-id` attribute. **Skip the item entirely if missing.** |
| `day` | `.open_info.menu-article-detail` → `data-date` attribute, format `MM-dd-yyyy` (§12). **Skip the item entirely if missing.** |
| `title` | `.title` → **direct text nodes only** (the category label, e.g. `MENÜ I`; the subtitle is a nested `div` and must be excluded), trimmed |
| `subtitle` | `.subtitle` → text, trimmed |
| `allergens` | `li.allergen` → text, trimmed; a **single element** containing comma-separated letters (e.g. `A, C, G`). Split on `,`, trim each, drop empties. Empty text → `[]`. |
| `available` | `true` iff `input[type="checkbox"].menu-clicked` exists in the meal |
| `ordered` | `true` iff that checkbox exists **and** is checked (has a `checked` attribute or checked state) |
| `price` | `.price span` → text, trimmed; `''` if absent. **Treat as opaque text — the format is not guaranteed**: the sanitized fixture shows `€ 5,50` (fixtures/gourmet/menus-page-0.html:25) while the live Playwright analysis recorded `6,00 EUR` (analysis/playwright-findings.md:100). |
| `category` | derived from `title`, §8.3 |

Menu IDs are per-category, not per-item: all `MENÜ I` items across days share one
`data-id` (v1: analysis/playwright-findings.md:96; CLAUDE.md "Key").

Result shape (`GourmetMenuItem`, v1: src/app/src-rn/types/menu.ts:1-11):
`{ id, day, title, subtitle, allergens[], available, ordered, category, price }`.

### 8.3 Category detection

(v1: src/app/src-rn/api/gourmetParser.ts:7-8,73-89)

Applied to the extracted `title`, in this order:

1. If the title **contains** the literal string `SUPPE & SALAT` (case-sensitive) →
   `SUPPE & SALAT` category.
2. Else match the regex (copy exactly, note the `i` flag and the `U`-without-umlaut
   alternative):

   ```
   /MEN(?:Ü|U)\s+([I]{1,3})/i
   ```

   Capture-group length 1/2/3 → `MENÜ I` / `MENÜ II` / `MENÜ III`.
3. Else → `UNKNOWN` (e.g. title `WOCHENANGEBOT`;
   v1: src/app/src-rn/__tests__/api/gourmetParser.test.ts:305-318).

Category enum values (v1: src/app/src-rn/types/menu.ts:13-19): `MENÜ I`, `MENÜ II`,
`MENÜ III`, `SUPPE & SALAT`, `UNKNOWN`.

> **Discrepancy vs CLAUDE.md (code wins):** CLAUDE.md gives `MENÜ\s+([I]{1,3})` — the code
> regex is case-insensitive and also accepts `MENU` without the umlaut.

---

## 9. Orders page (`/bestellungen/`)

### 9.1 Fetching orders

(v1: src/app/src-rn/api/gourmetApi.ts:160-169)

`GET https://alaclickneu.gourmet.at/bestellungen/` (trailing slash), `ensureSession` (§7,
re-fetch after re-login), then parse.

Order-item parsing (v1: src/app/src-rn/api/gourmetParser.ts:157-197), selector
`div.order-item, div[class*="order-item"]`; per item:

| Field | Extraction |
|---|---|
| `positionId` | `input[name="cp_PositionId"]` → `value`. **Skip the item if missing.** |
| `eatingCycleId` | `input[name^="cp_EatingCycleId_"]` (prefix match) → `value`, `''` if absent |
| `date` | `input[name^="cp_Date_"]` (prefix match) → `value`, format `dd.MM.yyyy HH:mm:ss` (§12); if the input is absent, v1 falls back to "now" |
| `title` | `.title` → direct text nodes only, trimmed (same rule as §8.2) |
| `subtitle` | `.subtitle` → text, trimmed |
| `approved` | `true` iff the item contains an element matching `.fa-check` **or** `.checkmark` |

> **Discrepancy vs CLAUDE.md (code wins):** CLAUDE.md says approved = presence of
> `.confirmed` class or `fa fa-check` icon. The parser comment states no `.confirmed`
> class exists on the site (v1: src/app/src-rn/api/gourmetParser.ts:181); the real
> indicators are `.fa-check` or `.checkmark`.

Result shape (`GourmetOrderedMenu`, v1: src/app/src-rn/types/order.ts:1-8):
`{ positionId, eatingCycleId, date, title, subtitle, approved }`.

### 9.2 Edit-mode toggle form — inverted-state machine

The orders page carries a toggle form, selector `form.form-toggleEditMode` (class, not id)
(v1: src/app/src-rn/api/gourmetParser.ts:202-215; analysis/playwright-findings.md:132-133,153).
Extract from it: `input[name="editMode"]` `value` (default `'True'` if the input is
missing), plus `ufprt` and `__ncforminfo` (both required, else error
`Could not extract edit mode form data`).

**Semantics are inverted** (v1: src/app/src-rn/api/gourmetApi.ts:221,249):

| `editMode` value in the form | Page state | Posting the form does |
|---|---|---|
| `"True"` | NOT in edit mode (orders confirmed view) | enters edit mode |
| `"False"` | IN edit mode (cancel forms visible) | exits edit mode (confirms) |

Toggling = `POST /bestellungen/` (multipart) with exactly three fields — the `editMode`
value **as extracted from the page** plus the two tokens:

```
editMode:     {extracted value, "True" or "False"}
ufprt:        {extracted}
__ncforminfo: {extracted}
```

> **Discrepancy vs CLAUDE.md (code wins):** CLAUDE.md says the toggle posts
> `editMode=True`. That is only the enter-edit-mode case; exiting posts `editMode=False`.
> The rule is: echo the value found in the form.

### 9.3 Confirm flow (`confirmOrders`)

After `AddToMenuesCart` (§10.1) new orders sit unconfirmed and the page is in edit mode;
confirming = exiting edit mode (v1: src/app/src-rn/api/gourmetApi.ts:214-233):

1. `GET /bestellungen/` + `ensureSession` (re-fetch after re-login).
2. Extract edit-mode form data (§9.2).
3. If `editMode == "False"` (in edit mode): POST the toggle (fields per §9.2).
4. If `editMode == "True"`: already confirmed — do nothing.

### 9.4 Cancel flow (`cancelOrders(positionIds)`)

(v1: src/app/src-rn/api/gourmetApi.ts:240-301) — cancel forms only exist while in edit mode.

1. `GET /bestellungen/` + `ensureSession` (re-fetch after re-login).
2. If not already in edit mode (`editMode != "False"`):
   a. POST the toggle (§9.2, echoing `editMode="True"`).
   b. Re-`GET /bestellungen/` (POST redirect responses may not reflect the new state).
   c. Verify the fresh page has `editMode == "False"`; otherwise error
      `Failed to enter edit mode`.
3. For each `positionId`, in order:
   a. Locate that order's cancel form: selector `form#form_{positionId}_cp`; if not found,
      fall back to the form containing
      `input[name="cp_PositionId"][value="{positionId}"]`
      (v1: src/app/src-rn/api/gourmetParser.ts:221-250).
   b. From that form extract the eating-cycle and date inputs — located by **name prefix**
      within the form (`input[name^="cp_EatingCycleId_"]`, `input[name^="cp_Date_"]`), not
      by exact name — plus `ufprt` and `__ncforminfo`
      (v1: src/app/src-rn/api/gourmetParser.ts:233-249). If either prefix-matched input is
      absent, its value falls back to the **empty string** and the POST still proceeds with
      all five field names present (never omit a field, never abort for these two). Only
      missing tokens abort, with error
      `Could not extract cancel form data for position: {positionId}`. The date value is
      echoed verbatim (format `dd.MM.yyyy HH:mm:ss`).
   c. `POST /bestellungen/` (multipart) with exactly these five fields (note the two
      dynamic field **names** embed the positionId):

      ```
      cp_PositionId:                  {positionId}
      cp_EatingCycleId_{positionId}:  {extracted value}
      cp_Date_{positionId}:           {extracted value}
      ufprt:                          {extracted}
      __ncforminfo:                   {extracted}
      ```

   d. Re-`GET /bestellungen/` to obtain **fresh tokens** before the next cancellation.
4. After the loop, if the last-fetched page still shows `editMode == "False"`, POST the
   toggle once more to exit edit mode (echoing `editMode="False"`).

Expected POST sequence for a single cancel starting from the confirmed view: toggle-enter →
cancel → toggle-exit (3 form POSTs, with a re-GET of the orders page between each)
(v1: src/app/src-rn/__tests__/api/gourmetApi.test.ts:418-444).

---

## 10. JSON APIs

Both are `POST` with `Content-Type: application/json` plus Origin/Referer per §2.3, and
require the user-info IDs from §5.

### 10.1 AddToMenuesCart (place orders)

(v1: src/app/src-rn/api/gourmetApi.ts:176-208; request type v1: src/app/src-rn/api/types.ts:1-12)

`POST https://alaclickneu.gourmet.at/umbraco/api/AlaCartApi/AddToMenuesCart`

Requires cached user info (error `Not logged in` otherwise). No session probe is performed
first. Input items `{date, menuId}` are grouped by day (first-seen date order preserved,
menuIds in input order); each date formatted `MM-dd-yyyy` (§12):

```json
{
  "eaterId": "{userInfo.eaterId}",
  "shopModelId": "{userInfo.shopModelId}",
  "staffgroupId": "{userInfo.staffGroupId}",
  "dates": [
    { "date": "02-10-2026", "menuIds": ["menu-001", "menu-004"] },
    { "date": "02-11-2026", "menuIds": ["menu-001"] }
  ]
}
```

**Note the key casing: `staffgroupId` with lowercase `g`** (the user-info field is
`staffGroupId`, but the API key is `staffgroupId`; v1: src/app/src-rn/api/types.ts:5).

Response: JSON `{ success: boolean, message?: string }`. If `success` is not `true`, raise
`Add to cart failed: {message | 'unknown error'}`
(v1: src/app/src-rn/api/gourmetApi.ts:200-207).

After a successful call the new orders are **unconfirmed**; run the confirm flow (§9.3)
(v1: src/app/src-rn/api/gourmetApi.ts:211-213).

### 10.2 GetMyBillings

(v1: src/app/src-rn/api/gourmetApi.ts:308-351; types v1: src/app/src-rn/api/types.ts:14-45)

0. Require cached user info **before any request is sent**: with no cached user info, throw
   `Not logged in` — no probe GET is emitted at all
   (v1: src/app/src-rn/api/gourmetApi.ts:309-311; pinned by
   src/app/src-rn/__tests__/api/gourmetApi.test.ts:503-506).
1. Session probe: `GET https://alaclickneu.gourmet.at/start/`, then `ensureSession` (§7).
   **If the probe triggers a re-login, do NOT re-fetch `/start/`** — proceed directly to
   step 2 (the re-fetch signal is ignored here; see §7 re-fetch exception,
   v1: src/app/src-rn/api/gourmetApi.ts:313-315).
2. `POST https://alaclickneu.gourmet.at/umbraco/api/AlaMyBillingApi/GetMyBillings`:

```json
{
  "eaterId": "{userInfo.eaterId}",
  "shopModelId": "{userInfo.shopModelId}",
  "checkLastMonthNumber": "0"
}
```

`checkLastMonthNumber` is a **string**: `"0"` = current month, `"1"` = last month,
`"2"` = two months ago (v1: src/app/src-rn/api/gourmetApi.ts:306).

Response: normally a wrapper object `{"Billings": [ ...bills ]}`, but a raw top-level array
must also be accepted; a response with neither yields `[]`
(v1: src/app/src-rn/api/gourmetApi.ts:329-331).

Bill shape (PascalCase, from server; v1: src/app/src-rn/api/types.ts:22-45 and fixture
src/app/src-rn/__tests__/fixtures/gourmet/billing-current.json):

```json
{
  "BillNr": 10001,
  "BillDate": "2026-02-10T12:00:00",
  "Location": "Betriebsrestaurant Wien",
  "BillingItemInfo": [
    {
      "Id": "ITEM-001", "ArticleId": "ART-001", "Count": 1,
      "Description": "Menü I - Wiener Schnitzel",
      "Total": 5.50, "Subsidy": 2.50, "DiscountValue": 0.00, "IsCustomMenu": false
    }
  ],
  "Billing": 4.50
}
```

Mapping to the app model (v1: src/app/src-rn/api/gourmetApi.ts:333-350; target types
v1: src/app/src-rn/types/billing.ts): `BillNr→billNr` (number), `BillDate→billDate`
(parsed as an ISO-ish local datetime string without timezone suffix), `Location→location`,
`Billing→billing` (number, total after subsidy/discount), and per item
`Id→id, ArticleId→articleId, Count→count, Description→description, Total→total,
Subsidy→subsidy, DiscountValue→discountValue, IsCustomMenu→isCustomMenu`.

---

## 11. Logout

(v1: src/app/src-rn/api/gourmetApi.ts:356-373; parser v1: src/app/src-rn/api/gourmetParser.ts:255-272)

1. `GET https://alaclickneu.gourmet.at/start/` (any authenticated page works — the logout
   form is in the site header).
2. Locate the logout form: the form containing the logout button — selector
   `form:has(button#btnHeaderLogout), form:has(button:contains("Logout"))`. Missing form →
   error `Could not find logout form`; missing tokens → `Could not extract logout form tokens`.
3. `POST /start/` (multipart) with exactly two fields: `ufprt`, `__ncforminfo`.
4. Any error in 1-3 is swallowed (logout is non-critical). Always clear the in-memory user
   info + credentials and reset the client (§2.4). Native cookies are NOT cleared — the
   next login's stale-session pre-logout (§6.1) covers leftovers.

---

## 12. Date formats

(v1: src/app/src-rn/utils/dateUtils.ts:1-27)

| Context | Format | Direction |
|---|---|---|
| Menu `data-date` attribute; `dates[].date` in AddToMenuesCart | `MM-dd-yyyy` (zero-padded, e.g. `02-10-2026`) | parse + format |
| Orders `cp_Date_{positionId}` value | `dd.MM.yyyy HH:mm:ss` (e.g. `10.02.2026 00:00:00`); when parsing, a missing time component defaults to `00:00:00`. When cancelling, the value is echoed back verbatim, never re-formatted. | parse (echo on POST) |
| `BillDate` in GetMyBillings response | ISO-like local datetime string, e.g. `2026-02-10T12:00:00` | parse |

All parsing produces **local** dates (no UTC conversion). Business-rule cutoffs (ordering/
cancellation blocked from 09:00 Europe/Vienna on the same day;
v1: src/app/src-rn/utils/dateUtils.ts:88-112, constants.ts:40-42) are client-side policy
owned by 03-features/menus and 03-features/orders — they do not change any request bytes.

---

## 13. Things that ban accounts

Verbatim operational rules (CLAUDE.md "Things That Will Break Accounts", cross-checked
against code):

1. **Missing `__ncforminfo`** — every Gourmet form POST needs both `ufprt` AND
   `__ncforminfo` (§3). This is the confirmed cause of the predecessor app's bans
   (v1: analysis/playwright-findings.md:6-10).
2. **Wrong Content-Type** — form POSTs must be `multipart/form-data`, NOT
   `application/x-www-form-urlencoded` (§2.2).
3. **Missing/stale CSRF tokens** — extract a fresh `ufprt` (+ `__ncforminfo`) from the
   page for every request; never reuse tokens across POSTs (§3, §9.4d).
4. **Wrong date formats** — Gourmet uses `MM-dd-yyyy` (§12).
5. **Missing form parameters** — all hidden inputs of the submitted form must be included
   (login: 5 fields; toggle: 3 fields; cancel: 5 fields; logout: 2 fields).
6. **Wrong parameter values** — `RememberMe` must be the literal string `false`
   (v1: src/app/src-rn/api/gourmetApi.ts:88).
7. **Changing request order** — login must complete before any data request; follow the
   sequences in §6, §9.3, §9.4 exactly.
8. **Rate limiting/delays** — intentionally NO throttling anywhere; added delays can cause
   session timeouts mid-flow (v1: src/app/src-rn/api/gourmetClient.ts:17).
9. **Changing edit-mode logic** — order cancellation requires the exact enter → cancel
   (re-fetch between each) → exit state management of §9.4, including echoing the
   extracted `editMode` value.
10. **Custom User-Agent** — do not set one; use the platform default
    (v1: src/app/src-rn/api/gourmetClient.ts:16).

---

## 14. Error and ban-detection behavior

There is **no dedicated ban-detection signal** in v1. Failures surface as:

- Failed login (wrong credentials OR banned account — indistinguishable): the login POST
  returns HTTP 200 with the login page again; `isLoggedIn()` fails → error
  `Login failed: invalid credentials or account blocked`
  (v1: src/app/src-rn/api/gourmetApi.ts:94-96). The failed-login page shows the German
  message `Benutzername oder Passwort falsch.` in `div.alert.alert-danger`
  (fixture src/app/src-rn/__tests__/fixtures/gourmet/login-failed.html) — v1 does not
  parse it.
- Expired session with no cached credentials: `SessionExpiredError` (§7).
- Missing tokens/user info in HTML: the specific parser errors quoted in §3, §5, §9, §11 —
  the request is **not sent**.
- HTTP status ≥ 400: transport-level error from the client (§2).
- AddToMenuesCart `success:false`: `Add to cart failed: {message}` (§10.1). No real server
  `message` value is recorded anywhere in v1 (`Order deadline passed` exists only as a Jest
  mock, src/app/src-rn/__tests__/api/gourmetApi.test.ts:340 — do not treat it as an
  observed server string).

---

## 15. Known site behavior NOT implemented by v1 (do not add without evidence)

- `POST /umbraco/api/AlaEaterNotificationsApi/GetNotifications` — the website's own JS
  calls this after page load with `(shopModelId, eaterId)`
  (v1: analysis/playwright-findings.md:114-117); v1 never calls it (no reference anywhere
  in src-rn). Whether skipping it is ban-safe is unverified from source — see Open
  questions.
- The site shows a cookie-consent dialog; the Playwright analysis flagged it as possibly
  needing dismissal (v1: analysis/playwright-findings.md:164). v1 implements no handling
  for it, so v2 should likewise ignore it.
- The orders page contains a third form — a **main order form** (`ufprt` +
  `__ncforminfo`, action `/bestellungen/`) — that v1 never posts (CLAUDE.md "Forms on
  orders page" #1; analysis/playwright-findings.md:129-133; present in fixture
  orders-page.html:19-23 with no code reference). Token extraction on `/bestellungen/`
  must therefore target the toggle/cancel forms by their specific selectors (§9.2, §9.4),
  never "the first form on the page".

---

## 16. Code vs. documentation discrepancies (code wins)

Summary of all discrepancies noted inline, for the verifier:

| Topic | CLAUDE.md / playwright doc says | Code does (authoritative) |
|---|---|---|
| Login verification | regex `<a href="https://alaclickneu.gourmet.at/einstellungen/" class="navbar-link">` | 4 substring indicators (§4) |
| Menu item selector | `div.meal` | `div.row.hide-sm-down .meal` (§8.2) |
| Category regex | `MENÜ\s+([I]{1,3})` | `/MEN(?:Ü|U)\s+([I]{1,3})/i` (§8.3) |
| Menus URL | `/menus?page={0-9}` | `GET /menus/` (trailing slash); no `page` param on page 0 (§8.1) |
| Next-page selector | `a.menues-next` | `a[class*="menues-next"]` (§8.1) |
| Order approved | `.confirmed` class or `fa fa-check` | `.fa-check` or `.checkmark` (§9.1) |
| Edit-mode toggle payload | `editMode=True` | echo the extracted `editMode` value (`True` to enter, `False` to exit) (§9.2) |
| Username selector | `.loginname` | `span.loginname` (§5) |
| Title extraction | "`.title` first child text" | direct text nodes of `.title`, excluding the nested `.subtitle` div (§8.2) |

---

## 17. Test evidence

v1 behavior above is pinned by these tests and fixtures (carried to `docs/fixtures/` per
appendix-source-map.md):

- `src/app/src-rn/__tests__/api/gourmetClient.test.ts` — client config, Origin/Referer
  behavior, referer fallback after reset.
- `src/app/src-rn/__tests__/api/gourmetParser.test.ts` — token extraction + all
  missing-token error paths, login detection, user info, 7-item desktop-only menu parse,
  category detection incl. `UNKNOWN`, ordered/available/allergen/price edge cases,
  pagination detection, order parsing incl. both approved markers, edit-mode values,
  cancel-form extraction incl. id-selector fallback, logout-form extraction.
- `src/app/src-rn/__tests__/api/gourmetApi.test.ts` — exact login POST payload,
  stale-session pre-logout sequence, user-info re-fetch fallback, pagination calls
  (`/menus/` with `undefined` params then `{page:'1'}`), re-login + re-fetch on expiry,
  `SessionExpiredError`, addToCart grouping + exact JSON payload, confirm/cancel POST
  sequences, billing request payload + wrapper/array/empty response handling, logout.
- Fixtures: `src/app/src-rn/__tests__/fixtures/gourmet/{login-page,login-success,login-failed,menus-page-0,menus-page-1,orders-page,orders-page-edit-mode}.html`,
  `billing-current.json`, `billing-last-month.json` (sanitized live recordings; re-recordable
  via `npm run record-fixtures` — see 06-testing).

## Dropped in v2

The Gourmet API layer itself has no desktop/web branches. Desktop-only HTTP proxying
(`src-rn/utils/tauriHttp.ts`) that the Tauri build used to route these requests is dropped
along with the desktop target (see appendix-source-map.md). On v2 mobile targets, requests
go directly through the native HTTP stack exactly as described in §2.
