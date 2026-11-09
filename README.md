# SnackPilot

Ordering and billing app for the company cafeteria — one Rust UI for iOS, Android
and a desktop development window. Scrapes two
external systems — Kantine (Gourmet) for menus/orders/billing and Automaten (Ventopay)
for POS transactions.

## Architecture

Shared Rust core (scraping, caching, business logic, notifications, demo mode) and one
gpui-kit UI written in Rust, mounted by thin platform hosts:

- UI — `src/ui/app` (gpui-pre + gpui-kit), screens in `src/ui/app/src/screens/`
- Model — `src/ui/model` (`snackpilot-model`: session, state, screen logic, tests)
- Hosts — `src/ui/ios` (UIKit containment, iOS 17+), `src/ui/android`
  (NativeActivity, Android 10+/API 29), `src/ui/app/src/bin/desktop.rs` (macOS)
- Core — `src/core/` (crate `snackpilot-core`), unchanged

`make check` runs the core suites and `make ui-check` (UI fmt, clippy, model tests).
See [`docs/requirements/08-gpui-desktop.md`](docs/requirements/08-gpui-desktop.md) for the
desktop addendum and known regressions.

## Build & release

`make help` at the repo root lists the local build/run/ship targets; see
[`tools/devops/README.md`](tools/devops/README.md).

## History

v1 (Expo React Native + Tauri desktop, through v1.4.5) is preserved on the `archive/v1`
branch. v2 ships iOS and Android; the macOS window is a development/gallery target.

## Credits

Based on [GourmetClient](https://github.com/patrickl92/GourmetClient) by patrickl92, the
original project this app was forked from.
