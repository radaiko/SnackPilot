# 02 — Ventopay (Automaten) API specification

Ventopay ("Automaten") is the cafeteria vending-machine / POS billing system. SnackPilot
reads the user's transactions from Ventopay's **"xperience" platform** for the Billing
feature. This doc is the complete HTTP + parsing contract.

> **2026-11 migration**: the old ASP.NET WebForms portal (`my.ventopay.com/mocca.website`)
> was retired. The xperience platform is an Angular SPA with a JSON auth service and an
> OpenAPI gateway — no HTML scraping, no ASP.NET form state. Section 8 records the
> migration decisions; the v1 (ASP.NET) contract lives only in git history.

How transactions are aggregated into monthly billing, cached, and displayed is owned by
`03-features/billing` and `03-features/caching`; credential storage and the auth store
lifecycle are owned by `03-features/settings` and `05-platform-services`; the demo-mode
substitute API is owned by `03-features/demo-mode`. Test fixtures and the record/replay
strategy are owned by `06-testing`.

**CRITICAL: Do not alter the request behavior specified here.** Deviating from the exact
request sequence, JSON shapes, headers or encodings risks account blocks on the external
service.

---

## 1. Constants

| Constant | Exact value |
|---|---|
| Web URL (SPA + auth service host) | `https://xperience-trumpf.ventopay.cloud` (no trailing slash) |
| API gateway URL | `https://gw-trumpf.ventopay.cloud` (from `GET /mocca.web.xml` → `<APIEndpoint>`) |
| Login URL | `https://xperience-trumpf.ventopay.cloud/auth/login` |
| Access-token URL | `https://xperience-trumpf.ventopay.cloud/auth/accessToken` |
| Logout URL | `https://xperience-trumpf.ventopay.cloud/auth/logout` |
| Transactions URL | `https://gw-trumpf.ventopay.cloud/Customers/v1/person/bons` |
| Restaurant ID (hardcoded) | `0da8d3ec-0178-47d5-9ccd-a996f04acb61` |
| Origin header value | `https://xperience-trumpf.ventopay.cloud` |

