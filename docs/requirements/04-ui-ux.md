# UI/UX specification

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This document specifies every screen, its navigation, layout, content, interactions, and
the shared UI components, precisely enough to rebuild the v1 mobile UI without access to
the v1 source. All user-facing text is German and must be reproduced verbatim.

Cross-references (do not restate here):

- Store behavior backing each screen (fetch/caching/pending-order semantics):
  **03-features/menus**, **03-features/orders**, **03-features/billing**,
  **03-features/settings**, **03-features/caching**.
- Theme system (light/dark resolution, accent-color persistence, alternate app icons):
  **03-features/themes**. Color token *values* are restated in §8 here because layout
  specs reference them.
- Notifications feature logic (reminder scheduling, geofencing, background tasks):
  **03-features/notifications-*** and **03-features/notification-log**.
- Analytics event names/transport (`trackSignal`): **03-features/analytics**.
- HTTP specifics: **01-gourmet-scraping**, **02-ventopay-scraping**.

Platform scope: v2 targets **iOS and Android only**. v1 additionally had web and
Tauri-desktop targets; every desktop/web-only branch below is marked **Dropped in v2**.

---

## 1. Screen inventory & navigation graph

v1 uses file-based routing (Expo Router). Structure (v1: src/app/app/_layout.tsx:95-106,
src/app/app/(tabs)/_layout.tsx):

```
Root stack (no headers on any screen)
├── (tabs)                       Tab container — 4 tabs
│   ├── index      "Menüs"          Menu browsing + ordering
│   ├── orders     "Bestellungen"   Ordered menus (upcoming/past)
│   ├── billing    "Abrechnung"     Combined billing (Kantine + Automaten)
│   └── settings   "Einstellungen"  Settings hub
├── kantine-login                Gourmet credentials      (push, card presentation)
├── automaten-login              Ventopay credentials     (push, card presentation)
├── notifications                Notification settings    (push, card presentation)
├── appearance                   Theme & accent color     (push, card presentation)
└── +not-found                   Fallback for unknown routes
```

- Every stack screen is declared with `headerShown: false`; the four sub-screens use
  `presentation: 'card'` (standard push transition) (v1: src/app/app/_layout.tsx:98-102).
  Screens draw their own back affordance (§3.5–3.8).
- The system status bar uses automatic styling (adapts to light/dark theme)
  (v1: src/app/app/_layout.tsx:104).
- The four sub-screens are reachable only from the Settings tab (§3.4). There are no
  deep links defined beyond the router defaults.
- On unknown route, the not-found screen shows — nav header IS shown for this screen
  with title `Hoppla!`, body text `Diese Seite existiert nicht.` and a link
  `Zur Startseite!` navigating to `/` (v1: src/app/app/+not-found.tsx:7-13; container
  centered, padding 20; link marginTop 15, paddingVertical 15).

Note: v1 contains a vestigial `App.tsx`/`index.ts` pair (template "Open up App.tsx…"
screen); it is dead code — `package.json` `"main": "expo-router/entry"` routes the app
through the `app/` directory instead (v1: src/app/package.json:4, src/app/App.tsx,
src/app/index.ts). Do not reproduce.

### 1.1 App bootstrap (root layout)

On app start, in order (v1: src/app/app/_layout.tsx:30-107):

1. Wrap the whole app in the analytics provider **only in release builds**; in dev
   builds analytics is omitted entirely (v1: src/app/app/_layout.tsx:109-119; see
   03-features/analytics).
2. Wrap the navigation stack in the dialog provider (§4.1) so any screen can show
   dialogs (v1: src/app/app/_layout.tsx:96).
3. On first mount:
   a. (native) migrate keychain accessibility for the 4 credential keys
      `gourmet_username`, `gourmet_password`, `ventopay_username`, `ventopay_password`
      (v1: src/app/app/_layout.tsx:39-42; see 05-platform-services);
   b. trigger auto-login with saved credentials for BOTH services concurrently
      (`loginWithSaved`, fire-and-forget) (v1: src/app/app/_layout.tsx:44-45).
4. When Gourmet auth status becomes `authenticated`: register the background menu-check
   task and request notification permissions, best-effort/silent
   (v1: src/app/app/_layout.tsx:49-59; see 03-features/notifications-new-menu).
5. On mount (native): set up the notification handler and the Android notification
   channel (v1: src/app/app/_layout.tsx:69-73; see 05-platform-services).
6. When a company location exists: enable location notifications
   (v1: src/app/app/_layout.tsx:75-78; see 03-features/notifications-location).
7. On mount: if the daily reminder is enabled, register the background sync task,
   best-effort/silent (v1: src/app/app/_layout.tsx:81-93; see
   03-features/notifications-daily-reminder).

**Dropped in v2:** the `tauriHttp` axios patch import (desktop HTTP proxying) and the
web `document.documentElement.style.colorScheme` sync (v1: src/app/app/_layout.tsx:8,
61-65).

---

## 2. Tab bar

Tab icons (Ionicons glyph names, outline when inactive / filled when active)
(v1: src/app/app/(tabs)/_layout.tsx:11-16):

| Tab | Title | Inactive icon | Active icon |
|---|---|---|---|
| index | `Menüs` | `restaurant-outline` | `restaurant` |
| orders | `Bestellungen` | `receipt-outline` | `receipt` |
| billing | `Abrechnung` | `wallet-outline` | `wallet` |
| settings | `Einstellungen` | `settings-outline` | `settings` |

Active icon/label color: `colors.primary`; inactive: `colors.textTertiary`.

### 2.1 iOS: floating glass pill ("GlassTabBar")

(v1: src/app/app/(tabs)/_layout.tsx:18-128, 186-196)

A custom floating pill replaces the system tab bar. **Icons only — no labels.**

- Wrapper: absolutely positioned, full width, children centered; `bottom =
  max(safeAreaBottomInset, 8)` (v1: (tabs)/_layout.tsx:23).
- Pill: width 248, height 56, borderRadius 28, `overflow: hidden`; shadow
  color `#000`, offset (0, 8), opacity 0.15, radius 32; Android elevation 6
  (unused on iOS path).
- Pill layering, back to front:
  1. Blur layer filling the pill: `AdaptiveBlurView` with `intensity =
     colors.blurIntensity`, `tint = colors.blurTint` (§4.3, §8).
  2. Fill layer: `backgroundColor: colors.glassSurface`.
  3. Hairline border layer: borderWidth 0.5, borderRadius 28,
     top/left border color `colors.glassHighlight`, bottom/right border color
     `colors.glassShadowEdge` (simulates a light edge).
  4. Icon row: horizontal, centered, gap 20.
- Each tab button: 44×44, centered content, hit slop 8, icon size 26.
- Active tab additionally shows a dot under the icon: 5×5, borderRadius 2.5,
  absolute bottom 4, color `colors.primary`.
- Tap behavior: emits tab-press; navigates only if not already focused (standard
  tab semantics) (v1: (tabs)/_layout.tsx:55-64).

### 2.2 Android: standard bottom tab bar

(v1: src/app/app/(tabs)/_layout.tsx:141-170)

Standard Material bottom tab bar **with labels**:

- `tabBarStyle`: backgroundColor `colors.surface`, borderTopColor `colors.border`,
  elevation 8.
- Label: fontSize 12, fontWeight '600'.
- Icon: platform default size, outline/filled per table above.

### 2.3 Desktop wide layout — Dropped in v2

