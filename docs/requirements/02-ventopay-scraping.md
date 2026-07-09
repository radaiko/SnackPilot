# 02 — Ventopay (Automaten) scraping specification

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

Ventopay ("Automaten") is the cafeteria vending-machine / POS billing system. SnackPilot
scrapes its ASP.NET WebForms website (no API) to list the user's transactions for the
Billing feature. This doc is the complete HTTP + parsing contract. How transactions are
aggregated into monthly billing, cached, and displayed is owned by
`03-features/billing` and `03-features/caching`; credential storage and the auth store
lifecycle are owned by `03-features/settings` and `05-platform-services`; the demo-mode
substitute API is owned by `03-features/demo-mode`. Test fixtures and the record/replay
strategy are owned by `06-testing`.

**CRITICAL: Do not alter the scraping behavior specified here.** Deviating from the exact
request sequence, form fields, or encodings risks account blocks on the external service
(v1: CLAUDE.md "Critical Warning").

---

## 1. Constants

All values from (v1: src/app/src-rn/utils/constants.ts:16-20):

| Constant | Exact value |
|---|---|
| Base URL | `https://my.ventopay.com/mocca.website` (no trailing slash) |
| Login URL | `https://my.ventopay.com/mocca.website/Login.aspx` |
| Transactions URL | `https://my.ventopay.com/mocca.website/Transaktionen.aspx` |
| Logout URL | `https://my.ventopay.com/mocca.website/Ausloggen.aspx` |
| Company ID (hardcoded) | `0da8d3ec-0178-47d5-9ccd-a996f04acb61` |
| Origin header value | `https://my.ventopay.com` (v1: src/app/src-rn/api/ventopayClient.ts:4) |

The company ID is the value of the `DropDownList1` company selector on the login page and
must never be changed (v1: CLAUDE.md "Modifying hardcoded company ID").

---

## 2. HTTP client requirements

Source: (v1: src/app/src-rn/api/ventopayClient.ts).

### 2.1 General behavior

- **Manual cookie management** — the client maintains its own in-memory cookie jar and
  must NOT let the platform HTTP stack manage cookies for these requests.
  **v1 mechanism**: an Axios instance with `withCredentials: false`; a response
  interceptor captures `Set-Cookie` headers, a request interceptor injects a `Cookie`
  header (v1: src/app/src-rn/api/ventopayClient.ts:24-58). The file comment explains why:
  React Native's native HTTP stack doesn't reliably persist `Set-Cookie` between
  requests, and enabling native cookie handling (`withCredentials: true`) alongside the
  manual jar creates a dual-cookie conflict with NSURLSession
  (v1: src/app/src-rn/api/ventopayClient.ts:9-16). In v2 the Rust core must own the
  cookie jar exclusively — the platform networking layer must not add its own cookies.
- **No custom `User-Agent` header** — the platform default is sent as-is
  (v1: src/app/src-rn/api/ventopayClient.ts:12).
- **Every request additionally carries `Accept: application/json, text/plain, */*`.**
  This is an app-level axios default (`headers.common.Accept`,
  v1: src/app/node_modules/axios/lib/defaults/index.js:167), NOT a platform default —
  v2 must send this exact value on every GET and POST. Header ownership: app-set are
  `Accept` (all requests), `Cookie` (per §2.2), and for POSTs
  `Content-Type`/`Origin`/`Referer` (§2.4); platform-stack defaults apply to
  `User-Agent`, `Accept-Encoding`, `Accept-Language`, `Connection`.
- **No request throttling or artificial delays**
  (v1: src/app/src-rn/api/ventopayClient.ts:13; CLAUDE.md "Rate limiting/delays").
- **Redirects**: followed transparently by the transport. v1 sets `maxRedirects: 5`
  (v1: src/app/src-rn/api/ventopayClient.ts:27), but this axios option is **inert in
  React Native's XHR adapter** — the native stack followed redirects with no 5-hop limit,
  so any sane redirect limit is acceptable in v2. The consequential rule is cookie
  capture, below (§2.2): only the FINAL response of a redirect chain is visible to the
  jar.
- **Status handling**: HTTP status 200–399 is success; 4xx/5xx must raise an error
  (v1: src/app/src-rn/api/ventopayClient.ts:28, `validateStatus: (status) => status >= 200 && status < 400`).
- Responses are consumed as text (HTML), not parsed as JSON
  (v1: src/app/src-rn/api/ventopayClient.ts:65,85 `responseType: 'text'`).