The restaurant ID is the `restaurant_ID` served by
`GET https://gw-trumpf.ventopay.cloud/Restaurant/v1/restaurants` (same UUID the retired
portal's company dropdown used) and must never be changed.

---

## 2. HTTP client requirements

### 2.1 General behavior

- **Manual cookie management** — the client maintains its own in-memory cookie jar; the
  platform HTTP stack must not add its own cookies (transport built with
  `cookie_store = false`). The auth service sets a `RefreshToken` cookie
  (`secure; samesite=strict; httponly`) on login; the gateway needs no cookies.
- **User-Agent**: the shared mobile-Safari UA from `docs/architecture §3.1` is sent
  (an absent UA is a bot signal, 01 §10.2). No other custom headers.
- **Every request carries `Accept: application/json, text/plain, */*`** (transport-level).
- **No request throttling or artificial delays.** One login per app session; the bons
  list is one GET per month fetched.
- **Redirects**: the transport follows redirects OFF for Ventopay; the client follows
  3xx manually (max 5) so `Set-Cookie` on intermediate hops is captured into the jar.
- **Status handling**: HTTP 200–399 is success; ≥ 400 raises
  `CoreError::Http { detail: "HTTP {status}", status: Some(status) }`.
- Responses are consumed as JSON text.

### 2.2 Cookie jar semantics

Same jar implementation as before (shared with nothing; Ventopay-only):

- On every response, for each `Set-Cookie` header: take the substring before the first
  `;`, split on the FIRST `=` into name and value (both trimmed), store `name → value`.
  Attributes (`Path`, `Expires`, `HttpOnly`, `Secure`, `Domain`) are ignored; setting an
  existing name overwrites its value. Entries without `=` or with an empty name are
  ignored.
- On every request, if the jar is non-empty, send ALL stored cookies as one `Cookie`
  header `name1=value1; name2=value2` (insertion order); if empty, send no header.
- `reset()` (logout) clears the jar, the bearer token and the tracked last-page URL.

### 2.3 GET requests

`get(url, params?)` performs a GET and returns the body as a string. Query parameters
are percent-encoded leaving only RFC 3986 unreserved characters (`A-Za-z0-9-._~`)
unencoded — the same serializer as the web app's `encodeURIComponent`, so ISO date
colons become `%3A` (e.g.
`…/bons?fromDate=2026-02-01T00%3A00%3A00&untilDate=2026-02-28T23%3A59%3A59`).

After each GET the client records the request URL (WITHOUT query) as `lastPageUrl` for
use as the `Referer` of the next POST/DELETE.

### 2.4 POST and DELETE requests

`post_json(url, body)` and `delete(url)` send, in addition to the jar Cookie header and
the bearer (§2.5):

- `Origin: https://xperience-trumpf.ventopay.cloud`
- `Referer: {lastPageUrl}` — the URL of the most recent GET (falls back to the request
  URL itself if no GET happened)
- POST bodies are `application/json` (transport sets `Content-Type`).

### 2.5 Bearer token

After `set_bearer(Some(token))` every request carries `authorization: Bearer {token}`
(lower-case header name, matching the web app). `set_bearer(None)` and `reset()` remove
it. The token is never logged.

---

## 3. Login flow

**DO NOT MODIFY THIS SEQUENCE.**

### Step 1 — GET the SPA shell

`GET https://xperience-trumpf.ventopay.cloud/` — seeds the cookie jar the way the
browser's first document request does. Body ignored.

### Step 2 — POST the login

`POST https://xperience-trumpf.ventopay.cloud/auth/login` with JSON body (field values
JSON-escaped):

```json
{
  "username": "{username}",
  "password": "{password}",
  "restaurantId": "0da8d3ec-0178-47d5-9ccd-a996f04acb61",
  "authEntity": "Person"
}
```

- `200` → JSON `{person, identification, otP_ID, entityType, mustProvideEmail,
  refreshToken}` plus `Set-Cookie: RefreshToken=…` (into the jar, §2.2).
- `401` → fail with error `Ventopay login failed: invalid credentials or account blocked`.
- `otP_ID` **non-null** → the account uses 2FA, which the app does not support: fail with
  error `Ventopay login failed: two-factor authentication is not supported`. (The
  reference account has `otP_ID: null`.)

### Step 3 — Fetch the access token

`GET https://xperience-trumpf.ventopay.cloud/auth/accessToken` (with the RefreshToken
cookie) → `{"accessToken": …, "expires": …, "refreshToken": …}`. Missing/empty
`accessToken` → parse error. Then `set_bearer(Some(accessToken))`.

On success store the credentials in memory for silent re-login and mark the session
authenticated.

### Token lifetime — `renewToken` deliberately NOT implemented

The access-token JWT `exp` is ~year 6128 (never expires in practice). The web app's
`PUT /auth/renewToken` (raw JWT text body; needs `Content-Type: application/json` +
`Content-Length: 0` or IIS answers 411) is therefore not replicated. Expiry is handled
by the gateway-401 rule in §4.

---

## 4. Session management

- The API layer keeps a boolean `loggedIn` flag and the in-memory credentials captured
  by the last successful `login()`.
- `ensure_session()`: if not logged in and credentials exist, run the full login flow
  again; if no credentials, fail with `CoreError::SessionExpired` (the v1 wording
  "Ventopay session expired and no credentials saved" is swallowed by
  `03-features/billing`).
- **`login()` must NOT clear the cookie jar or prior session state.** During silent
  re-login the stale cookies are still sent, exactly as a browser would. A **failed**
  login leaves the logged-in flag, stored credentials, jar and bearer at their previous
  values; only `logout()` resets them.
- **Bearer expiry detection**: if the gateway answers `401` to the bons GET, mark the
  session not-logged-in, drop the bearer, run `ensure_session()` (full re-login) and
  retry the bons GET **once**; the retry result is parsed without a second 401 check.
- Each Ventopay session is fully independent of the Gourmet session — separate cookie
  jar, separate bearer, separate credentials.

### Logout

`DELETE https://xperience-trumpf.ventopay.cloud/auth/logout` (with bearer + cookies;
`204` on success). Any error from this request is swallowed (logout is non-critical).
Regardless of outcome: clear the logged-in flag, discard stored credentials, reset the
client (jar + bearer + lastPageUrl).

---

## 5. Fetching transactions (bons)

`GET https://gw-trumpf.ventopay.cloud/Customers/v1/person/bons` with the bearer token
and two query parameters, in this order:

| Param | Format | Example (encoded) |
|---|---|---|
| `fromDate` | `YYYY-MM-DDT00:00:00` (local) | `2026-02-01T00%3A00%3A00` |
| `untilDate` | `YYYY-MM-DDT23:59:59` (local) | `2026-02-28T23%3A59%3A59` |

The server filters on `originalBonCreationDate` (verified: a bon whose
`bonCreationDate` is next-day is returned for the original date's range). Which date
ranges are requested (per month, etc.) is owned by `03-features/billing`.
`get_transactions` requires an authenticated session (`ensure_session()` first) and
applies the 401-retry rule from §4.

**There is NO per-transaction detail request.** All data comes from the bons list in a
single GET.

### 5.1 Skip rules

- Skip a bon whose `id` is missing or empty.
- If `originalBonCreationDate` is missing or unparseable, use the CURRENT date/time
  (`parse_bill_date` fallback → `now_epoch_ms`).

### 5.2 Gourmet filter

Skip (do not emit) any bon whose `restaurantName` contains `gourmet` case-insensitively.
Rationale: those purchases are already covered by the Gourmet billing system
(`01-gourmet-scraping`). There is no "Kaffeeautomat" exception (unchanged from v1).

### 5.3 Output data model

Each surviving bon yields:

| Field | Type | Value |
|---|---|---|
| `id` | string | `id` (GUID; opaque identity/key) |
| `date_epoch_ms` | int64 | `parse_bill_date(originalBonCreationDate)`, fallback now |
| `amount` | f64 (EUR) | `bonTotal` |
| `restaurant` | string | `restaurantName` |
| `location` | string | **same value as `restaurant`** (duplicated, v1 parity) |

Transactions are returned in response order. An empty `bons` array yields an empty list
(no error).

---

## 6. Bons response shape

`{"error": "ERR_NONE", "bons": [ … ]}`. `error` ≠ `ERR_NONE` → `CoreError::Parse`.
Per-bon fields used: `id`, `bonTotal`, `restaurantName`, `originalBonCreationDate`.
Other fields (`bonNumber`, `cashPointName`, `bonType`, `bonCreationDate`, payment-type
amounts, …) are ignored. Notes:

- .NET serialises zero amounts as `0E-10` — a valid JSON number; the parser must accept
  it (serde_json does).
- `originalBonCreationDate` is a naive local ISO timestamp (`2026-02-09T11:49:00`,
  no timezone) → parsed as **local** time (same rule as Gourmet bill dates).
- `restaurantName` observed values: `Café + Co. Automaten`, `Gourmet Betriebsverpflegung`.
- A gateway request with a bad/missing bearer answers `401` (drives §4).

---

## 7. Things that break accounts

1. Changing the hardcoded restaurant UUID `0da8d3ec-0178-47d5-9ccd-a996f04acb61`.
2. Changing literal JSON field names or `authEntity: "Person"` on the login body.
3. Sending the bons query dates in any format other than §5 (the server filters on
   `originalBonCreationDate` with `fromDate`/`untilDate` names).
4. Reordering requests — login must complete (token fetched) before the bons GET.
5. Dual cookie management — exactly one cookie jar (the app's) may be active.
6. Adding throttling/delays (may cause session timeout) or a custom User-Agent beyond
   the shared one in §2.1.
7. Calling `PUT /auth/renewToken` or the 2FA endpoints (`/authStep1`, `/unifiedStep2`) —
   not part of this contract.
8. Polling the gateway more than once per billing month fetch.

---

## 8. Migration decisions (2026-11, v1 ASP.NET → xperience)

- **Login**: ASP.NET 11-field form POST → JSON `POST /auth/login` (§3). The login
  response's `otP_ID` gates 2FA (unsupported → hard error; the reference account has
  none).
- **Auth to the gateway**: cookie session → `authorization: Bearer` from
  `GET /auth/accessToken` (§2.5, §3 step 3).
- **`renewToken` not implemented** — the JWT never expires in practice (§3 note);
  gateway 401 triggers full re-login + single retry (§4), faithful to v1's
  expiry-retry shape.
- **Transactions**: HTML `Transaktionen.aspx` scrape → `GET /Customers/v1/person/bons`
  JSON (§5). All v1 HTML helpers (ASP.NET state, German amount/timestamp parsing) are
  deleted.
- **Logout**: GET `Ausloggen.aspx` → `DELETE /auth/logout` (§4).
- Verified live 2026-11-06 against the production tenant with one login, a handful of
  calls, then `DELETE /auth/logout` (refresh cookie dead afterwards, `401`).

---

## 9. Test expectations (evidence)

- Login performs exactly: shell GET, login POST, accessToken GET; the POST body carries
  `username`, `password`, `restaurantId`, `authEntity: "Person"`; every later request
  carries `authorization: Bearer …`.
- Login `401` → error `Ventopay login failed: invalid credentials or account blocked`;
  `otP_ID` non-null → `… two-factor authentication is not supported`.
- Fixture `bons.json` has 6 bons; 1 (`Gourmet Betriebsverpflegung`) is filtered →
  5 returned; first is id `687efd5d-0fe5-4149-9e61-7183b9a616a3`, amount `1.8`,
  restaurant `Café + Co. Automaten`, date 2026-02-09 11:49 local.
- Bons URL encodes dates as `…T00%3A00%3A00` / `…T23%3A59%3A59` (§2.3).
- Gateway `401` on the bons GET → exactly one full re-login (3 requests) + one retry.
- `error` ≠ `ERR_NONE` → parse error; `0E-10` amount parses as `0.0`.
- Logout DELETEs `/auth/logout`, errors swallowed, `is_authenticated()` → false, jar and
  bearer cleared.
- Cookie jar: `RefreshToken=abc; secure; httponly` response → next request carries
  `Cookie: RefreshToken=abc`; no header when jar empty.

Note: fixture bon ids and dates are sanitized placeholders; treat the live `id` as an
opaque string.