When running on desktop with window width ≥ 700, v1 replaced the tab bar with a left
sidebar (`DesktopSidebar`: app name, collapsible 180/48 px, nav items, Velopack update
hint, version footer) (v1: src/app/app/(tabs)/_layout.tsx:172-184,
src/app/src-rn/components/DesktopSidebar.tsx, src/app/src-rn/hooks/useDesktopLayout.ts).
All of this, including `isWideLayout` branches inside screens, `DateListPanel`,
`OrdersPanel`, `BillingFiltersPanel`, and `DesktopContentWrapper` (a max-width centering
wrapper that is a pass-through on mobile, v1:
src/app/src-rn/components/DesktopContentWrapper.tsx:14-16), is **dropped in v2**. The
mobile behavior documented below is the `isWideLayout === false` path.

---

## 3. Screens

Common to all four tab screens: root container has `flex: 1`, background
`colors.background`, and `paddingTop = safeAreaTopInset` (screens sit under the
status bar; there is no nav header).

### 3.1 Menus tab (`index`, title "Menüs")

(v1: src/app/app/(tabs)/index.tsx)

Primary screen. Shows one day's menu, grouped by category, with order/cancel selection.
Feature logic (pending-order keys, submit pipeline, caching) is owned by
03-features/menus; this section covers presentation and interaction.

**Auth gating** (v1: index.tsx:268-283):

- Gourmet auth status `idle` or `loading` → centered `LoadingOverlay` (§4.2).
- Status `error` or `no_credentials` → centered text
  `Nicht angemeldet` (fontSize 18, weight '600', color `colors.error`) with hint below
  `Gehe zu Einstellungen, um Zugangsdaten einzugeben` (fontSize 14,
  `colors.textTertiary`, marginTop 8).
- Status `authenticated` → main content.

**Refresh triggers** (v1: index.tsx:87-138): on every screen focus, track
`screen.viewed {screen: 'menus'}` (focus only — the analytics event fires before the auth
check, index.tsx:128) and run the refresh pipeline. Additionally, whenever auth status
becomes `authenticated` (index.tsx:134-138), run the refresh pipeline **with no analytics
event on that path**. The pipeline: load cached menus+orders for instant display, then
refresh from network (availability-only refresh when a cache exists, full fetch
otherwise), then run new-menu toast detection (§4.8, 03-features/notifications-new-menu),
then fetch orders. Details in 03-features/menus.

**Layout, top to bottom:**

1. `NewMenuToast` overlay (§4.8) when new menus were detected this refresh.
2. `DayNavigator` (§4.4) — only when at least one menu date exists.
3. Swipeable content area (all remaining vertical space):
   - Refresh banner — shown while `refreshing && !orderProgress`: row, centered,
     gap 8, paddingVertical 8, marginHorizontal 16, marginTop 8, `tintedBanner`
     style (§7) with background `colors.glassPrimary`; contains a small spinner
     (`colors.primary`) and text `Aktualisiere...` (fontSize 12, `colors.primary`,
     weight '500') (v1: index.tsx:287-292, 442-456).
   - Order-progress banner — shown while an order operation runs: row, centered,
     gap 8, paddingVertical 10, marginHorizontal 16, marginTop 8, `buttonPrimary`
     style (§7); small white spinner + white text (fontSize 13, weight '600') with
     the phase label (v1: index.tsx:294-301, 40-45):

     | progress phase | label |
     |---|---|
     | adding | `Wird in den Warenkorb gelegt...` |
     | confirming | `Bestellung wird bestätigt...` |
     | cancelling | `Bestellung wird storniert...` |
     | refreshing | `Menüs werden aktualisiert...` |

   - `LoadingOverlay` (§4.2) while `loading && !orderProgress` (initial load).
   - Error banner — when the store has an error message: padding 12,
     marginHorizontal 16, marginTop 8, `tintedBanner` with `colors.glassError`
     background; message text fontSize 13, color `colors.errorText`
     (v1: index.tsx:305-309, 486-495).
   - Scrollable menu list (content padding 16, paddingBottom 100): items of the
     selected day grouped by category in fixed order `MENÜ I`, `MENÜ II`, `MENÜ III`,
     `SUPPE & SALAT`, `UNKNOWN`; empty groups are omitted (v1: index.tsx:47-53,
     171-174). Each group: marginBottom 16; a category title (fontSize 22,
     weight '600', `colors.primary`, letterSpacing 0.5, marginBottom 8,
     paddingLeft 4) — **suppressed for the `SUPPE & SALAT` group** (v1:
     index.tsx:317-319); then one `MenuCard` (§4.5) per item. An item renders as
     "ordered" if its own `ordered` flag is set OR its category is in the ordered-set
     for that day (union of ordered menu items' categories and fetched orders whose
     date matches the selected day and whose title equals the category)
     (v1: index.tsx:157-169, 321).
   - Empty state (only when not loading): centered `Keine Menüs verfügbar`
     (fontSize 16, `colors.textTertiary`) (v1: index.tsx:335-341).
4. Floating action button — visible when `pendingCount > 0 && !orderProgress`:
   absolute, right 24, bottom **80 on iOS / 24 on Android** (clears the floating
   pill tab bar on iOS), paddingHorizontal 24, paddingVertical 14, `fab` style (§7);
   label white, fontSize 16, weight '700' (v1: index.tsx:391-395, 496-516).
   Label logic (v1: index.tsx:146-154), where `newOrderCount = pendingCount −
   cancellationCount`:
   - cancellations AND new orders pending → `Änderungen bestätigen ({pendingCount})`
   - only cancellations → `Stornieren ({cancellationCount})`
   - otherwise → `Bestellen ({newOrderCount})`
   Tapping submits all pending changes (03-features/menus).

**Swipe-between-days gesture** (v1: index.tsx:192-266, 384-389): the content area
(everything below the DayNavigator) is horizontally draggable:

- Gesture claims: horizontal movement dominates (`|dx| > |dy|`) and `|dx| > 10`.
- While dragging, content translates by dx; at the first/last date the translation is
  multiplied by 0.3 (rubber-band resistance).
- On release with `dx > 50` and a previous day exists: animate content off-screen right
  (180 ms), switch to previous day, place content off-screen left, spring back to 0
  (tension 65, friction 11). Mirror-image for `dx < −50` → next day.
- Below the 50 px threshold: spring back to 0 (default spring).

### 3.2 Orders tab (`orders`, title "Bestellungen")

(v1: src/app/app/(tabs)/orders.tsx)

**Auth gating**: any status except `authenticated` → centered `Anmeldung erforderlich`
(fontSize 16, `colors.textTertiary`) (v1: orders.tsx:94-100).

**Refresh**: on every focus: track `screen.viewed {screen: 'orders'}`; if
authenticated, load cached orders then fetch orders AND menus from network (menus are
needed for description lookup below) (v1: orders.tsx:52-63).

**Layout, top to bottom** (mobile path, v1: orders.tsx:154-224):

1. Segment tabs row (background `colors.glassSurface` on iOS / `colors.surface` on
   Android; borderBottom 0.5 `colors.glassHighlight` on iOS / 1 `colors.border` on
   Android): two equal-width tabs `Kommende ({upcomingCount})` and
   `Vergangene ({pastCount})`; each paddingVertical 14, centered, 3 px bottom border —
   transparent when inactive, `colors.primary` when active; text fontSize 14,
   weight '600', `colors.textTertiary` inactive / `colors.primary` active
   (v1: orders.tsx:157-174, 240-262). Default tab: `upcoming`.
