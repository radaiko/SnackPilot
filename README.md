# SnackPilot

Native iOS & Android app for ordering and billing at the company cafeteria. Scrapes two
external systems — Kantine (Gourmet) for menus/orders/billing and Automaten (Ventopay)
for POS transactions.

## Architecture

Shared Rust core (scraping, caching, business logic, notifications, demo mode) exposed via
UniFFI to two native shells:

- iOS — SwiftUI (iOS 17+), `src/ios/`
- Android — Jetpack Compose (Android 10+/API 29), `src/android/`
- Core — `src/core/` (crate `snackpilot-core`)

## Build & release

`make help` at the repo root lists the local build/run/ship targets; see
[`tools/devops/README.md`](tools/devops/README.md).

## History

v1 (Expo React Native + Tauri desktop, through v1.4.5) is preserved on the `archive/v1`
branch. Desktop is not supported in v2.

## Credits

Based on [GourmetClient](https://github.com/patrickl92/GourmetClient) by patrickl92, the
original project this app was forked from.
