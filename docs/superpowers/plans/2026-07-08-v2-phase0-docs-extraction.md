# SnackPilot v2.0 Phase 0 — Requirements Extraction & Branch Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract every requirement embodied in SnackPilot v1.4.5 into a self-contained `docs/` tree, adversarially verify it against the code, and create the orphan `v2` branch whose first commit contains only those docs plus repo scaffolding.

**Architecture:** Extraction runs as a multi-agent Workflow against the v1 source on the `v2/planning` branch (identical to `main` @ `6997c44` plus planning docs). One extractor agent per target doc writes directly into `docs/requirements/` etc.; synthesis docs (overview, v2-architecture) run after a barrier. A second Workflow adversarially verifies every doc with omission/invention lenses (plus a request-shape auditor for the two scraping docs) and loops fix→re-verify until dry. The orphan branch is assembled in a separate git worktree so the main checkout's untracked files (`.env`, `node_modules`) are never touched.

**Tech Stack:** git (orphan branch, worktree), Claude Workflow tool (extraction + verification), Markdown docs.

## Global Constraints

- v1 baseline is `main` @ `6997c442132db1f3341d6ee6f17e509e454d8e9d` (v1.4.5). Every produced doc carries the provenance header line: `> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.`
- The v2 `README.md` MUST include a credit line linking to `https://github.com/patrickl92/GourmetClient` as the base/original project.
- Scraping docs (`01-`, `02-`) must be precise enough that an implementer **with no access to v1 source** produces byte-identical HTTP requests (method, URL, headers, encoding, field names, field values, order of requests).
- The v2 branch's first commit contains ONLY: `docs/` (requirements, architecture, fixtures, superpowers specs+plans), `README.md`, `CLAUDE.md`, `.gitignore`, `.env.example`. No v1 source.
- Dropped in v2 (docs must mark these, not silently omit them): desktop (Tauri/Velopack), web target, `*.web.ts(x)` variants, `tauriHttp`, `desktopUpdater`, `DesktopSidebar`/`DesktopContentWrapper`, `useDesktopLayout`, `cryptoPolyfill`.
- App identity: iOS bundle ID and Android package are both `dev.radaiko.gourmetclient` and MUST stay unchanged in v2 (store-update path + credential takeover).
- All extraction/verification work commits on `v2/planning`; nothing is pushed until the user confirms.
- Working directory for all commands: `/Users/radaiko/dev/private/SnackPilot` unless stated otherwise.

---

### Task 1: Coverage matrix (acceptance criteria first)

The matrix is written BEFORE extraction — it is the "failing test": every v1 file must be owned by a doc or explicitly marked dropped/not-applicable. Verification (Task 4/5) checks docs against this matrix.

**Files:**
- Create: `docs/requirements/appendix-source-map.md`

**Interfaces:**
- Produces: the authoritative file→doc mapping consumed by Task 2 (extractor source lists) and Task 5 (completeness critic).

- [ ] **Step 1: Generate the v1 file inventory**

Run:
```bash
cd /Users/radaiko/dev/private/SnackPilot && git ls-files 'src/app/**' 'docs/**' 'analysis/**' '.github/**' 'src/desktop/**' 'tools/**' | grep -v -E 'package-lock|docs/superpowers' | sort > /private/tmp/claude-501/-Users-radaiko-dev-private-SnackPilot/1a058a60-76de-4c36-9bfc-8098a10ea161/scratchpad/v1-files.txt && wc -l /private/tmp/claude-501/-Users-radaiko-dev-private-SnackPilot/1a058a60-76de-4c36-9bfc-8098a10ea161/scratchpad/v1-files.txt
```
Expected: a file count (roughly 150–200 lines).

- [ ] **Step 2: Write `docs/requirements/appendix-source-map.md`**

Content (the table below is the complete mapping; append any inventory files it misses to the correct row or to the Dropped/N-A section — every line of `v1-files.txt` must appear exactly once):