2. Confirm banner — only when `unconfirmedCount > 0` AND active tab is `upcoming`:
   row, space-between, padding 12, marginHorizontal 16, marginTop 8, `tintedBanner`
   with `colors.glassWarning`; left text
   `{n} unbestätigte Bestellung` + plural suffix `en` when n > 1 (i.e.
   `1 unbestätigte Bestellung`, `2 unbestätigte Bestellungen`; color
   `colors.warningText`, fontSize 14, weight '600'); right button `Bestätigen`
   (`buttonPrimary` style, paddingHorizontal 20, paddingVertical 8, white text
   weight '700' fontSize 14, disabled while loading) which confirms all unconfirmed
   orders (v1: orders.tsx:176-189, 264-288).
3. `LoadingOverlay` while loading (v1: orders.tsx:191).
4. Error banner — identical recipe to Menus (padding 12, marginHorizontal 16,
   marginTop 8, `tintedBanner` glassError, text fontSize 13 `colors.errorText`)
   (v1: orders.tsx:193-197).
5. Scrollable list (padding 16, paddingBottom 100) of `OrderItem` cards (§4.6) for the
   active tab's orders. Empty state (only when not loading), centered:
   `Keine kommenden Bestellungen` on the upcoming tab /
   `Keine vergangenen Bestellungen` on the past tab (fontSize 16,
   `colors.textTertiary`) (v1: orders.tsx:212-220).

**Menu description lookup** (v1: orders.tsx:71-80): a map from
`"{order-date-as-day-key}|{order.title}"` to the menu item's `subtitle` (built from all
fetched menu items that have a subtitle; v1 uses `Date.toDateString()` as the day key —
any same-calendar-day key works). Passed to `OrderItem` so the card can show the actual
dish name for a category title like "MENÜ I".

**Cancel flow** (v1: orders.tsx:82-92): tapping an order's ✕ opens a confirm dialog
(§4.1): title `Bestellung stornieren`, message `"{order.title}" stornieren?` (the title
in typographic quotes exactly as: `"` + title + `" stornieren?`), confirm label
`Stornieren` (destructive), cancel label `Behalten`. On confirm, the order is cancelled
(03-features/orders). While one cancellation is in flight, all other cancel buttons are
hidden (`canCancel` requires `cancellingId === null`) and the affected card dims
(v1: orders.tsx:135, 208-209; §4.6).

### 3.3 Billing tab (`billing`, title "Abrechnung")

(v1: src/app/app/(tabs)/billing.tsx)

**Auth gating**: shows content if EITHER Gourmet or Ventopay is authenticated;
otherwise centered `Anmeldung erforderlich` (fontSize 16, `colors.textTertiary`)
(v1: billing.tsx:108, 122-128).

**Refresh**: on every focus: track `screen.viewed {screen: 'billing'}`; if any auth,
load cached months, then fetch Gourmet billing if Gourmet is authenticated, and
Ventopay billing if Ventopay is authenticated (v1: billing.tsx:110-120).

**Data shown**: a single unified list combining Gourmet bills and Ventopay transactions
of the selected month, filtered by source, sorted by date **descending** (newest
first). List keys: `g-{billNr}` for Gourmet, `v-{id}` for Ventopay
(v1: billing.tsx:63-85, 267-271). Month options and totals come from the billing store
(03-features/billing).

**Currency formatting** (used here and in `BillCard`): German-Austria locale currency
formatting of a number as EUR — `value.toLocaleString('de-AT', { style: 'currency',
currency: 'EUR' })` (e.g. `€ 12,34`) (v1: billing.tsx:30-32).

**Layout, top to bottom** (mobile path, v1: billing.tsx:188-284):

1. Month selector row (same container recipe as the Orders segment tabs: glassSurface/
   surface background + hairline bottom border): one equal-width tab per month option
   (store provides 3: current month and the two before, labeled in German, see
   03-features/billing), paddingVertical 14, 3 px bottom border (transparent /
   `colors.primary` when selected), label fontSize 13 weight '600',
   `colors.textTertiary` / `colors.primary` selected, single line
   (v1: billing.tsx:191-209, 299-322).
2. Source filter chips row (paddingHorizontal 16, paddingVertical 8, gap 8), options
   in order: `Alle` (all), `Kantine` (gourmet), `Automaten` (ventopay)
   (v1: billing.tsx:24-28). Chip: paddingHorizontal 14, paddingVertical 6,
   borderRadius 20, background `colors.glassSurface` (iOS) / `colors.surface`
   (Android), borderWidth 1, borderColor `colors.glassHighlight` (iOS) /
   `colors.border` (Android); selected chip: background `colors.glassPrimary` (iOS) /
   `colors.primarySurface` (Android), borderColor `colors.primary`; text fontSize 12
   weight '600', tertiary / primary when selected (v1: billing.tsx:212-229, 323-348).
3. Error banner — same recipe as Menus (v1: billing.tsx:231-235).
4. While `loading` and no entries yet: centered large spinner in `colors.primary`
   (v1: billing.tsx:237-241).
5. When entries exist:
   - Summary bar: row, space-around, paddingVertical 12, paddingHorizontal 16,
     marginHorizontal 16, marginTop 4, `bannerSurface` style (§7). Columns
     (label fontSize 11 `colors.textTertiary` weight '600'; value fontSize 18
     weight '700' `colors.textPrimary`):
     `Gesamt` = formatted total of visible sources; `Belege` = entry count;
     `Zuschuss` = formatted Gourmet subsidy total, value colored `colors.success` —
     **column only rendered when subsidy > 0** (and Gourmet source not filtered out)
     (v1: billing.tsx:88-106, 246-263).
   - Scrollable list (padding 16, paddingBottom 100) of `BillCard`s (§4.7).
6. Empty state (not loading, no entries): centered
   `Keine Abrechnungsdaten für diesen Monat` (fontSize 16, `colors.textTertiary`)
   (v1: billing.tsx:278-282).

Totals respect the source filter: filtering to `Kantine` zeroes the Ventopay total and
vice versa; `Zuschuss` is zero when `Automaten` is selected (v1: billing.tsx:88-106).

### 3.4 Settings tab (`settings`, title "Einstellungen")

(v1: src/app/app/(tabs)/settings.tsx)

A scrollable list (content padding 20, paddingBottom 100). On focus: track
`screen.viewed {screen: 'settings'}`. Mobile renders rows separated by 1 px dividers
(`colors.glassHighlight` iOS / `colors.border` Android, marginVertical 24); each
navigation row is a pressable with title (fontSize 18, weight '600',
`colors.textPrimary`), a hint line below (fontSize 13, `colors.textTertiary`,
marginTop 2), and a trailing `chevron-forward` Ionicon (size 20,
`colors.textTertiary`); row paddingVertical 8 (v1: settings.tsx:56-115, 207-226).

Rows in mobile order (v1: settings.tsx:166-174):

1. **Kantine-Zugangsdaten** → pushes `/kantine-login`. Hint: `Angemeldet als
   {gourmetUsername}` when Gourmet status is `authenticated`, else `Nicht angemeldet`
   (v1: settings.tsx:56-70).
2. **Automaten-Zugangsdaten** → pushes `/automaten-login`. Hint: `Sitzung aktiv` when
   Ventopay status is `authenticated`, else `Nicht angemeldet`
   (v1: settings.tsx:72-87).
3. **Darstellung** → pushes `/appearance`. Hint: current theme preference label —
   `System`, `Hell`, or `Dunkel` (v1: settings.tsx:29-33, 89-102).
