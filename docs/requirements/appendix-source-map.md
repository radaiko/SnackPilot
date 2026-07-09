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
| `src-rn/utils/notificationTasks.ts` (defines `COMPANY_GEOFENCE_TASK` + `BACKGROUND_ORDER_SYNC_TASK`; no menu logic) | 03-features/notifications-location, notifications-daily-reminder, notifications-cancel-reminder, notification-log, 05-platform-services |
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
| `src-rn/__tests__/**` (all tests + setup) | 06-testing (and evidence for owning feature docs) |
| `src-rn/utils/__mocks__/**` | 06-testing |
| `scripts/record-fixtures.ts` | 06-testing |
| `jest.config.js` | 06-testing |
| `src-rn/__tests__/fixtures/**` (13 files) | copied verbatim to `docs/fixtures/` |
| `app.json` | 05-platform-services, 07-release |
| `eas.json` | 07-release |
| `package.json` | 05-platform-services (plugin list), 06-testing, 07-release |
| `assets/**` (app icon, splash, 5 theme icon variants: berry, emerald, golden, ocean, orange) | 03-features/themes, 07-release; binary/SVG assets copied from `main` during app implementation |
| `patches/@g9k+expo-dynamic-app-icon+2.0.8.patch` | 03-features/themes (v1 mechanism for alternate icons) |
| `docs/app-store-release.md` | 07-release |
| `docs/privacy.html` | 07-release (store privacy policy; must carry to v2) |
| `.github/workflows/release.yml` | 07-release |
| `.github/workflows/security-audit.yml` | 07-release |
| `.github/dependabot.yml` | 07-release (dependency automation; v2 needs Rust/Swift/Gradle equivalents) |
| `.github/icon.png` | 07-release (repo branding asset) |
| `tools/icon-tools/**` | 07-release |
| `docs/plans/2026-02-21-menu-reordering-*` | 03-features/menus |
| `docs/plans/2026-02-21-cache-menus-orders-design.md` | 03-features/caching |
| `docs/plans/2026-02-21-themes-app-icon-*` | 03-features/themes |
| `docs/plans/2026-02-24-new-menu-notifications*` | 03-features/notifications-new-menu |
| `docs/plans/2026-02-24-location-notifications*` | 03-features/notifications-location |
| `docs/plans/2026-02-25-daily-order-reminder*` | 03-features/notifications-daily-reminder |
| `analysis/playwright-findings.md` | 01-gourmet-scraping |
| `CLAUDE.md`, `README.md` | 00-overview, 01-, 02- |
| `node_modules/expo-secure-store/**` (native impls, v~55.0.15; not in git) | 05-platform-services (credential-takeover format) |

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
| `src-rn/utils/secureStorage.web.ts`, `notificationService.web.ts`, `notificationTasks.web.ts`, `cryptoPolyfill.ts`, `cryptoPolyfill.web.ts` | Web/RN-runtime specific |
| `src-rn/__tests__/utils/desktopUpdater.web.test.ts`, `tauriHttp.web.test.ts` | Tests of dropped modules |
| `src/app/.gitignore`, `.npmrc`, `tsconfig.json`, `metro.config.js` | v1 (RN/Metro) build configuration |