### 2.2 Cookie jar semantics

(v1: src/app/src-rn/api/ventopayClient.ts:31-58)

- On every **non-error final response** (HTTP 200–399), for each `Set-Cookie` header
  (handle both a single string and an array of strings): take the substring before the
  first `;`, split it on the FIRST `=` into name and value (both trimmed), and store
  `name → value`. Cookie attributes (`Path`, `Expires`, `HttpOnly`, `Secure`, `Domain`)
  are **ignored**; cookies never expire within a session. Setting an existing name
  overwrites its value (insertion position preserved).
  Two capture restrictions a v2 client that follows redirects itself must replicate:
  - **Final-response-only**: v1's interceptor only ever saw the last response of a
    redirect chain — `Set-Cookie` on intermediate 3xx hops (e.g. an ASP.NET post-login
    302) was never stored. v2 must NOT add intermediate-hop cookies to the jar, while
    the outgoing `Cookie`/`Origin`/`Referer` headers carry through redirect hops as the
    transport default.
  - **Success-only**: 4xx/5xx responses raise before the capture handler runs
    (`interceptors.response.use` registers no rejection handler,
    v1: src/app/src-rn/api/ventopayClient.ts:28,32) — cookies on error responses are
    never persisted.
- Entries where `=` is absent or at index 0 are ignored
  (v1: src/app/src-rn/api/ventopayClient.ts:39 `if (eqIndex > 0)`).
- On every request, if the jar is non-empty, send ALL stored cookies as a single
  `Cookie` header formatted `name1=value1; name2=value2` (joined with `"; "`, insertion
  order). If the jar is empty, send no `Cookie` header at all
  (v1: src/app/src-rn/api/ventopayClient.ts:50-58; test: src/app/src-rn/__tests__/api/ventopayClient.test.ts:164-189).
- `resetClient()` (used on logout) clears the jar and the tracked last-page URL
  (v1: src/app/src-rn/api/ventopayClient.ts:97-100).

### 2.3 GET requests

`get(url, params?)` performs a GET with the given query parameters and returns the
response body as a string (v1: src/app/src-rn/api/ventopayClient.ts:62-72).
Query parameters are appended by axios' default serializer
(e.g. `Transaktionen.aspx?fromDate=01.02.2026&untilDate=28.02.2026` — `.` is not
percent-encoded).

After each GET, the client records the request URL (WITHOUT query parameters) as
`lastPageUrl` for use as the `Referer` of the next POST
(v1: src/app/src-rn/api/ventopayClient.ts:67-70). If a relative URL were used it would be
resolved against the base URL, but in practice every call site passes an absolute URL.

### 2.4 POST requests (form submissions)

`postForm(url, data)` (v1: src/app/src-rn/api/ventopayClient.ts:75-89):

- **Content-Type: `application/x-www-form-urlencoded`** — NOT multipart. (This is the
  opposite of the Gourmet system; see `01-gourmet-scraping`.)
- Body is standard URL-encoding of the field map in insertion order.
  **v1 mechanism**: `new URLSearchParams(data).toString()` — spaces become `+`, and
  base64 characters in ASP.NET state values are percent-encoded (`+` → `%2B`,
  `/` → `%2F`, `=` → `%3D`). v2 must produce equivalent encoding.
- Headers sent in addition to Content-Type:
  - `Origin: https://my.ventopay.com`
  - `Referer: {lastPageUrl}` — the URL of the most recent GET (falls back to the POST
    URL itself if no GET happened) (v1: src/app/src-rn/api/ventopayClient.ts:83).
- The stored session cookies are injected as described in 2.2.

---

## 3. Login flow

Source: (v1: src/app/src-rn/api/ventopayApi.ts:32-62). **DO NOT MODIFY THIS SEQUENCE.**

### Step 1 — GET the login page

`GET https://my.ventopay.com/mocca.website/Login.aspx`
(v1: src/app/src-rn/api/ventopayApi.ts:34). This both seeds the session cookies and
provides the ASP.NET hidden form state.

### Step 2 — Extract ASP.NET state

Extract SIX hidden inputs by element id (v1: src/app/src-rn/api/ventopayParser.ts:20-39):