4. **Benachrichtigungen** → pushes `/notifications`. Hint:
   `Erinnerungen und Standort-Benachrichtigungen`. Rendered on native only
   (always in v2) (v1: settings.tsx:104-115).
5. **Updates** — desktop-only Velopack update card (`Version {v} ist bereit zur
   Installation.` + `Jetzt aktualisieren` + hint). **Dropped in v2**
   (v1: settings.tsx:117-136).
6. **Datenschutz** link — centered text link (fontSize 14, `colors.textTertiary`,
   paddingVertical 16). Tapping opens an alert dialog (§4.1), title `Datenschutz`,
   message exactly:
   `Diese App erfasst anonyme Nutzungsstatistiken zur Verbesserung der Benutzererfahrung. Die Analyse erfolgt über TelemetryDeck — einen datenschutzfreundlichen, cookielosen Dienst. Es werden keine persönlichen Daten, Passwörter, Menüauswahl oder Abrechnungsdaten erfasst.`
   (v1: settings.tsx:138-149).

### 3.5 Kantine login screen (`kantine-login`)

(v1: src/app/app/kantine-login.tsx)

Scrollable form (content padding 20, paddingBottom 100) inside a keyboard-avoiding
container (iOS `padding` behavior; on Android the system default resize handles it)
(v1: kantine-login.tsx:77). Taps outside inputs keep the keyboard usable
(`keyboardShouldPersistTaps="handled"`).

Layout, top to bottom:

1. Back affordance: row with `chevron-back` Ionicon (size 24, `colors.primary`) +
   text `Einstellungen` (fontSize 17, `colors.primary`); navigates back
   (v1: kantine-login.tsx:83-86).
2. Page title `Kantine-Zugangsdaten` (fontSize 28, weight '700',
   `colors.textPrimary`, marginBottom 24).
3. Input group `Benutzername`: label (fontSize 13, weight '600',
   `colors.textSecondary`, marginBottom 6) + text field (placeholder
   `Benutzername eingeben`, placeholder color `colors.textTertiary`, no
   auto-capitalize, no autocorrect, `inputField` style §7, text fontSize 15
   `colors.textPrimary`).
4. Input group `Passwort`: same, placeholder `Passwort eingeben`, secure entry.
5. Primary button `Speichern` (`buttonPrimary` style, paddingVertical 14,
   borderRadius 14, centered white text weight '700' fontSize 15). While the save/login
   round-trip is in flight the button is disabled and its label reads `Speichern...`
   (v1: kantine-login.tsx:117-125).
6. When Gourmet status is `authenticated`: a session section (marginTop 16) with text
   `Angemeldet als: {userInfo.username}` (fontSize 14, `colors.textSecondary`,
   marginBottom 12) and a danger button `Abmelden` (`buttonDanger` style,
   paddingVertical 14, paddingHorizontal 24, white text weight '700' fontSize 15)
   that logs out (v1: kantine-login.tsx:127-136).

Behavior:

- On mount, saved credentials (if any) pre-fill both fields
  (v1: kantine-login.tsx:45-53).
- Save tap with an empty username or password → alert `Fehler` /
  `Bitte Benutzername und Passwort eingeben`; nothing else happens
  (v1: kantine-login.tsx:56-59).
- Otherwise: **credentials are saved to secure storage first, then login is
  attempted** (i.e. bad credentials still overwrite the stored ones)
  (v1: kantine-login.tsx:61-62).
  - Login success → alert `Gespeichert` / `Kantine-Zugangsdaten sicher gespeichert`.
  - Login failure → alert `Login fehlgeschlagen` / the store's error message, or
    fallback `Anmeldung nicht möglich. Bitte Zugangsdaten prüfen.`
    (v1: kantine-login.tsx:64-69).

### 3.6 Automaten login screen (`automaten-login`)

(v1: src/app/app/automaten-login.tsx)

Identical structure and behavior to §3.5 with these differences:

- Page title `Automaten-Zugangsdaten`, followed by a subtitle
  `Für Automaten und Kassenabrechnungen` (fontSize 14, `colors.textTertiary`,
  marginBottom 24; title marginBottom 4) (v1: automaten-login.tsx:87-88).
- Success alert message: `Automaten-Zugangsdaten sicher gespeichert`
  (v1: automaten-login.tsx:64).
- Authenticated session text: `Automaten-Sitzung aktiv` (no username shown — the
  Ventopay store keeps no user info) (v1: automaten-login.tsx:129).
- Backed by the Ventopay auth store; same save-then-login ordering, same empty-field
  and failure alerts (v1: automaten-login.tsx:54-69).

### 3.7 Appearance screen (`appearance`)

(v1: src/app/app/appearance.tsx)

Scrollable (content padding 20, paddingBottom 100). Back affordance + page title
`Darstellung` as in §3.5. Two cards (`cardSurface` style §7, padding 20,
marginBottom 16; section title fontSize 18, weight '600', `colors.textPrimary`,
marginBottom 14):

1. **`Design`** — three equal-width options in a row (gap 10):
   `System` (icon `phone-portrait-outline`), `Hell` (`sunny-outline`),
   `Dunkel` (`moon-outline`) (v1: appearance.tsx:18-22). Each option:
   `bannerSurface` style, paddingVertical 16, centered; icon size 22 above the label
   (marginBottom 6), icon color `colors.primary` when selected else
   `colors.textSecondary`; label fontSize 14 weight '600', `colors.textSecondary` /
   `colors.primary` when selected. Selected option background:
   `colors.glassPrimary` (iOS) / `colors.primarySurface` (Android), bottom border
   `colors.primary` (Android also all borders `colors.primary`)
   (v1: appearance.tsx:48-77, 156-177). Tapping sets the theme preference
   (03-features/themes).
2. **`Akzentfarbe`** — five swatches in a row (gap 16), order and labels exactly:
   `Orange`, `Smaragd` (emerald), `Beere` (berry), `Gold` (golden), `Ozean` (ocean)
   (v1: src/app/src-rn/theme/colors.ts:177-253; iteration order = object key order
   orange, emerald, berry, golden, ocean, v1: appearance.tsx:24, 82). Each swatch:
   40×40 circle filled with the accent's **light-mode** primary color (even in dark
   mode; v1: appearance.tsx:91), borderWidth 2 transparent; when selected: borderWidth
   3 in the same accent color and a white `checkmark` Ionicon (size 20) centered.
   Label below (fontSize 12, `colors.textTertiary`, weight '500'; selected:
   `colors.primary`). Tapping sets the accent color (03-features/themes, which also
   covers the accent → alternate-app-icon coupling).

### 3.8 Notifications screen (`notifications`)

(v1: src/app/app/notifications.tsx)

Scrollable (content padding 20, paddingBottom 100). Back affordance + page title
`Benachrichtigungen` as in §3.5. Three sections separated by dividers (1 px,
`colors.glassHighlight` iOS / `colors.border` Android, marginVertical 24). Section
titles: fontSize 18 weight '600' `colors.textPrimary`, marginBottom 4; section hints:
fontSize 13 `colors.textTertiary`, marginBottom 16. Feature semantics live in
03-features/notifications-daily-reminder, notifications-location, notification-log;
below is the UI contract.

**Section 1 — `Bestell-Erinnerung`** (hint: `Tägliche Erinnerung an deine Bestellung`):

