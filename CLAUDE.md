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