| Field | CSS selector (value attribute) | Required? |
|---|---|---|
| `__LASTFOCUS` | `#__LASTFOCUS` | optional, default `''` |
| `__EVENTTARGET` | `#__EVENTTARGET` | optional, default `''` |
| `__EVENTARGUMENT` | `#__EVENTARGUMENT` | optional, default `''` |
| `__VIEWSTATE` | `#__VIEWSTATE` | required |
| `__VIEWSTATEGENERATOR` | `#__VIEWSTATEGENERATOR` | required |
| `__EVENTVALIDATION` | `#__EVENTVALIDATION` | required |

If any of the three required values is missing or empty, fail with error
`Could not extract ASP.NET state from page`
(v1: src/app/src-rn/api/ventopayParser.ts:27-29). `__LASTFOCUS`, `__EVENTTARGET`, and
`__EVENTARGUMENT` are typically empty but MUST still be included in the POST
(v1: src/app/src-rn/api/ventopayParser.ts:17-18).

### Step 3 — POST the login form

`POST https://my.ventopay.com/mocca.website/Login.aspx` as
`application/x-www-form-urlencoded` (see 2.4) with EXACTLY these 11 fields in this
order ("exact browser order", v1: src/app/src-rn/api/ventopayApi.ts:38-51):

```
__LASTFOCUS:          {extracted, usually ""}
__EVENTTARGET:        {extracted, usually ""}
__EVENTARGUMENT:      {extracted, usually ""}
__VIEWSTATE:          {extracted}
__VIEWSTATEGENERATOR: {extracted}
__EVENTVALIDATION:    {extracted}
DropDownList1:        0da8d3ec-0178-47d5-9ccd-a996f04acb61
TxtUsername:          {username}
TxtPassword:          {password}
BtnLogin:             Login
languageRadio:        DE
```

`BtnLogin` is the literal string `Login`; `languageRadio` is the literal string `DE`.
The `Referer` on this POST is `https://my.ventopay.com/mocca.website/Login.aspx`
(from the Step-1 GET).

### Step 4 — Verify login

Test the POST response HTML against the regex (v1: src/app/src-rn/api/ventopayParser.ts:46):

```regex
/href="Ausloggen\.aspx"/i
```

(case-insensitive, tests for the logout link). If it does not match, fail with error
`Ventopay login failed: invalid credentials or account blocked`
(v1: src/app/src-rn/api/ventopayApi.ts:56-58). On success, store the credentials in
memory for silent re-login and mark the session authenticated
(v1: src/app/src-rn/api/ventopayApi.ts:60-61).

> **Discrepancy (code wins)**: CLAUDE.md's Ventopay section lists only the three
> `__VIEWSTATE*`/`__EVENTVALIDATION` fields and orders `DropDownList1` first, and gives
> the verification regex as `<a\s+href="Ausloggen.aspx">`. The code additionally sends
> `__LASTFOCUS`, `__EVENTTARGET`, `__EVENTARGUMENT`, orders the ASP.NET fields first,
> and verifies with the looser case-insensitive `/href="Ausloggen\.aspx"/i` (no `<a`
> prefix, escaped dot). Follow the code.

---

## 4. Session management

(v1: src/app/src-rn/api/ventopayApi.ts:15-76)

- The API layer keeps a boolean `loggedIn` flag and the in-memory credentials captured
  by the last successful `login()`.
- `ensureSession()`: if not logged in and credentials exist, run the full login flow
  again; if no credentials, fail with error
  `Ventopay session expired and no credentials saved`
  (v1: src/app/src-rn/api/ventopayApi.ts:67-76).
- **`login()` must NOT clear the cookie jar or prior session state.** During silent
  re-login the stale cookies from the expired session are still sent with the login GET
  and POST, exactly as a browser would (v1: src/app/src-rn/api/ventopayApi.ts:32-62 —
  `login()` never calls `resetClient()`). A **failed** login leaves the logged-in flag,
  stored credentials, and cookie jar at their previous values (the assignments happen
  only on the success path); only `logout()` resets them via `resetClient()`
  (v1: src/app/src-rn/api/ventopayApi.ts:97-99,121; ventopayClient.ts:97-100).
- **Expiry detection during fetch**: after fetching the transactions page, run the same
  logged-in regex (`/href="Ausloggen\.aspx"/i`) against the response. If it does not
  match, mark the session expired, re-login via `ensureSession()`, and retry the fetch
  ONCE; the retry response is parsed without a second logged-in check
  (v1: src/app/src-rn/api/ventopayApi.ts:96-105; test:
  src/app/src-rn/__tests__/api/ventopayApi.test.ts:139-156).
- Each Ventopay session is fully independent of the Gourmet session — separate cookie
  jar, separate credentials.

### Logout