- Row `Aktiviert` (label fontSize 15 weight '600' `colors.textSecondary`) with a
  switch (track color `colors.border` off / `colors.primary` on).
- Toggling ON: request notification permission; if denied → alert
  `Berechtigung fehlt` / `Benachrichtigungen werden für diese Funktion benötigt. Bitte
  in den Einstellungen aktivieren.` and the switch stays off. If granted: persist the
  currently shown time, register background sync, persist enabled=true. Toggling OFF
  persists enabled=false. Either way an analytics signal
  `notification.reminderToggled` is sent (v1: notifications.tsx:116-133).
- When enabled, a `Uhrzeit` label and a horizontal, scrollbar-less chip row appear
  with the time options **11:00 through 13:45 in 15-minute steps** (12 chips; label
  `HH:MM` zero-padded; generated by looping hour 11–13 × minute 0/15/30/45, v1:
  notifications.tsx:51-60). Chip: paddingHorizontal 14, paddingVertical 10,
  `bannerSurface` style; selected: background `colors.glassPrimary` (iOS) /
  `colors.primarySurface` + primary border (Android), bottom border `colors.primary`;
  text fontSize 14 weight '600', secondary / primary selected. Tapping persists the
  time immediately (v1: notifications.tsx:135-139, 238-261).
- Android only: below the chips, a pressable hint banner (row, gap 8, marginTop 12,
  padding 12, borderRadius 10, background `colors.surfaceVariant`) with an
  `information-circle-outline` icon (18, `colors.textTertiary`) and text
  `Damit Erinnerungen zuverlässig funktionieren, muss die Hintergrundaktivität für
  diese App erlaubt sein. ` followed by the inline link-styled text
  `App-Einstellungen öffnen` (`colors.primary`, weight '600'); tapping anywhere opens
  the system app settings (v1: notifications.tsx:262-270, 491-509).

**Section 2 — `Standort-Benachrichtigungen`** (hint:
`Erinnerung um 8:45 basierend auf deinem Standort`):

- No company location set → primary button
  `Aktuellen Standort als Firmenstandort setzen`; while resolving it is disabled and
  reads `Standort wird ermittelt...` (v1: notifications.tsx:292-301).
  Tap flow (v1: notifications.tsx:145-177):
  1. Request location permissions. If not fully granted and background permission is
     missing → alert titled `Standort „Immer" erforderlich` with message
     `Für Standort-Benachrichtigungen muss der Standortzugriff auf „Immer" gesetzt
     werden.\n\nBitte öffne die Einstellungen und wähle unter Standort „Immer" aus.`
     then open the system app settings; abort.
  2. Request notification permission; if denied → alert `Berechtigung fehlt` / same
     message as in section 1; abort.
  3. Read current position, store it as company location, enable notifications, then
     alert `Gespeichert` / `Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt,
     wenn du im Büro bist und nicht bestellt hast.` and send `notification.locationSet`.
  4. Any thrown error → alert `Fehler` / `Standort konnte nicht ermittelt werden.`
- Company location set → status text `Firmenstandort gesetzt` (fontSize 14,
  `colors.textSecondary`) + danger button `Standort entfernen` which clears the
  location and disables notifications (v1: notifications.tsx:282-290, 179-182).

**Section 3 — `Benachrichtigungs-Log`** (hint: `Zeichnet 24 Stunden lang
Diagnose-Daten auf, um Probleme mit Benachrichtigungen zu analysieren.`):

Three mutually exclusive states derived from the stored activation expiry
(v1: notifications.tsx:83-84, 311-355):

- **Inactive** (never activated / cleared): two equal-width primary buttons in a row
  (gap 12): `12 Stunden` and `24 Stunden`; tapping activates the log for that many
  hours (v1: notifications.tsx:340-355).
- **Active** (now < expiry): status text `Aufzeichnung läuft bis {expiry formatted
  de-AT dd.MM., HH:mm}` plus ` ({n} Einträge)` when n > 0
  (v1: notifications.tsx:311-323).
- **Expired** (now ≥ expiry): status text `Aufzeichnung abgeschlossen ({n} Einträge).`
  followed by a primary button with a `mail-outline` icon (16, white) and label
  `Log per E-Mail senden`, and below it a borderless text button `Log verwerfen`
  (fontSize 14, `colors.textTertiary`) that clears the log
  (v1: notifications.tsx:324-339).
- Send-by-email opens the system mail composer with recipient `aiko@spitzbub.app`,
  subject `SnackPilot Notification Log (bis {expiry, de-AT locale string})`, body =
  formatted log (03-features/notification-log). Failure to open →
  alert `Fehler` / `E-Mail-App konnte nicht geöffnet werden.`
  (v1: notifications.tsx:189-203).
- On mount (native), reminder state and log state are loaded; log state also reloads
  on every screen focus (the recording may have expired while away)
  (v1: notifications.tsx:95-114).

---

## 4. Shared components

### 4.1 Dialog system (`DialogProvider`)

(v1: src/app/src-rn/components/DialogProvider.tsx)

v1 deliberately does NOT use the platform alert; it renders a custom themed modal so
dialogs match the app theme on all platforms. v2 MAY substitute native alert dialogs
(UIAlertController / Material dialog) as long as the three call shapes and all texts
are preserved; the v1 visual spec follows for a faithful port.

API (promise-based; a single dialog can be visible at a time):

- `showDialog({title, message?, buttons})` → resolves the tapped button's index.
- `alert(title, message?)` → single `OK` button.
- `confirm(title, message, confirmLabel = 'Bestätigen', cancelLabel = 'Abbrechen')`
  → renders `[cancel, confirm(destructive)]` and resolves `true` iff confirm tapped
  (v1: DialogProvider.tsx:56-76).

Visuals/behavior (v1: DialogProvider.tsx:83-193):

- Fade-in transparent modal. Backdrop: `colors.overlay`, content centered,
  padding 40. Tapping the backdrop, or the system back gesture/button, dismisses and
  resolves index 0 (for `confirm` that means "cancelled"); tapping the dialog body
  does not dismiss.
- Dialog box: background `colors.surface`, borderRadius 14, padding 24, maxWidth 340,
  width 100%, border 0.5 `colors.border`.
- Title: fontSize 17, weight '600', `colors.textPrimary`, centered. Message (optional):
  fontSize 14, `colors.textSecondary`, centered, marginTop 8, lineHeight 20.
- Button row (marginTop 20, gap 10): buttons flex equally; paddingVertical 10,
  borderRadius 8. Default style: background `colors.primary`, white text fontSize 15
  weight '600'. `destructive`: background `colors.error`, white text. `cancel`:
  transparent background, 1 px `colors.border` border, text `colors.textSecondary`.
  A single-button dialog centers the button with paddingHorizontal 32 instead of
  stretching.

### 4.2 Loading overlay (`LoadingOverlay`)

(v1: src/app/src-rn/components/LoadingOverlay.tsx)

Fills its parent absolutely (zIndex 10) and centers a spinner card:

- iOS: an `AdaptiveBlurView` layer with `intensity = colors.blurIntensityStrong`,
  `tint = colors.blurTint` blurs the underlying content.
- Android (flat style): an opaque-ish layer with background `colors.overlay` instead
  of blur.
- Spinner card: borderRadius 24, padding 28, `bannerSurface` style (§7),
  elevation 6, containing a large activity indicator in `colors.primary`.

### 4.3 Blur wrapper (`AdaptiveBlurView`)

(v1: src/app/src-rn/components/AdaptiveBlurView.tsx)