```markdown
# Appendix: v1 Source → v2 Requirements Map

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

Every v1 file is either owned by a requirements doc or explicitly dropped. Paths are
relative to the repo root; app paths omit the `src/app/` prefix.

## Owned by docs

| v1 source | Owning doc(s) |
|---|---|
| `src-rn/api/gourmetClient.ts` | 01-gourmet-scraping |
| `src-rn/api/gourmetParser.ts` | 01-gourmet-scraping |
| `src-rn/api/gourmetApi.ts` | 01-gourmet-scraping |
| `src-rn/api/ventopayClient.ts` | 02-ventopay-scraping |
| `src-rn/api/ventopayParser.ts` | 02-ventopay-scraping |
| `src-rn/api/ventopayApi.ts` | 02-ventopay-scraping |
| `src-rn/api/types.ts` | 01-gourmet-scraping, 02-ventopay-scraping |
| `src-rn/api/demoData.ts` | 03-features/demo-mode |
| `src-rn/api/demoGourmetApi.ts` | 03-features/demo-mode |
| `src-rn/api/demoVentopayApi.ts` | 03-features/demo-mode |
| `src-rn/utils/constants.ts` | 01-gourmet-scraping, 02-ventopay-scraping |
| `src-rn/utils/dateUtils.ts` | 01-gourmet-scraping, 02-ventopay-scraping |
| `src-rn/store/menuStore.ts` | 03-features/menus, 03-features/caching |
| `src-rn/store/orderStore.ts` | 03-features/orders, 03-features/caching |
| `src-rn/store/billingStore.ts` | 03-features/billing, 03-features/caching |
| `src-rn/store/authStore.ts` | 03-features/settings, 05-platform-services, 01-gourmet-scraping |
| `src-rn/store/ventopayAuthStore.ts` | 03-features/settings, 05-platform-services, 02-ventopay-scraping |
| `src-rn/store/themeStore.ts` | 03-features/themes |
| `src-rn/store/locationStore.ts` | 03-features/notifications-location |
| `src-rn/utils/menuFingerprint.ts` | 03-features/notifications-new-menu |
| `src-rn/utils/menuChangeStorage.ts` | 03-features/notifications-new-menu |
| `src-rn/utils/backgroundMenuCheck.ts` | 03-features/notifications-new-menu |
| `src-rn/utils/notificationTasks.ts` | 03-features/notifications-new-menu, 05-platform-services |
| `src-rn/utils/notificationService.ts` | 03-features (all notifications-*), 05-platform-services |
| `src-rn/utils/dailyReminderCheck.ts` | 03-features/notifications-daily-reminder |
| `src-rn/utils/cancelReminderCheck.ts` | 03-features/notifications-cancel-reminder |
| `src-rn/utils/reminderStorage.ts` | 03-features/notifications-daily-reminder, notifications-cancel-reminder |
| `src-rn/utils/notificationLogStorage.ts` | 03-features/notification-log |
| `src-rn/utils/analytics.ts` | 03-features/analytics |
| `src-rn/components/AnalyticsProvider.tsx` | 03-features/analytics |
| `src-rn/utils/secureStorage.ts` | 05-platform-services |
| `src-rn/utils/platform.ts` | 05-platform-services |
| `src-rn/theme/colors.ts` | 03-features/themes, 04-ui-ux |
| `src-rn/theme/platformStyles.ts` | 03-features/themes, 04-ui-ux |
| `src-rn/theme/useTheme.ts` | 03-features/themes |
| `src-rn/types/menu.ts` | 03-features/menus |
| `src-rn/types/order.ts` | 03-features/orders |
| `src-rn/types/billing.ts` | 03-features/billing |
| `src-rn/types/ventopay.ts` | 02-ventopay-scraping |
| `app/_layout.tsx` | 04-ui-ux |
| `app/(tabs)/_layout.tsx` | 04-ui-ux |
| `app/(tabs)/index.tsx` | 03-features/menus, 04-ui-ux |
| `app/(tabs)/orders.tsx` | 03-features/orders, 04-ui-ux |
| `app/(tabs)/billing.tsx` | 03-features/billing, 04-ui-ux |
| `app/(tabs)/settings.tsx` | 03-features/settings, 04-ui-ux |
| `app/+not-found.tsx` | 04-ui-ux |
| `app/appearance.tsx` | 03-features/themes, 03-features/settings, 04-ui-ux |
| `app/kantine-login.tsx` | 03-features/settings, 04-ui-ux |
| `app/automaten-login.tsx` | 03-features/settings, 04-ui-ux |
| `app/notifications.tsx` | 03-features/notification-log, 03-features/settings, 04-ui-ux |
| `App.tsx`, `index.ts` | 04-ui-ux (bootstrap; RN-specific parts N/A) |
| `src-rn/components/MenuCard.tsx` | 03-features/menus, 04-ui-ux |
| `src-rn/components/DayNavigator.tsx` | 03-features/menus, 04-ui-ux |
| `src-rn/components/DateListPanel.tsx` | 03-features/menus, 04-ui-ux |
| `src-rn/components/NewMenuToast.tsx` | 03-features/notifications-new-menu, 04-ui-ux |
| `src-rn/components/OrderItem.tsx` | 03-features/orders, 04-ui-ux |
| `src-rn/components/OrdersPanel.tsx` | 03-features/orders, 04-ui-ux |
| `src-rn/components/BillCard.tsx` | 03-features/billing, 04-ui-ux |
| `src-rn/components/BillingFiltersPanel.tsx` | 03-features/billing, 04-ui-ux |
| `src-rn/components/DialogProvider.tsx` | 04-ui-ux |
| `src-rn/components/LoadingOverlay.tsx` | 04-ui-ux |
| `src-rn/components/AdaptiveBlurView.tsx` | 04-ui-ux |
| `src-rn/__tests__/**` (all) | 06-testing (and evidence for owning feature docs) |
| `src-rn/utils/__mocks__/**` | 06-testing |
| `scripts/record-fixtures.ts` | 06-testing |
| `jest.config.js` | 06-testing |
| `src-rn/__tests__/fixtures/**` | copied verbatim to `docs/fixtures/` |
| `app.json` | 05-platform-services, 07-release |
| `eas.json` | 07-release |
| `package.json` | 05-platform-services (plugin list), 06-testing, 07-release |
| `docs/app-store-release.md` | 07-release |
| `.github/workflows/release.yml` | 07-release |
| `.github/workflows/security-audit.yml` | 07-release |
| `tools/icon-tools/**` | 07-release |
| `docs/plans/2026-02-21-menu-reordering-*` | 03-features/menus |
| `docs/plans/2026-02-21-cache-menus-orders-design.md` | 03-features/caching |
| `docs/plans/2026-02-21-themes-app-icon-*` | 03-features/themes |
| `docs/plans/2026-02-24-new-menu-notifications*` | 03-features/notifications-new-menu |
| `docs/plans/2026-02-24-location-notifications*` | 03-features/notifications-location |
| `docs/plans/2026-02-25-daily-order-reminder*` | 03-features/notifications-daily-reminder |
| `analysis/playwright-findings.md` | 01-gourmet-scraping |
| `CLAUDE.md`, `README.md` | 00-overview, 01-, 02- |
| `node_modules/expo-secure-store/**` (native impls, v~55.0.15) | 05-platform-services (credential-takeover format) |

## Dropped in v2 / not applicable

| v1 source | Reason |
|---|---|
| `src/desktop/**` | Desktop app dropped in v2 |
| `docs/plans/2026-02-24-winhttp-proxy-*` | Desktop-only (Tauri HTTP proxy) |
| `src-rn/utils/tauriHttp.ts`, `tauriHttp.web.ts` | Desktop-only HTTP proxying |
| `src-rn/utils/desktopUpdater.ts`, `.web.ts` | Velopack updates retired |
| `src-rn/components/DesktopSidebar.tsx`, `DesktopContentWrapper.tsx` | Desktop layout |
| `src-rn/hooks/useDesktopLayout.ts` | Desktop layout |
| `src-rn/components/AdaptiveBlurView.web.tsx`, `AnalyticsProvider.web.tsx` | Web target dropped |
| `src-rn/utils/secureStorage.web.ts`, `notificationService.web.ts`, `notificationTasks.web.ts`, `cryptoPolyfill*.ts` | Web/RN-runtime specific |
| `src-rn/utils/desktopUpdater.web.test.ts`, `tauriHttp.web.test.ts` | Tests of dropped modules |
```

- [ ] **Step 3: Verify no inventory file is unmapped**

Cross-check `v1-files.txt` against the matrix by reading both; add missing rows. Every file must appear once (glob rows like `src-rn/__tests__/**` cover their matches).

- [ ] **Step 4: Commit**

```bash
git add docs/requirements/appendix-source-map.md && git commit -m "docs(v2): add v1 source coverage matrix (extraction acceptance criteria)"
```

---

### Task 2: Extraction workflow — produce all requirement docs

**Files:**
- Create: all docs listed in the `DOCS` array below (under `docs/requirements/`, `docs/architecture/`)

**Interfaces:**
- Consumes: `docs/requirements/appendix-source-map.md` (source lists), v1 source tree, `docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md` (for the two synthesis docs).
- Produces: the complete docs tree that Task 4 verifies.

- [ ] **Step 1: Run the extraction Workflow**

Invoke the Workflow tool with this script (inline):

```js
export const meta = {
  name: 'v2-docs-extraction',
  description: 'Extract v2 requirements docs from v1.4.5 source',
  phases: [
    { title: 'Extract', detail: 'one agent per requirements doc' },
    { title: 'Synthesize', detail: 'overview + v2 architecture (need all docs)' },
  ],
}

const ROOT = '/Users/radaiko/dev/private/SnackPilot'
const APP = ROOT + '/src/app'

const PREAMBLE = `You are extracting requirements for SnackPilot v2 (a native Swift/Kotlin rewrite
over a shared Rust core) from the v1.4.5 codebase. Baseline: main @ 6997c44.

Rules:
- Write the doc so an implementer WITH NO ACCESS to v1 source can rebuild the behavior
  exactly. For anything HTTP: byte-level precision (method, full URL, headers,
  content-type/encoding, every form field name and exact value, request ordering).
- State facts with exact values copied from code (URLs, selectors, regexes, key names,
  formats, thresholds, durations). Never paraphrase a regex or selector — copy it.
- Annotate provenance inline like (v1: src/app/src-rn/api/gourmetClient.ts:42).
- Start the doc with: "> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File
  references point into that commit."
- v1 is Expo/React Native; describe requirements platform-neutrally (WHAT, not the RN HOW),
  except where the RN mechanism matters for fidelity (e.g. cookie handling) — then describe
  v1's mechanism explicitly and mark it "v1 mechanism".
- Desktop (Tauri) and web targets are DROPPED in v2. Where a source file has desktop/web
  branches, document the mobile behavior and add a short "Dropped in v2" note.
- Read the listed test files too — they encode expected behavior and edge cases.
- If code contradicts CLAUDE.md or a docs/plans design doc, THE CODE WINS; note the
  discrepancy explicitly in the doc.
- Record anything you could not determine from source in openQuestions (do not guess).
- Write the file with the Write tool, then return the structured summary.`

const EXTRACT_SCHEMA = {
  type: 'object',
  properties: {
    summary: { type: 'string' },
    sourcesRead: { type: 'array', items: { type: 'string' } },
    openQuestions: { type: 'array', items: { type: 'string' } },
  },
  required: ['summary', 'sourcesRead', 'openQuestions'],
}

const DOCS = [
  {
    path: 'docs/requirements/01-gourmet-scraping.md',
    title: 'Gourmet (Kantine) scraping specification',
    sources: [
      APP + '/src-rn/api/gourmetClient.ts', APP + '/src-rn/api/gourmetParser.ts',
      APP + '/src-rn/api/gourmetApi.ts', APP + '/src-rn/api/types.ts',
      APP + '/src-rn/utils/constants.ts', APP + '/src-rn/utils/dateUtils.ts',
      ROOT + '/CLAUDE.md', ROOT + '/analysis/playwright-findings.md',
      APP + '/src-rn/__tests__/api/gourmetClient.test.ts',
      APP + '/src-rn/__tests__/api/gourmetParser.test.ts',
      APP + '/src-rn/__tests__/api/gourmetApi.test.ts',
      APP + '/src-rn/__tests__/fixtures/gourmet/ (list and skim all)',
    ],
    notes: 'THE most safety-critical doc: deviations ban accounts. Cover: full auth flow ' +
      '(ufprt + __ncforminfo, multipart/form-data, RememberMe literal "false", login ' +
      'verification regex), user-info extraction, menu pagination + selectors + category ' +
      'regexes, orders page forms incl. edit-mode toggle state machine, cancel flow, ' +
      'AddToMenuesCart + GetMyBillings JSON APIs with exact payload shapes and date ' +
      'formats, cookie/session handling (v1: withCredentials native cookies — mark as v1 ' +
      'mechanism), all "things that ban accounts" rules, error/ban detection behavior.',
  },
  {
    path: 'docs/requirements/02-ventopay-scraping.md',
    title: 'Ventopay (Automaten) scraping specification',
    sources: [
      APP + '/src-rn/api/ventopayClient.ts', APP + '/src-rn/api/ventopayParser.ts',
      APP + '/src-rn/api/ventopayApi.ts', APP + '/src-rn/types/ventopay.ts',
      APP + '/src-rn/api/types.ts', APP + '/src-rn/utils/constants.ts',
      ROOT + '/CLAUDE.md',
      APP + '/src-rn/__tests__/api/ventopayClient.test.ts',
      APP + '/src-rn/__tests__/api/ventopayParser.test.ts',
      APP + '/src-rn/__tests__/api/ventopayApi.test.ts',
      APP + '/src-rn/__tests__/fixtures/ventopay/ (list and skim all)',
    ],
    notes: 'Cover: ASP.NET viewstate login flow (all extracted fields, hardcoded company ' +
      'UUID 0da8d3ec-0178-47d5-9ccd-a996f04acb61, languageRadio, login verification ' +
      'regex), manual cookie management via interceptors (v1 mechanism), transaction ' +
      'list + detail requests with date param formats, all parsing selectors and the ' +
      'timestamp regex, item-row column semantics, German number parsing, the ' +
      'Gourmet/Kaffeeautomat filter rule.',
  },
  {
    path: 'docs/requirements/03-features/menus.md',
    title: 'Menu browsing feature',
    sources: [
      APP + '/src-rn/store/menuStore.ts', APP + '/app/(tabs)/index.tsx',
      APP + '/src-rn/components/MenuCard.tsx', APP + '/src-rn/components/DayNavigator.tsx',
      APP + '/src-rn/components/DateListPanel.tsx', APP + '/src-rn/types/menu.ts',
      ROOT + '/docs/plans/2026-02-21-menu-reordering-design.md',
      ROOT + '/docs/plans/2026-02-21-menu-reordering-impl.md',
      APP + '/src-rn/__tests__/store/menuStore.test.ts',
    ],
    notes: 'Cover: data model, day navigation, category ordering/reordering rules, ' +
      'availability display, allergens, refresh behavior, loading/error states, ' +
      'interaction with cart/ordering.',
  },
  {
    path: 'docs/requirements/03-features/orders.md',
    title: 'Ordering & cancellation feature',
    sources: [
      APP + '/src-rn/store/orderStore.ts', APP + '/app/(tabs)/orders.tsx',
      APP + '/src-rn/components/OrderItem.tsx', APP + '/src-rn/components/OrdersPanel.tsx',
      APP + '/src-rn/types/order.ts',
      APP + '/src-rn/__tests__/store/orderStore.test.ts',
    ],
    notes: 'Cover: cart building, order submission, ordered-menus list, approval states, ' +
      'cancellation UX + edit-mode flow, error handling, optimistic updates if any.',
  },
  {
    path: 'docs/requirements/03-features/billing.md',
    title: 'Billing feature (both sources)',
    sources: [
      APP + '/src-rn/store/billingStore.ts', APP + '/app/(tabs)/billing.tsx',
      APP + '/src-rn/components/BillCard.tsx',
      APP + '/src-rn/components/BillingFiltersPanel.tsx',
      APP + '/src-rn/types/billing.ts',
      APP + '/src-rn/__tests__/store/billingStore.test.ts',
    ],
    notes: 'Cover: merging Gourmet billing + Ventopay transactions, month/date handling, ' +
      'filters, totals/grouping, display rules, refresh behavior.',
  },
  {
    path: 'docs/requirements/03-features/caching.md',
    title: 'Caching & offline behavior',
    sources: [
      APP + '/src-rn/store/menuStore.ts', APP + '/src-rn/store/orderStore.ts',
      APP + '/src-rn/store/billingStore.ts',
      ROOT + '/docs/plans/2026-02-21-cache-menus-orders-design.md',
      APP + '/src-rn/__tests__/store/menuStore.test.ts',
      APP + '/src-rn/__tests__/store/orderStore.test.ts',
    ],
    notes: 'Cover: what is cached, storage mechanism/keys, staleness/invalidation rules, ' +
      'startup-from-cache behavior, cache vs live merge semantics.',
  },
  {
    path: 'docs/requirements/03-features/notifications-new-menu.md',
    title: 'New-menu notifications (background check + fingerprinting)',
    sources: [
      APP + '/src-rn/utils/backgroundMenuCheck.ts', APP + '/src-rn/utils/menuFingerprint.ts',
      APP + '/src-rn/utils/menuChangeStorage.ts', APP + '/src-rn/utils/notificationTasks.ts',
      APP + '/src-rn/utils/notificationService.ts',
      APP + '/src-rn/components/NewMenuToast.tsx',
      ROOT + '/docs/plans/2026-02-24-new-menu-notifications-design.md',
      ROOT + '/docs/plans/2026-02-24-new-menu-notifications.md',
      APP + '/src-rn/__tests__/utils/menuFingerprint.test.ts',
      APP + '/src-rn/__tests__/utils/menuChangeStorage.test.ts',
      APP + '/src-rn/__tests__/utils/notificationService.test.ts',
    ],
    notes: 'Cover: fingerprint algorithm exactly, change-detection semantics, background ' +
      'task cadence/registration (v1 mechanism: expo-background-task), notification ' +
      'content, in-app toast behavior, settings gating.',
  },
  {
    path: 'docs/requirements/03-features/notifications-daily-reminder.md',
    title: 'Daily order reminder',
    sources: [
      APP + '/src-rn/utils/dailyReminderCheck.ts', APP + '/src-rn/utils/reminderStorage.ts',
      APP + '/src-rn/utils/notificationService.ts',
      ROOT + '/docs/plans/2026-02-25-daily-order-reminder-design.md',
      ROOT + '/docs/plans/2026-02-25-daily-order-reminder.md',
      APP + '/src-rn/__tests__/utils/dailyReminderCheck.test.ts',
      APP + '/src-rn/__tests__/utils/reminderStorage.test.ts',
    ],
    notes: 'Cover: when the reminder fires (time, weekday, has-order conditions), ' +
      'dedupe/once-per-day logic, settings, notification content.',
  },
  {
    path: 'docs/requirements/03-features/notifications-cancel-reminder.md',
    title: 'Cancel reminder',
    sources: [
      APP + '/src-rn/utils/cancelReminderCheck.ts', APP + '/src-rn/utils/reminderStorage.ts',
      APP + '/src-rn/utils/notificationService.ts',
      APP + '/src-rn/__tests__/utils/cancelReminderCheck.test.ts',
    ],
    notes: 'Cover: trigger conditions (deadline proximity, ordered state), timing rules, ' +
      'dedupe logic, notification content, settings gating.',
  },
  {
    path: 'docs/requirements/03-features/notifications-location.md',
    title: 'Location-based notifications',
    sources: [
      APP + '/src-rn/store/locationStore.ts',
      ROOT + '/docs/plans/2026-02-24-location-notifications-design.md',
      ROOT + '/docs/plans/2026-02-24-location-notifications.md',
      APP + '/src-rn/__tests__/store/locationStore.test.ts',
    ],
    notes: 'Cover: geofence/region definition (coordinates, radius), permission flow, ' +
      'what fires on enter/exit, settings, battery considerations noted in design docs.',
  },
  {
    path: 'docs/requirements/03-features/notification-log.md',
    title: 'Notification debug log',
    sources: [
      APP + '/src-rn/utils/notificationLogStorage.ts', APP + '/app/notifications.tsx',
      APP + '/src-rn/__tests__/utils/notificationLogStorage.test.ts',
    ],
    notes: 'Cover: what gets logged, storage format/limits/rotation, log screen UX, ' +
      'clearing behavior.',
  },
  {
    path: 'docs/requirements/03-features/themes.md',
    title: 'Themes, appearance & app icons',
    sources: [
      APP + '/src-rn/store/themeStore.ts', APP + '/src-rn/theme/colors.ts',
      APP + '/src-rn/theme/useTheme.ts', APP + '/src-rn/theme/platformStyles.ts',
      APP + '/app/appearance.tsx',
      ROOT + '/docs/plans/2026-02-21-themes-app-icon-design.md',
      ROOT + '/docs/plans/2026-02-21-themes-app-icon-impl.md',
      APP + '/src-rn/__tests__/store/themeStore.test.ts',
      APP + '/src-rn/__tests__/theme/colors.test.ts',
    ],
    notes: 'Cover: theme list with EXACT color values, light/dark/system behavior, ' +
      'persistence, alternate app icons (v1: @g9k/expo-dynamic-app-icon) and which icon ' +
      'maps to which theme/setting.',
  },
  {
    path: 'docs/requirements/03-features/demo-mode.md',
    title: 'Demo mode',
    sources: [
      APP + '/src-rn/api/demoData.ts', APP + '/src-rn/api/demoGourmetApi.ts',
      APP + '/src-rn/api/demoVentopayApi.ts',
      APP + '/src-rn/__tests__/api/demoGourmetApi.test.ts',
      APP + '/src-rn/__tests__/api/demoVentopayApi.test.ts',
    ],
    notes: 'Cover: activation trigger (credentials? setting?), exactly what canned data is ' +
      'served, behavioral differences vs live mode, how it is indicated to the user.',
  },
  {
    path: 'docs/requirements/03-features/settings.md',
    title: 'Settings & login screens',
    sources: [
      APP + '/app/(tabs)/settings.tsx', APP + '/app/kantine-login.tsx',
      APP + '/app/automaten-login.tsx', APP + '/app/appearance.tsx',
      APP + '/app/notifications.tsx',
      APP + '/src-rn/store/authStore.ts', APP + '/src-rn/store/ventopayAuthStore.ts',
      APP + '/src-rn/__tests__/store/authStore.test.ts',
      APP + '/src-rn/__tests__/store/ventopayAuthStore.test.ts',
    ],
    notes: 'Cover: full settings inventory (every toggle/field/action incl. any ' +
      'mail-composer feedback action), login/logout flows for both services, credential ' +
      'validation UX, subpage navigation.',
  },
  {
    path: 'docs/requirements/03-features/analytics.md',
    title: 'Analytics',
    sources: [
      APP + '/src-rn/utils/analytics.ts', APP + '/src-rn/components/AnalyticsProvider.tsx',
      APP + '/src-rn/utils/__mocks__/analytics.ts',
    ],
    notes: 'Cover: provider/endpoint, every event name + payload, opt-in/out, ' +
      'PII handling.',
  },
  {
    path: 'docs/requirements/04-ui-ux.md',
    title: 'UI/UX specification',
    sources: [
      APP + '/app/ (every screen file)', APP + '/src-rn/components/ (every component)',
      APP + '/src-rn/theme/platformStyles.ts', APP + '/app/_layout.tsx',
      APP + '/app/(tabs)/_layout.tsx', APP + '/App.tsx',
    ],
    notes: 'Cover: screen inventory + navigation graph (tabs + sub-screens), per-screen ' +
      'layout/content/interactions, shared components and their behavior (dialogs, ' +
      'loading overlay, blur), platform styling differences, empty/loading/error states. ' +
      'Mark desktop-only components as dropped.',
  },
  {
    path: 'docs/requirements/05-platform-services.md',
    title: 'Platform services (storage, background, permissions, credential takeover)',
    sources: [
      APP + '/src-rn/utils/secureStorage.ts', APP + '/src-rn/store/authStore.ts',
      APP + '/src-rn/store/ventopayAuthStore.ts', APP + '/src-rn/utils/platform.ts',
      APP + '/src-rn/utils/notificationService.ts', APP + '/src-rn/utils/notificationTasks.ts',
      APP + '/app.json', APP + '/package.json',
      APP + '/node_modules/expo-secure-store/android/src/main/java/expo/modules/securestore/ (read the Kotlin sources)',
      APP + '/node_modules/expo-secure-store/ios/ (read the Swift sources)',
    ],
    notes: 'CRITICAL for credential takeover: document the exact v1 storage format — every ' +
      'secure-storage key name the app uses, and from expo-secure-store ~55.0.15 native ' +
      'sources: iOS keychain item class/service/account/accessible attributes; Android ' +
      'SharedPreferences file name, entry format (JSON structure), Keystore key alias, ' +
      'cipher/scheme (AES-GCM params). Also: background task registration (names, ' +
      'intervals), notification permission flow + channels, location permissions, ' +
      'expo-localization usage, all app.json permissions/plugins relevant to v2.',
  },
  {
    path: 'docs/requirements/06-testing.md',
    title: 'Testing strategy & fixtures',
    sources: [
      APP + '/jest.config.js', APP + '/scripts/record-fixtures.ts',
      APP + '/src-rn/__tests__/ (structure: list all files; read setup.ts and 2-3 ' +
      'representative test files across api/store/utils)',
      ROOT + '/.env.example',
    ],
    notes: 'Cover: record & replay strategy, fixture inventory (all 13 files) + what each ' +
      'covers, sanitization rules applied when recording, recorder script behavior + ' +
      '.env requirements, what the v2 Rust core test suite must replicate (request-shape ' +
      'assertions, parser tests, orchestration tests).',
  },
  {
    path: 'docs/requirements/07-release.md',
    title: 'Release, distribution & CI',
    sources: [
      ROOT + '/docs/app-store-release.md', ROOT + '/.github/workflows/release.yml',
      ROOT + '/.github/workflows/security-audit.yml', APP + '/eas.json', APP + '/app.json',
      ROOT + '/tools/icon-tools/ (skim)',
    ],
    notes: 'Cover: store identities (bundle ID / package dev.radaiko.gourmetclient — MUST ' +
      'stay for update path), versioning scheme, release process per store, what v1 CI ' +
      'does and what v2 CI must do (Rust core build+test both archs, iOS build, Android ' +
      'build), icon generation pipeline. Velopack/desktop release: mark retired.',
  },
]

phase('Extract')
const extractResults = await parallel(DOCS.map(d => () =>
  agent(
    PREAMBLE +
    `\n\nYour doc: ${ROOT}/${d.path}\nTitle: ${d.title}\n\nScope notes: ${d.notes}\n\n` +
    `Sources to read (read ALL of them):\n${d.sources.join('\n')}\n\n` +
    `Also consult ${ROOT}/docs/requirements/appendix-source-map.md to see which OTHER ` +
    `docs exist, so you can reference instead of duplicating (e.g. features reference ` +
    `01-/02- for HTTP details; do not restate request specs).`,
    { label: d.path.split('/').pop(), phase: 'Extract', schema: EXTRACT_SCHEMA }
  )
))

// Barrier justified: overview + architecture must read ALL extracted docs.
phase('Synthesize')
const synthesis = await parallel([
  () => agent(
    PREAMBLE +
    `\n\nWrite ${ROOT}/docs/requirements/00-overview.md: product overview (company ` +
    `cafeteria ordering + billing, two scraped sources), users, v2 platform scope ` +
    `(iOS 17+ Swift/SwiftUI, Android 10+ Kotlin/Compose, shared Rust core; desktop/web ` +
    `dropped), v1→v2 rationale, glossary (Kantine, Automaten, Gourmet, Ventopay, menu ` +
    `categories, eating cycle, etc.), and an index of every doc in docs/requirements ` +
    `with one-line summaries. Read: ${ROOT}/README.md, ${ROOT}/CLAUDE.md, ` +
    `${ROOT}/docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md, and every ` +
    `doc under ${ROOT}/docs/requirements/.`,
    { label: '00-overview.md', phase: 'Synthesize', schema: EXTRACT_SCHEMA }
  ),
  () => agent(
    PREAMBLE +
    `\n\nWrite ${ROOT}/docs/architecture/v2-architecture.md: expand §4 of ` +
    `${ROOT}/docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md into the v2 ` +
    `architecture doc: repo layout, thick Rust core responsibilities (map each ` +
    `docs/requirements doc to the core module or native shell that implements it), ` +
    `proposed UniFFI facade surface (operations + record types derived from the domain ` +
    `models in the feature docs), host-injection points (storage path, credentials, ` +
    `settings), threading/async model, per-platform service table, credential-takeover ` +
    `flow (from 05-platform-services.md), testing architecture (from 06-testing.md). ` +
    `Read the spec and every doc under ${ROOT}/docs/requirements/ first.`,
    { label: 'v2-architecture.md', phase: 'Synthesize', schema: EXTRACT_SCHEMA }
  ),
])

return {
  docs: extractResults.map((r, i) => r ? ({
    path: DOCS[i].path,
    summary: r.summary,
    openQuestions: r.openQuestions,
  }) : ({ path: DOCS[i].path, summary: 'AGENT FAILED — re-run this doc', openQuestions: [] })),
  synthesis: synthesis.filter(Boolean).map(r => ({ summary: r.summary, openQuestions: r.openQuestions })),
}
```

- [ ] **Step 2: Review workflow result**

Check the returned `openQuestions` across all docs. For each: answer it yourself by reading the v1 source, then edit the doc; if genuinely unanswerable from source (e.g. live-site behavior), record it in the doc under an explicit `## Open questions` section — that is allowed; silent gaps are not.

- [ ] **Step 3: Verify all files exist**

Run: `ls docs/requirements docs/requirements/03-features docs/architecture`
Expected: 8 files in `requirements/` (00,01,02,04,05,06,07,appendix), 13 in `03-features/`, 1 in `architecture/`.

- [ ] **Step 4: Commit**

```bash
git add docs/requirements docs/architecture && git commit -m "docs(v2): extract requirements from v1.4.5 (unverified draft)"
```

---

### Task 3: Fixtures carry-over + v2 scaffolding

**Files:**
- Create: `docs/fixtures/` (copied), `v2-scaffold/README.md`, `v2-scaffold/CLAUDE.md`, `v2-scaffold/gitignore`, `v2-scaffold/env.example`

Scaffold files are stored WITHOUT leading dots (`gitignore`, `env.example`) so they don't take effect on `v2/planning`; Task 6 renames them at the orphan-branch root.

**Interfaces:**
- Produces: everything Task 6 places at the v2 branch root.

- [ ] **Step 1: Copy fixtures**

```bash
mkdir -p docs/fixtures && cp -R src/app/src-rn/__tests__/fixtures/gourmet docs/fixtures/gourmet && cp -R src/app/src-rn/__tests__/fixtures/ventopay docs/fixtures/ventopay && find docs/fixtures -type f | wc -l
```
Expected: `13`

- [ ] **Step 2: Write `v2-scaffold/README.md`**

```markdown
# SnackPilot

Company cafeteria menu ordering and billing for iOS and Android. Scrapes two
external systems — Kantine (Gourmet) for menus/orders/billing and Automaten
(Ventopay) for POS transactions.

**v2 status:** requirements phase. This branch currently contains the complete,
verified requirements extracted from v1.4.5 under [`docs/`](docs/). Implementation
(Rust core + native SwiftUI/Compose apps) lands next; see
`docs/architecture/v2-architecture.md`.

v1 (Expo React Native + Tauri, shipped through v1.4.5) lives on the `main` branch.

## Credits

Based on [GourmetClient](https://github.com/patrickl92/GourmetClient) by patrickl92,
the original project this app was forked from.
```

- [ ] **Step 3: Write `v2-scaffold/CLAUDE.md`**

```markdown
# SnackPilot v2

Native mobile apps for company cafeteria ordering and billing: iOS (Swift/SwiftUI,
iOS 17+) and Android (Kotlin/Jetpack Compose, Android 10+/API 29) over a shared
Rust core (`snackpilot-core`, UniFFI bindings) that owns all scraping, parsing,
caching, fingerprinting, and notification decision logic.

## Source of truth

`docs/requirements/` is authoritative — extracted and adversarially verified from
v1.4.5 (`main` @ 6997c44). When code and docs disagree during the rewrite, treat it
as a defect in one of them and resolve explicitly; never silently diverge.

## Critical Warning

**DO NOT DEVIATE FROM THE SCRAPING SPECS** in `docs/requirements/01-gourmet-scraping.md`
and `02-ventopay-scraping.md`. The app scrapes websites, not APIs. Any deviation from
the exact request sequences, headers, encodings, or parameter values can trigger
account bans on the external services.

## README Requirements

The README must always include a credit line linking to
https://github.com/patrickl92/GourmetClient as the base/original project.

## Layout

```
docs/requirements/     Verified v1-parity requirements (authoritative)
docs/architecture/     v2 architecture (Rust core + native shells)
docs/fixtures/         Sanitized HTML/JSON fixtures recorded from the live sites
src/core/              Rust core (planned)
src/ios/               SwiftUI app (planned)
src/android/           Compose app (planned)
```

## App identity

iOS bundle ID and Android package are `dev.radaiko.gourmetclient` and MUST NOT
change — v2 ships as a store update to v1 installs and imports v1 credentials
(see `docs/requirements/05-platform-services.md`).
```

- [ ] **Step 4: Copy env example and write gitignore**

```bash
cp .env.example v2-scaffold/env.example
```

Write `v2-scaffold/gitignore`:

```gitignore
.env
.DS_Store

# Rust
src/core/target/

# iOS
src/ios/build/
*.xcuserdatad/
src/ios/DerivedData/

# Android
src/android/.gradle/
src/android/build/
src/android/app/build/
src/android/local.properties
```

- [ ] **Step 5: Commit**

```bash
git add docs/fixtures v2-scaffold && git commit -m "docs(v2): carry over sanitized fixtures; add v2 branch scaffolding"
```

---

### Task 4: Adversarial verification workflow (loop until dry)

**Files:**
- Modify: any doc under `docs/requirements/`, `docs/architecture/` (fixes)

**Interfaces:**
- Consumes: all docs from Tasks 1–2, v1 source, coverage matrix.
- Produces: verified docs; a residual-findings list if rounds are exhausted (must be surfaced to the user, not dropped).

- [ ] **Step 1: Run the verification Workflow**

```js
export const meta = {
  name: 'v2-docs-verify',
  description: 'Adversarially verify extracted docs against v1 source, fix, repeat until dry',
  phases: [{ title: 'Verify' }, { title: 'Fix' }],
}

const ROOT = '/Users/radaiko/dev/private/SnackPilot'

const FINDINGS_SCHEMA = {
  type: 'object',
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          kind: { type: 'string', enum: ['omission', 'invention', 'imprecision'] },
          claim: { type: 'string' },
          evidence: { type: 'string' },
          severity: { type: 'string', enum: ['ban-risk', 'behavior', 'minor'] },
          suggestedFix: { type: 'string' },
        },
        required: ['kind', 'claim', 'evidence', 'severity', 'suggestedFix'],
      },
    },
  },
  required: ['findings'],
}
const FIX_SCHEMA = {
  type: 'object',
  properties: { applied: { type: 'number' }, rejected: { type: 'array', items: { type: 'string' } } },
  required: ['applied', 'rejected'],
}

// Doc → the v1 sources a verifier must re-read (same lists as extraction).
// Keep in sync with docs/requirements/appendix-source-map.md.
const DOCS = [
  { path: 'docs/requirements/01-gourmet-scraping.md', scraping: true,
    sources: 'src/app/src-rn/api/gourmetClient.ts, gourmetParser.ts, gourmetApi.ts, api/types.ts, src/app/src-rn/utils/constants.ts, dateUtils.ts, CLAUDE.md, analysis/playwright-findings.md, src/app/src-rn/__tests__/api/gourmet*.test.ts, fixtures/gourmet/' },
  { path: 'docs/requirements/02-ventopay-scraping.md', scraping: true,
    sources: 'src/app/src-rn/api/ventopayClient.ts, ventopayParser.ts, ventopayApi.ts, src/app/src-rn/types/ventopay.ts, src/app/src-rn/utils/constants.ts, CLAUDE.md, src/app/src-rn/__tests__/api/ventopay*.test.ts, fixtures/ventopay/' },
  { path: 'docs/requirements/03-features/menus.md',
    sources: 'src/app/src-rn/store/menuStore.ts, src/app/app/(tabs)/index.tsx, src/app/src-rn/components/{MenuCard,DayNavigator,DateListPanel}.tsx, src/app/src-rn/types/menu.ts, docs/plans/2026-02-21-menu-reordering-*' },
  { path: 'docs/requirements/03-features/orders.md',
    sources: 'src/app/src-rn/store/orderStore.ts, src/app/app/(tabs)/orders.tsx, src/app/src-rn/components/{OrderItem,OrdersPanel}.tsx, src/app/src-rn/types/order.ts' },
  { path: 'docs/requirements/03-features/billing.md',
    sources: 'src/app/src-rn/store/billingStore.ts, src/app/app/(tabs)/billing.tsx, src/app/src-rn/components/{BillCard,BillingFiltersPanel}.tsx, src/app/src-rn/types/billing.ts' },
  { path: 'docs/requirements/03-features/caching.md',
    sources: 'src/app/src-rn/store/{menuStore,orderStore,billingStore}.ts, docs/plans/2026-02-21-cache-menus-orders-design.md' },
  { path: 'docs/requirements/03-features/notifications-new-menu.md',
    sources: 'src/app/src-rn/utils/{backgroundMenuCheck,menuFingerprint,menuChangeStorage,notificationTasks,notificationService}.ts, src/app/src-rn/components/NewMenuToast.tsx, docs/plans/2026-02-24-new-menu-notifications*' },
  { path: 'docs/requirements/03-features/notifications-daily-reminder.md',
    sources: 'src/app/src-rn/utils/{dailyReminderCheck,reminderStorage,notificationService}.ts, docs/plans/2026-02-25-daily-order-reminder*' },
  { path: 'docs/requirements/03-features/notifications-cancel-reminder.md',
    sources: 'src/app/src-rn/utils/{cancelReminderCheck,reminderStorage,notificationService}.ts' },
  { path: 'docs/requirements/03-features/notifications-location.md',
    sources: 'src/app/src-rn/store/locationStore.ts, docs/plans/2026-02-24-location-notifications*' },
  { path: 'docs/requirements/03-features/notification-log.md',
    sources: 'src/app/src-rn/utils/notificationLogStorage.ts, src/app/app/notifications.tsx' },
  { path: 'docs/requirements/03-features/themes.md',
    sources: 'src/app/src-rn/store/themeStore.ts, src/app/src-rn/theme/*.ts, src/app/app/appearance.tsx, docs/plans/2026-02-21-themes-app-icon-*' },
  { path: 'docs/requirements/03-features/demo-mode.md',
    sources: 'src/app/src-rn/api/{demoData,demoGourmetApi,demoVentopayApi}.ts' },
  { path: 'docs/requirements/03-features/settings.md',
    sources: 'src/app/app/(tabs)/settings.tsx, src/app/app/{kantine-login,automaten-login,appearance,notifications}.tsx, src/app/src-rn/store/{authStore,ventopayAuthStore}.ts' },
  { path: 'docs/requirements/03-features/analytics.md',
    sources: 'src/app/src-rn/utils/analytics.ts, src/app/src-rn/components/AnalyticsProvider.tsx' },
  { path: 'docs/requirements/04-ui-ux.md',
    sources: 'src/app/app/** (all screens), src/app/src-rn/components/** (all), src/app/src-rn/theme/platformStyles.ts' },
  { path: 'docs/requirements/05-platform-services.md',
    sources: 'src/app/src-rn/utils/{secureStorage,platform,notificationService,notificationTasks}.ts, src/app/src-rn/store/{authStore,ventopayAuthStore}.ts, src/app/app.json, src/app/node_modules/expo-secure-store native sources' },
  { path: 'docs/requirements/06-testing.md',
    sources: 'src/app/jest.config.js, src/app/scripts/record-fixtures.ts, src/app/src-rn/__tests__/** structure' },
  { path: 'docs/requirements/07-release.md',
    sources: 'docs/app-store-release.md, .github/workflows/*.yml, src/app/eas.json, src/app/app.json, tools/icon-tools/' },
  { path: 'docs/architecture/v2-architecture.md',
    sources: 'docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md and all docs/requirements/*' },
]

const LENSES = [
  { key: 'omission', prompt: 'Hunt OMISSIONS: behavior, values, edge cases, or rules present in the v1 sources but missing from the doc. Read the sources first, list every requirement-relevant fact, then check each against the doc.' },
  { key: 'invention', prompt: 'Hunt INVENTIONS and IMPRECISIONS: claims in the doc not supported by the v1 sources, or stated values (URLs, selectors, regexes, key names, formats, timings) that do not match the code exactly. Check every concrete value in the doc against the source, character by character.' },
]
const SHAPE_LENS = {
  key: 'request-shape',
  prompt: 'You are a request-shape auditor. Reconstruct, from the doc ALONE, every HTTP request the client would send (method, URL, headers, content-type, every field name/value, ordering). Then diff that against what the v1 client code actually sends and what the recorded fixtures/tests assert. Any mismatch or underspecification that could change a request byte is a finding with severity ban-risk.',
}

let round = 0
let pending = DOCS
let residual = []

while (pending.length && round < 3) {
  round += 1
  const results = await pipeline(
    pending,
    (d) => {
      const lenses = d.scraping ? [...LENSES, SHAPE_LENS] : LENSES
      return parallel(lenses.map(lens => () =>
        agent(
          `Verify ${ROOT}/${d.path} against v1 source (baseline main@6997c44, checked out in ${ROOT}).\n` +
          `${lens.prompt}\n\nSources to re-read: ${d.sources}\n\n` +
          `Do NOT report style issues. Only requirement-relevant discrepancies. For each finding give exact evidence as v1 path:line. If the doc references another doc for details (e.g. defers HTTP to 01-/02-), that is correct, not an omission.`,
          { label: `${lens.key}:${d.path.split('/').pop()}`, phase: 'Verify', schema: FINDINGS_SCHEMA }
        )
      )).then(vs => ({ d, findings: vs.filter(Boolean).flatMap(v => v.findings) }))
    },
    (r, d) => {
      if (!r || r.findings.length === 0) return { d, applied: 0 }
      return agent(
        `Apply these verified findings to ${ROOT}/${r.d.path}. For each: re-check the evidence in the v1 source yourself; if it holds, edit the doc; if not, reject it with a reason. Preserve the doc's structure and provenance annotations.\n\nFindings:\n${JSON.stringify(r.findings, null, 2)}`,
        { label: `fix:${r.d.path.split('/').pop()}`, phase: 'Fix', schema: FIX_SCHEMA }
      ).then(f => ({ d: r.d, applied: f ? f.applied : 0, rejected: f ? f.rejected : [] }))
    }
  )
  const clean = results.filter(Boolean)
  const changed = clean.filter(r => r.applied > 0)
  log(`Round ${round}: ${clean.reduce((s, r) => s + r.applied, 0)} fixes across ${changed.length} docs`)
  if (round === 3 && changed.length) residual = changed.map(r => r.d.path)
  pending = changed.map(r => r.d)   // re-verify only docs that were modified
}

return { rounds: round, residualDocs: residual }
```

- [ ] **Step 2: Handle residuals**

If `residualDocs` is non-empty after 3 rounds, read those docs' latest findings yourself, resolve each by reading the v1 source directly, and note anything still uncertain under `## Open questions` in the doc. Report this to the user in the task summary.

- [ ] **Step 3: Commit**

```bash
git add docs/requirements docs/architecture && git commit -m "docs(v2): adversarial verification fixes"
```

---

### Task 5: Cross-doc consistency critic + final read gate

**Files:**
- Modify: any doc under `docs/` (consistency fixes)

- [ ] **Step 1: Dispatch completeness critic**

Dispatch one agent (Agent tool, general-purpose):

> Read every file under `/Users/radaiko/dev/private/SnackPilot/docs/requirements/` and `docs/architecture/`. You are the completeness critic for the SnackPilot v2 requirements. Report: (1) cross-doc contradictions (same fact stated differently in two docs — e.g. a date format, key name, or URL); (2) terminology drift (same concept named differently across docs; check against 00-overview's glossary); (3) coverage-matrix violations — pick 10 random rows of `appendix-source-map.md`, open each v1 source file, and confirm its owning doc actually covers it; (4) dangling references (doc A references doc B section that doesn't exist); (5) anything in the design spec `docs/superpowers/specs/2026-07-08-v2-native-rewrite-design.md` §1 feature list with no covering requirements doc. Return a numbered list of defects with file paths; empty list if clean.

- [ ] **Step 2: Fix reported defects, re-run critic if any were structural**

Apply each defect fix directly. If any defect was a missing feature/coverage gap (category 3 or 5), re-run the Step 1 critic once after fixing.

- [ ] **Step 3: Personally read the two scraping docs (executor gate)**

Read `docs/requirements/01-gourmet-scraping.md` and `02-ventopay-scraping.md` end-to-end and check them against the CLAUDE.md scraping specification sections (the historically trusted baseline). Every rule in CLAUDE.md's "Things That Will Break Accounts" must appear in the docs. Fix any gap.

- [ ] **Step 4: Commit**

```bash
git add docs && git commit -m "docs(v2): cross-doc consistency fixes; verification complete"
```

---

### Task 6: Assemble the orphan `v2` branch

**Files:**
- Create: git branch `v2` (orphan), worktree at `/Users/radaiko/dev/private/SnackPilot-v2`

**Interfaces:**
- Consumes: committed `docs/` tree and `v2-scaffold/` from `v2/planning`.
- Produces: the `v2` branch whose single commit is the starting point for all v2 implementation plans; the worktree is the workspace for the next phase.

- [ ] **Step 1: Create orphan branch in a fresh worktree**

```bash
cd /Users/radaiko/dev/private/SnackPilot
git worktree add ../SnackPilot-v2 --detach v2/planning
cd ../SnackPilot-v2
git checkout --orphan v2
git rm -rf . >/dev/null
```
Expected: empty working tree (`ls` shows nothing), on branch `v2` with no commits.

- [ ] **Step 2: Bring in docs + scaffolding**

```bash
git checkout v2/planning -- docs/requirements docs/architecture docs/fixtures docs/superpowers v2-scaffold
mv v2-scaffold/README.md README.md
mv v2-scaffold/CLAUDE.md CLAUDE.md
mv v2-scaffold/gitignore .gitignore
mv v2-scaffold/env.example .env.example
rmdir v2-scaffold
git add -A
```

- [ ] **Step 3: Verify first-commit contents**

```bash
git status --short | grep -v '^A ' ; git diff --cached --stat | tail -3
```
Expected: no non-`A` status lines; staged files = docs tree + README.md + CLAUDE.md + .gitignore + .env.example only. Verify no `src/`, no `package.json`:
```bash
git diff --cached --name-only | grep -E '^(src/|package)' ; echo "exit: $?"
```
Expected: no output, `exit: 1`.

- [ ] **Step 4: Create the first commit**

```bash
git commit -m "SnackPilot v2.0 — requirements extracted from v1.4.5 (main @ 6997c44)

Docs-only starting point for the native rewrite: verified requirements,
v2 architecture (Rust core + SwiftUI/Compose shells), sanitized fixtures,
and repo scaffolding. Implementation follows per docs/superpowers/plans/."
git log --oneline --all --graph | head -5
```
Expected: `v2` has exactly one commit, unconnected to `main`'s history.

- [ ] **Step 5: Report to user (do not push)**

Summarize: branch created, worktree location `/Users/radaiko/dev/private/SnackPilot-v2`, doc inventory, residual open questions if any. Ask whether to push `v2` and `v2/planning` to origin.

---

## Post-plan

Subsequent phases (each gets its own plan, written after this one completes, informed by the verified docs):
1. Rust core (`src/core`) — models, clients, parsers, APIs, caching, fingerprinting, decision logic, demo mode, fixture test suite.
2. iOS app (`src/ios`).
3. Android app (`src/android`).
4. Parity audit.