`GET https://my.ventopay.com/mocca.website/Ausloggen.aspx`. Any error from this request
is swallowed (logout is non-critical). Regardless of outcome: clear the logged-in flag,
discard stored credentials, and reset the cookie jar
(v1: src/app/src-rn/api/ventopayApi.ts:113-123).

---

## 5. Fetching transactions

(v1: src/app/src-rn/api/ventopayApi.ts:85-108)

`GET https://my.ventopay.com/mocca.website/Transaktionen.aspx` with two query
parameters, in this order:

| Param | Format | Example |
|---|---|---|
| `fromDate` | `dd.MM.yyyy`, zero-padded, local time | `01.02.2026` |
| `untilDate` | `dd.MM.yyyy`, zero-padded, local time | `28.02.2026` |

Date formatting: day and month zero-padded to 2 digits, 4-digit year, joined with `.`
(v1: src/app/src-rn/api/ventopayApi.ts:131-137). Which date ranges are requested (per
month, etc.) is owned by `03-features/billing`.

The response HTML is parsed per section 6. `getTransactions` requires an authenticated
session (calls `ensureSession()` first) and applies the expiry-retry rule from section 4.

**There is NO per-transaction detail request.** All data comes from the list page in a
single GET. See section 8 for the discrepancy with CLAUDE.md.

---

## 6. Parsing the transactions page

Source: (v1: src/app/src-rn/api/ventopayParser.ts:49-152). v1 parses with Cheerio
(jQuery-like CSS selection over the HTML string); v2 needs an equivalent HTML parser.

### 6.1 Transaction elements

Selector: `div.transact` (v1: src/app/src-rn/api/ventopayParser.ts:129). Observed
structure (v1: src/app/src-rn/api/ventopayParser.ts:114-120 and fixture
src/app/src-rn/__tests__/fixtures/ventopay/transactions-page.html):

```html
<div class="transact" id="{transactionId}">
  <a href="rechnung.aspx?id={transactionId}">
    <div class="transact_title">€ 1,80 (Café + Co. Automaten)</div>
    <div class="transact_timestamp">09. Feb 2026 - 11:49 Uhr</div>
  </a>
</div>
```

Per element:

| Field | Extraction |
|---|---|
| `id` | `id` attribute of the `div.transact` itself |
| title text | text of descendant `.transact_title`, trimmed |
| timestamp text | text of descendant `.transact_timestamp`, trimmed |

Skip rules (v1: src/app/src-rn/api/ventopayParser.ts:132-139):
- Skip the element if the `id` attribute is missing or empty.
- Skip the element if the title text is empty.
- If the timestamp text is empty, use the CURRENT date/time instead of skipping
  (v1: src/app/src-rn/api/ventopayParser.ts:146).

> **Discrepancy (code wins)**: CLAUDE.md gives the selector as
> `div.content div.transact`; the code selects `div.transact` with no ancestor
> constraint.

### 6.2 Title parsing (amount + restaurant)

(v1: src/app/src-rn/api/ventopayParser.ts:92-109)

Match the trimmed title against:

```regex
/€\s*([\d,]+)\s*\((.+)\)/
```

- Group 1 → amount (German number parsing, 6.3). Group 2 (trimmed) → restaurant name.
  Note `(.+)` is greedy, so with nested parentheses everything up to the LAST `)` is
  taken.
- **Fallback** (no match): amount = German number parsing of the whole trimmed title;
  restaurant = the whole trimmed title (test:
  src/app/src-rn/__tests__/api/ventopayParser.test.ts:132-147, e.g. title `1,80` →
  amount `1.8`, restaurant `"1,80"`).

### 6.3 German number parsing

(v1: src/app/src-rn/api/ventopayParser.ts:82-85)

1. Remove every character that is not a digit, `,`, or `-`:
   `text.replace(/[^\d,\-]/g, '')` — this strips `€`, spaces, and any `.` thousands
   separators (`"1.234,56"` → `"1234,56"`).
2. Replace the FIRST `,` with `.`.
3. Parse as a float; if the result is NaN, use `0`.
   **v1 mechanism — prefix-lenient parse**: v1 uses JavaScript `parseFloat`, which parses
   the longest leading numeric prefix and silently ignores trailing characters
   (e.g. cleaned `"1.802,00"` → `1.802`; `"12-34"` → `12`); only a string with no leading
   numeric prefix yields NaN → `0`. A strict float parse (the natural choice in Rust)
   returns 0 where v1 returns a number — v2 must replicate the prefix parse or explicitly
   accept divergence on malformed titles.