On native this is a direct pass-through to the system blur view (expo-blur BlurView;
v2: UIVisualEffectView on iOS) receiving `intensity` and `tint`. Tint names used:
`systemThinMaterial` (light theme) and `systemThickMaterialDark` (dark theme) —
these are iOS material styles (§8). Android never renders blur (all call sites are
behind iOS-only branches or `useFlatStyle`).
**Dropped in v2:** the web variant that approximated blur with CSS
`backdrop-filter: blur(intensity * 0.4 px)`
(v1: src/app/src-rn/components/AdaptiveBlurView.web.tsx).

### 4.4 Day navigator (`DayNavigator`)

(v1: src/app/src-rn/components/DayNavigator.tsx)

Header bar on the Menus tab. Row (paddingHorizontal 16, paddingVertical 12,
space-between):

- Left/right arrow buttons: 48×48 circles (`circleButton` style §7), glyphs `❮`
  (U+276E) and `❯` (U+276F) at fontSize 20 in `colors.textPrimary`. Disabled (and at
  opacity 0.3) at the respective end of the date list.
- Center (pressable): selected date formatted via de-AT locale with short weekday,
  short month, numeric day — e.g. `Mo., 10. Feb.` (v1:
  src/app/src-rn/utils/dateUtils.ts:32-38) at fontSize 19 weight '600'
  `colors.textPrimary`; below it `"{index+1} / {count}"` (fontSize 12,
  `colors.textTertiary`, marginTop 2).
- Tapping the center jumps to "today": the nearest date ≥ today from the list, falling
  back to the latest past date (v1: DayNavigator.tsx:33-38,
  src/app/src-rn/utils/dateUtils.ts:119-148).
- Container: iOS — blur wrapper (`intensity = colors.blurIntensity`,
  `tint = colors.blurTint`) with 0.5 px bottom border `colors.glassShadowEdge`;
  Android — solid `colors.surface`, 1 px bottom border `colors.border`, elevation 2
  (v1: DayNavigator.tsx:69-81).

### 4.5 Menu card (`MenuCard`)

(v1: src/app/src-rn/components/MenuCard.tsx)

One card per menu item. Base: `cardSurface` style (§7), padding 16, marginBottom 10.

Inputs: the item, `isSelected` (pending order), `ordered`, `isPendingCancel`
(marked for cancellation), and an `onToggle` callback.

**Interactivity rule** (v1: MenuCard.tsx:20-25): the card is tappable iff
`ordered || (item.available && !orderingCutoff(item.day))`. Ordering cutoff: menu day
is in the past, or is today and current Vienna time (Europe/Vienna) ≥ 09:00
(v1: src/app/src-rn/utils/dateUtils.ts:94-99). Tapping toggles selection (selects
for ordering, or marks/unmarks an ordered item for cancellation —
03-features/menus).

**Visual states** (applied in this order, later wins; v1: MenuCard.tsx:29-35):

| state | style delta |
|---|---|
| ordered && !pendingCancel | background `colors.glassSuccess` (iOS) / `colors.successSurface` + border `colors.successBorder` (Android) |
| pendingCancel | opacity 0.55, dashed border |
| selected | background `colors.primary`; iOS: borderTop `rgba(255,255,255,0.30)`, borderLeft `rgba(255,255,255,0.15)`, borderBottom `colors.primaryDark`; Android: all borders `colors.primaryDark` |
| (!available \|\| cutoff) && !ordered | opacity 0.5 (and not tappable) |

**Content, top to bottom:**

1. Badge row (gap 6, marginBottom 2), each badge paddingHorizontal 8,
   paddingVertical 2, borderRadius 12, border 0.5 (iOS) / 1 (Android), text
   fontSize 10 weight '700'. At most one badge shows; conditions in order:
   - `isPendingCancel` → `Wird storniert` (warning colors: bg `colors.glassWarning`
     iOS / `colors.warningSurface` Android, border `colors.warning`, text
     `colors.warningText`).
   - `ordered && !isPendingCancel` → `Bestellt` (success colors).
   - `!ordered && !item.available` → `Ausverkauft` (error colors).
   - `cutoff && !ordered && item.available` → `Geschlossen` (warning colors).
2. Subtitle (the dish text): fontSize 16, lineHeight 21, `colors.textPrimary`, max
   4 lines. When selected: white. When pendingCancel: line-through +
   `colors.textTertiary`.
3. Bottom row (marginTop 6, space-between): allergens text
   `Allergene: {letters joined by ", "}` (empty string when none; fontSize 11,
   `colors.textTertiary`, 1 line) and price string as-is (fontSize 13, weight '600',
   `colors.textSecondary`). Both take the selected/pendingCancel text overrides.
4. When selected: a checkmark disc overlays the top-right corner (absolute top 8,
   right 8): 24×24 white circle containing `✓` (U+2713) in `colors.primary`,
   fontSize 14 weight '700'.

### 4.6 Order item (`OrderItem`)

(v1: src/app/src-rn/components/OrderItem.tsx)

Card row (`cardSurface`, padding 16, marginBottom 8, space-between; while its
cancellation is in flight: opacity 0.6).

Left column (flex 1, marginRight 12):

1. Date: `formatDisplayDate(order.date)` (§4.4 format), fontSize 12,
   `colors.textTertiary`, weight '600'.
2. Category label: `order.title` (e.g. `MENÜ I`), fontSize 11, weight '700',
   `colors.primary`, letterSpacing 0.5.
3. Description (max 2 lines, fontSize 16, `colors.textPrimary`): first non-empty of
   `menuDescription` (looked up from menu data, §3.2) → `order.subtitle` →
   `order.title` (v1: OrderItem.tsx:39-41).

Right column (aligned right, gap 8):

1. Status badge (paddingHorizontal 10, paddingVertical 4, borderRadius 12, border
   0.5 iOS / 1 Android, text fontSize 11 weight '600'): `Bestätigt` in success colors
   when `order.approved`, else `Ausstehend` in warning colors.
2. Cancel affordance — rendered only when `canCancel` (upcoming tab and no other
   cancellation in flight):
   - normally a 32×32 circular button (background `colors.glassError` iOS /
     `colors.errorSurface` Android, border `colors.error`, hit slop 8) containing
     `✕` (U+2715) fontSize 16 weight '700' in `colors.error`;
   - if this order is being cancelled, a small spinner in `colors.error` instead;
   - **cancellation cutoff**: same rule as ordering cutoff (past day, or today ≥
     09:00 Europe/Vienna) disables the button and greys it (opacity 0.4, border
     `colors.textTertiary`, background `colors.glassSurfaceVariant` iOS /
     `colors.surfaceVariant` Android, glyph `colors.textTertiary`)
     (v1: OrderItem.tsx:21-32, 58-64; src/app/src-rn/utils/dateUtils.ts:107-112).
     While not yet cut off, the component re-evaluates the cutoff every 30 seconds so
     the button locks at 09:00 without a re-render from outside
     (v1: OrderItem.tsx:23-30).

### 4.7 Bill card (`BillCard`)

(v1: src/app/src-rn/components/BillCard.tsx)

Card (`cardSurface`, padding 16, marginBottom 8) with two variants sharing a header:

Header (row, space-between, marginBottom 12): left — date line
(de-AT locale, short weekday + numeric day + short month + numeric year, e.g.
`Mo., 10. Feb. 2026`; fontSize 14, weight '600', `colors.textPrimary`) with the time
below (de-AT `HH:mm`, fontSize 12, `colors.textTertiary`) (v1: BillCard.tsx:22-36);
right (aligned right, gap 4) — a source badge and the amount (fontSize 18,
weight '700', `colors.primary`, formatted per §3.3 currency rule).

