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