Examples: `€ 1,80` → `1.8`; `0,50` → `0.5`; garbage → `0`.

### 6.4 Timestamp parsing

(v1: src/app/src-rn/api/ventopayParser.ts:50-77)

Observed formats: `09. Feb 2026 - 11:49 Uhr`, `03. Mrz 2026 - 11:40 Uhr`,
`15. Jän 2026 - 09:15 Uhr`. Match the trimmed text against (Unicode mode; `\p{L}`
matches letters including umlauts):

```regex
/(\d{1,2})\.\s*(\p{L}{3})\p{L}*\s+(\d{4})\s*-\s*(\d{1,2}):(\d{2})/u
```

- Group 1 = day, group 2 = first three letters of the month name, group 3 = 4-digit
  year, group 4 = hours, group 5 = minutes. Seconds are always 0. The result is a
  LOCAL date-time.
- Month lookup: lowercase group 2 and map via this exact table
  (v1: src/app/src-rn/api/ventopayParser.ts:50-54; 0-based month index):

  ```
  jan→0  jän→0  feb→1  mär→2  mar→2  mrz→2  apr→3
  mai→4  jun→5  jul→6  aug→7  sep→8  okt→9  nov→10  dez→11
  ```

  Both the Austrian `Jän` (January) and the `Mrz`/`Mär`/`Mar` variants (March) must
  work (tests: src/app/src-rn/__tests__/api/ventopayParser.test.ts:91-105).
- An unknown month abbreviation falls back to month 0 (January)
  (v1: src/app/src-rn/api/ventopayParser.ts:72 `GERMAN_MONTHS[monthStr] ?? 0`).
- **Fallback** (regex does not match): parse the whole trimmed string with the host
  date parser — **v1 mechanism**: `new Date(trimmed)`, which handles ISO 8601 strings
  like `2026-02-09T11:49:00` (test:
  src/app/src-rn/__tests__/api/ventopayParser.test.ts:114-130). v2 should keep ISO 8601
  as the fallback format; other unparseable strings produced an Invalid Date in v1
  (behavior undefined downstream).

> **Discrepancy (code wins)**: CLAUDE.md documents the timestamp regex as
> `(\d+)\.\s+([a-zA-z]+)\s+(\d+)\s+-\s+(\d+):(\d+)` (attributed to the detail page).
> The code uses the Unicode-aware regex above on the LIST page and requires exactly a
> 4-digit year and 2-digit minutes. Follow the code.

### 6.5 Gourmet filter

Skip (do not emit) any transaction whose parsed restaurant name contains `gourmet`
case-insensitively (v1: src/app/src-rn/api/ventopayParser.ts:143-144:
`restaurant.toLowerCase().includes('gourmet')`). Rationale: those purchases are already
covered by the Gourmet billing system (`01-gourmet-scraping`).

> **Discrepancy (code wins)**: CLAUDE.md states the filter as "Skip transactions where
> restaurant name contains 'Gourmet' AND location does NOT contain 'Kaffeeautomat'".
> The code has NO Kaffeeautomat exception — any restaurant containing "gourmet" is
> filtered unconditionally. (The code also sets `location` equal to `restaurant`, so
> the CLAUDE.md rule could not be evaluated as written.)

### 6.6 Output data model

Each surviving element yields (v1: src/app/src-rn/types/ventopay.ts:2-8;
src/app/src-rn/api/ventopayParser.ts:148):

| Field | Type | Value |
|---|---|---|
| `id` | string | `id` attribute (opaque; used only as identity/key) |
| `date` | date-time | parsed timestamp (6.4) |
| `amount` | number (EUR) | parsed amount (6.2/6.3) |
| `restaurant` | string | parsed restaurant name |
| `location` | string | **same value as `restaurant`** (duplicated) |

Transactions are returned in document order. An empty transactions page yields an empty
list (no error) (test: src/app/src-rn/__tests__/api/ventopayParser.test.ts:107-110).

The `VentopayMonthlyBilling` aggregate (v1: src/app/src-rn/types/ventopay.ts:11-17) is
built downstream — owned by `03-features/billing`.

---

## 7. Things that will break accounts

From (v1: CLAUDE.md "Things That Will Break Accounts") as applicable to Ventopay,
plus code-level invariants:

1. Stale or missing ASP.NET state — extract fresh `__VIEWSTATE`,
   `__VIEWSTATEGENERATOR`, `__EVENTVALIDATION` from a fresh GET before every login POST.