Source badge: paddingHorizontal 8, paddingVertical 2, borderRadius 8; text
fontSize 10, weight '700', `colors.textSecondary`, uppercase, letterSpacing 0.5:

- Gourmet variant: text `Kantine`, background `colors.glassPrimary`.
- Ventopay variant: text `Automaten`, background `colors.glassSuccess`.

Body:

- **Gourmet** (v1: BillCard.tsx:38-69): item rows (gap 6), each row:
  `{count}x` (fontSize 13, `colors.textTertiary`, fixed width 28), the item
  description (flex 1, fontSize 14, `colors.textSecondary`, 1 line), the item total
  (currency-formatted, fontSize 13, `colors.textTertiary`, marginLeft 8).
- **Ventopay** (v1: BillCard.tsx:71-96): a single optional restaurant line
  (fontSize 13, `colors.textSecondary`, 1 line) — omitted when empty. Individual
  transaction items are NOT rendered.

### 4.8 New-menu toast (`NewMenuToast`)

(v1: src/app/src-rn/components/NewMenuToast.tsx)

In-app toast shown by the Menus screen when new menus are detected
(03-features/notifications-new-menu decides when).

- Absolute overlay at `top = safeAreaTopInset + 8`, left 16, right 16, zIndex 100;
  padding 12, centered content, `tintedBanner` style with `colors.glassPrimary`
  background.
- Text: `Neue Menüs verfügbar!` (fontSize 14, weight '600', `colors.primary`).
- Animation: slides in from translateY −100 → 0 while fading 0 → 1 over **300 ms**;
  stays for **4000 ms**; then reverses over 300 ms and invokes dismiss
  (v1: NewMenuToast.tsx:13-54). Not manually dismissible.

### 4.9 Desktop-only components — Dropped in v2

`DesktopSidebar`, `DesktopContentWrapper`, `DateListPanel` (date list side panel),
`OrdersPanel` (orders filter/summary side panel), `BillingFiltersPanel`
(month/source/summary side panel) render only when `isWideLayout` is true, which
requires the desktop platform (v1: src/app/src-rn/hooks/useDesktopLayout.ts:24).
All dropped; their summary/totals information is fully covered by the mobile layouts
above. `AnalyticsProvider` is documented in 03-features/analytics (it renders no UI;
only the "release builds only" wrapping in §1.1 is UI-relevant).

---

## 5. Loading / empty / error state summary

| Screen | Loading | Empty | Error | Unauthenticated |
|---|---|---|---|---|
| Menus | LoadingOverlay (initial); refresh banner `Aktualisiere...` (background refresh); progress banner (order ops) | `Keine Menüs verfügbar` | inline error banner with store message | `Nicht angemeldet` + `Gehe zu Einstellungen, um Zugangsdaten einzugeben`; LoadingOverlay while auth pending |
| Orders | LoadingOverlay | `Keine kommenden Bestellungen` / `Keine vergangenen Bestellungen` | inline error banner | `Anmeldung erforderlich` |
| Billing | centered large spinner (only when no data yet) | `Keine Abrechnungsdaten für diesen Monat` | inline error banner | `Anmeldung erforderlich` (only when BOTH services are logged out) |
| Settings | — | — | — | rows show `Nicht angemeldet` hints |
| Login screens | button label `Speichern...`, disabled | — | alert dialog `Login fehlgeschlagen` | n/a |
| Notifications | button label `Standort wird ermittelt...` | — | alert dialogs (§3.8) | n/a |

Cached data is always shown immediately while a network refresh runs in the
background; only a truly empty state shows a full-screen spinner (see
03-features/caching).

---

## 6. Interaction/animation constants

| Constant | Value | Where |
|---|---|---|
| Day-swipe claim threshold | \|dx\| > 10 and \|dx\| > \|dy\| | Menus (v1: index.tsx:209-210) |
| Day-swipe commit threshold | 50 px | Menus (v1: index.tsx:204) |
| Edge rubber-band factor | 0.3 | Menus (v1: index.tsx:217) |
| Swipe-out duration | 180 ms | Menus (v1: index.tsx:228) |
| Swipe-in spring | tension 65, friction 11 | Menus (v1: index.tsx:236-237) |
| Toast display time | 4000 ms | NewMenuToast (v1: NewMenuToast.tsx:13) |
| Toast animation | 300 ms | NewMenuToast (v1: NewMenuToast.tsx:14) |
| Cancel-cutoff re-check interval | 30 000 ms | OrderItem (v1: OrderItem.tsx:27) |
| Ordering/cancellation cutoff | 09:00 Europe/Vienna, same-day only | dateUtils (v1: dateUtils.ts:94-112) |
| Dialog modal animation | fade | DialogProvider (v1: DialogProvider.tsx:86) |
| Tab bar pill (iOS) | 248×56, radius 28 | (tabs)/_layout (v1: :97-107) |

---

## 7. Platform style recipes

(v1: src/app/src-rn/theme/platformStyles.ts)

Two style dialects: **glass** (iOS — translucent surfaces, hairline borders, soft
shadows) and **flat** (Android — opaque surfaces, 1 px borders, elevation). v1
selected them via `useFlatStyle = (Android || desktop)` and compact sizing via
`isCompactDesktop = desktop` (v1: src/app/src-rn/utils/platform.ts:20-23). In v2:
iOS uses the glass recipes, Android the flat/Android recipes; the desktop recipes and
all `isCompactDesktop` size reductions are **dropped** (mobile always uses the larger
of the two values; the mobile values are the ones quoted in §3).

All colors below are theme tokens (§8). "shadow(c, o, r, (x,y))" = shadowColor c,
shadowOpacity o, shadowRadius r, shadowOffset (x,y).

