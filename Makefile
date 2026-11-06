# SnackPilot v2 — local devops entrypoint. Run `make help`.
SHELL := /bin/bash
DEVOPS := tools/devops

.DEFAULT_GOAL := help

.PHONY: help doctor clean check lint test spec-lint ui-check ios-run ios-archive android-run android-keystore ship

help: ## List available targets
	@grep -hE '^[a-zA-Z0-9_-]+:.*?## ' $(MAKEFILE_LIST) \
	  | awk 'BEGIN{FS=":.*?## "} {printf "  \033[1m%-18s\033[0m %s\n", $$1, $$2}'

check: lint test spec-lint ui-check ## Everything CI runs (fmt, clippy, core tests, spec validation, ui model)

lint: ## Rust formatting and lints
	cd src/core && cargo fmt --all --check
	cd src/core && cargo clippy --all-targets --all-features -- -D warnings

test: ## Core tests
	cd src/core && cargo test --workspace --all-targets --all-features

spec-lint: ## OpenSpec validation
	OPENSPEC_TELEMETRY=0 openspec validate --all --strict --no-interactive

# The gpui app crate needs system graphics libraries and only builds for mobile/desktop
# targets (design Risks), so clippy and tests here cover the model crate only.
ui-check: ## UI workspace fmt, model lints and tests
	cd src/ui && cargo fmt --all --check
	cd src/ui && cargo clippy -p snackpilot-model --all-targets -- -D warnings
	cd src/ui && cargo test -p snackpilot-model

doctor: ## Check that required build tools are installed
	@bash $(DEVOPS)/doctor.sh

clean: ## Remove dist/ and generated build artifacts
	@rm -rf dist src/ios/build src/android/app/build && echo "cleaned dist/ + build outputs"

ios-run: ## Build + install + launch on an iOS simulator (DEVICE= to override)
	@bash $(DEVOPS)/run-ios.sh

ios-archive: ## Build a signed release archive + open Xcode Organizer (App Store/TestFlight upload)
	@bash $(DEVOPS)/archive-ios.sh

android-run: ## Build + install + launch on an Android emulator/device
	@bash $(DEVOPS)/run-android.sh

android-keystore: ## One-time: generate the Android release keystore
	@bash $(DEVOPS)/android-keystore.sh

ship: ## Interactive release: bump version, build artifacts, commit + tag (DRY_RUN=1)
	@bash $(DEVOPS)/ship.sh