2. Omitting the empty `__LASTFOCUS` / `__EVENTTARGET` / `__EVENTARGUMENT` fields.
3. Wrong date format — Ventopay uses `dd.MM.yyyy` (Gourmet uses `MM-dd-yyyy`).
4. Changing the hardcoded company UUID `0da8d3ec-0178-47d5-9ccd-a996f04acb61`.
5. Changing literal values `BtnLogin=Login`, `languageRadio=DE`.
6. Adding throttling/delays (may cause session timeout) or custom User-Agent headers.
7. Reordering requests — login must complete (verified) before the transactions GET.
8. Dual cookie management — exactly one cookie jar (the app's) must be active.

---

## 8. Discrepancies between code and CLAUDE.md (code wins)

Summarized from the inline notes above, plus one large one:

1. **Transaction detail scraping is documented but NOT implemented.** CLAUDE.md
   specifies fetching `Rechnung.aspx?id={transactionId}` and parsing
   `#ContentPlaceHolder1_LblTimestamp`, `#ContentPlaceHolder1_LblRestaurantInfo`
   (split by `<br>`), and item rows from `div.rechnungpart table tbody` (excluding
   `rechnungsdetail` rows; column 0 = count like `2x`, column 1 = item name,
   column 4 = cost in German format `12,34`). At v1.4.5 (@6997c44) **no code performs
   this request or parsing** — `rechnung.aspx` (always lowercase in v1; the capitalized
   form exists only in CLAUDE.md) appears only as an href inside test/fixture HTML and a
   parser doc-comment (v1: src/app/src-rn/api/ventopayParser.ts:116), and no
   `ContentPlaceHolder`/`rechnungpart` selector exists anywhere in `src-rn`. All
   transaction data (id, timestamp, amount, restaurant) comes from the list page.
   v2 must NOT add detail requests to replicate v1 behavior.
2. Login POST fields/order and verification regex — section 3.
3. Transactions selector `div.transact` vs documented `div.content div.transact` —
   section 6.1.
4. Timestamp regex — section 6.4.
5. Gourmet filter has no Kaffeeautomat exception — section 6.5.
6. Item-row column semantics (count/name/cost columns) exist only in the unimplemented
   detail-page spec (point 1); there is no item-level data in v1's Ventopay pipeline.

---

## 9. Dropped in v2

The Ventopay API files contain no desktop/web platform branches. On the (dropped)
desktop/web targets the same code ran inside the Tauri webview / browser; only the
mobile behavior described above carries to v2. The `withCredentials`/interceptor design
is an RN-axios mechanism ("v1 mechanism" notes above); the v2 requirement is the
platform-neutral one: a single app-owned cookie jar with the semantics of section 2.2.

---

## 10. Test expectations (evidence)

From (v1: src/app/src-rn/__tests__/api/ventopayParser.test.ts,
ventopayApi.test.ts, ventopayClient.test.ts and fixtures under
src/app/src-rn/__tests__/fixtures/ventopay/):

- Login POST carries exactly the 11 fields of section 3 with the fixture's state values
  verbatim (ventopayApi.test.ts:51-74).
- A login response without the `Ausloggen.aspx` link → error
  `Ventopay login failed: invalid credentials or account blocked`.
- Fixture list page has 6 transactions; 1 (`Gourmet Betriebsrestaurant`) is filtered →
  5 returned; first is id `dHhuLTAwMQ==`, amount `1.8`,
  restaurant `Café + Co. Automaten`, date 2026-02-09 11:49 local.
- Amount parsing: `€ 1,80`→1.8, `€ 3,20`→3.2, `€ 0,50`→0.5, `€ 2,40`→2.4, `€ 1,50`→1.5.
- Month variants: `03. Mrz 2026` → March 3, 2026; `15. Jän 2026` → January 15, 2026.
- Session-expiry retry performs exactly 3 GETs total: expired transactions page, login
  page, retried transactions page (ventopayApi.test.ts:139-156).
- Logout GETs `Ausloggen.aspx` and resets the cookie jar; `isAuthenticated()` → false.
- Cookie jar: `SessId=abc; path=/` + `Token=def; path=/` responses →
  `Cookie: SessId=abc; Token=def` on the next request; no header when jar empty.

Note: fixture transaction ids (`dHhuLTAwMQ==` etc.) are sanitized placeholders; treat
the live `id` attribute as an opaque string.