| Recipe | iOS (glass) | Android (flat) |
|---|---|---|
| `cardSurface` | bg `glassSurface`, radius 16, border 0.5 `glassShadowEdge`, shadow(#000, 0.06, 8, (0,2)) | bg `surface`, radius 14, border 1 `border`, elevation 1 |
| `bannerSurface` | bg `glassSurface`, radius 12, border 0.5 `glassShadowEdge`, shadow(#000, 0.04, 4, (0,1)) | bg `surface`, radius 12, border 1 `border`, elevation 1 |
| `tintedBanner(bg)` | bg as given, radius 12, border 0.5 `glassShadowEdge`, shadow(#000, 0.04, 4, (0,1)) | bg as given, radius 12, border 1 `border`, elevation 1 |
| `buttonPrimary` | bg `primary`, radius 14, shadow(`primary`, 0.20, 6, (0,3)) | bg `primary`, radius 12, elevation 2 |
| `buttonSecondary` | bg `glassSurface`, radius 14, border 1 `border`, shadow(#000, 0.04, 4, (0,1)) | bg `surface`, radius 12, border 1 `primary` |
| `buttonDanger` | bg `error`, radius 14 | bg `error`, radius 12, elevation 2 |
| `inputField` | bg `glassSurface`, radius 12, padding 14/12 (h/v), border 0.5 `glassShadowEdge`, shadow(#000, 0.03, 3, (0,1)) | bg `surface`, radius 12, border 1 `borderInput`, padding 14/12 |
| `circleButton` | bg `glassSurfaceVariant`, radius 24, border 0.5 `glassShadowEdge`, shadow(#000, 0.04, 4, (0,1)) | bg `surfaceVariant`, radius 24, elevation 1 |
| `fab` | bg `primary`, radius 28, shadow(`primary`, 0.25, 12, (0,4)), elevation 6 | bg `primary`, radius 16, elevation 4 |
| `sidebarSurface`, `panelSurface` | — | — (desktop-only, **dropped**) |

Screens with iOS/Android inline differences beyond these recipes:

- Segment-tab containers (Orders tabs, Billing month selector): iOS bg
  `glassSurface` + 0.5 border `glassHighlight`; Android bg `surface` + 1 px border
  `border` (v1: orders.tsx:240-244, billing.tsx:299-304).
- Selected chips/options (billing source, appearance options, time chips): iOS bg
  `glassPrimary`; Android bg `primarySurface` (+ primary border)
  (v1: billing.tsx:337-339, appearance.tsx:162-166, notifications.tsx:427-431).
- Badges and small status surfaces use `glass{Success,Warning,Error}` on iOS vs
  `{success,warning,error}Surface` + 1 px border on Android (§4.5–4.6).
- Menus FAB bottom offset: 80 iOS / 24 Android (§3.1).
- Tab bar: §2.1 vs §2.2. DayNavigator wrapper: §4.4. LoadingOverlay: §4.2.
- Divider color: `glassHighlight` iOS / `border` Android (settings, notifications).

Note on v1 color constants: because `useFlatStyle` was a module-level constant, the
`glass*` tokens themselves resolve to opaque colors on Android (see §8) — i.e. on
Android, code paths referencing `glassSurface` get `surface`-like opaque values. v2
implementations that split by platform explicitly get the same result.

## 8. Color tokens

(v1: src/app/src-rn/theme/colors.ts; ownership shared with 03-features/themes, which
covers preference persistence and accent switching. Values restated here because the
layout specs reference tokens.)

Light theme:

| token | value | dark theme value |
|---|---|---|
| background | `#F5F5F7` | `#000000` |
| surface | `#fff` | `#1C1C1E` |
| surfaceVariant | `#EDEDF0` | `#2C2C2E` |
| textPrimary | `#1D1D1F` | `#F5F5F7` |
| textSecondary | `#6E6E73` | `#A1A1A6` |
| textTertiary | `#AEAEB2` | `#636366` |
| primary (default accent "orange") | `#D4501A` | `#FF6B35` |
| primaryDark | `#B84415` | `#D4501A` |
| primarySurface | `#FFF1EB` | `#2A1A10` |
| border | `#D6D6D8` | `#38383A` |
| borderInput | `#CECED0` | `#48484A` |
| success | `#34A853` | `#34A853` |
| successSurface | `#EBF5EE` | `#142018` |
| successText | `#1E7E34` | `#5DB075` |
| successBorder | `#34A853` | `#2E7D32` |
| warning | `#F5A623` | `#F5A623` |
| warningSurface | `#FEF6E6` | `#2A1E0E` |
| warningText | `#B57A10` | `#F5C564` |
| warningBorder | `#F5C564` | `#B57A10` |
| error | `#D93025` | `#EA4335` |
| errorSurface | `#FCE8E6` | `#2A1614` |
| errorText | `#B3261E` | `#F28B82` |
| overlay | `rgba(255,255,255,0.7)` | `rgba(0,0,0,0.7)` |
| glassSurface (iOS) | `rgba(255,255,255,0.70)` | `rgba(28,28,30,0.72)` |
| glassSurfaceVariant (iOS) | `rgba(237,237,240,0.60)` | `rgba(44,44,46,0.60)` |
| glassHighlight (iOS) | `rgba(255,255,255,0.85)` | `rgba(255,255,255,0.12)` |
| glassShadowEdge (iOS) | `rgba(0,0,0,0.06)` | `rgba(0,0,0,0.40)` |
| glassSuccess (iOS) | `rgba(52,168,83,0.10)` | `rgba(52,168,83,0.16)` |
| glassWarning (iOS) | `rgba(245,166,35,0.10)` | `rgba(245,166,35,0.16)` |
| glassError (iOS) | `rgba(217,48,37,0.10)` | `rgba(234,67,53,0.16)` |
| glassPrimary (iOS, orange accent) | `rgba(212,80,26,0.08)` | `rgba(255,107,53,0.14)` |
| blurTint | `systemThinMaterial` | `systemThickMaterialDark` |
| blurIntensity | 40 | 50 |
| blurIntensityStrong | 80 | 90 |
| blurIntensitySubtle | 25 | 30 |

On Android the `glass*` tokens fall back to opaque values:
glassSurface→`#ffffff`/`#1C1C1E`, glassSurfaceVariant→`#EDEDF0`/`#2C2C2E`,
glassHighlight→`#D6D6D8`/`#38383A`, glassShadowEdge→`#D6D6D8`/`#000000`,
glassSuccess→successSurface, glassWarning→warningSurface, glassError→errorSurface,
glassPrimary→primarySurface (v1: colors.ts:95-104, 145-154).

Accent color sets (each overrides `primary`, `primaryDark`, `primarySurface`,
`glassPrimary`; light / dark) (v1: colors.ts:177-253):

| id | label | light primary / dark primary | light primaryDark / dark | light primarySurface / dark | light glassPrimary (iOS) / dark |
|---|---|---|---|---|---|
| orange | `Orange` | `#D4501A` / `#FF6B35` | `#B84415` / `#D4501A` | `#FFF1EB` / `#2A1A10` | `rgba(212,80,26,0.08)` / `rgba(255,107,53,0.14)` |
| emerald | `Smaragd` | `#2E7D4F` / `#4CAF7D` | `#236B3F` / `#2E7D4F` | `#E8F5ED` / `#102A1A` | `rgba(46,125,79,0.08)` / `rgba(76,175,125,0.14)` |
| berry | `Beere` | `#A62547` / `#E04868` | `#8C1E3B` / `#A62547` | `#FCEEF2` / `#2A1018` | `rgba(166,37,71,0.08)` / `rgba(224,72,104,0.14)` |
| golden | `Gold` | `#C08B1A` / `#E8B03E` | `#A07415` / `#C08B1A` | `#FDF5E3` / `#2A2210` | `rgba(192,139,26,0.08)` / `rgba(232,176,62,0.14)` |
| ocean | `Ozean` | `#2563A8` / `#4A90D9` | `#1E528C` / `#2563A8` | `#EBF2FC` / `#101A2A` | `rgba(37,99,168,0.08)` / `rgba(74,144,217,0.14)` |

On Android the accents' `glassPrimary` equals their `primarySurface`
(v1: colors.ts:183-249). Theme resolution (`system`/`light`/`dark` preference →
effective scheme) is specified in 03-features/themes.

---

## 9. Dropped in v2 (summary)

- Desktop wide layout: sidebar, side panels, `isWideLayout` branches, 700 px
  breakpoint, sidebar widths 180/48, panel width 200
  (v1: src/app/src-rn/hooks/useDesktopLayout.ts:6-9).
- `isCompactDesktop` compact sizing variants throughout all stylesheets.
- Desktop style dialect (borderRadius 4 flat recipes) in platformStyles.
- Settings "Updates" card and sidebar update hint (Velopack).
- Web blur fallback (`AdaptiveBlurView.web.tsx`), web color-scheme DOM sync,
  `tauriHttp` import.
- Vestigial `App.tsx`/`index.ts` template entry (dead code even in v1).
