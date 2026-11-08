# Host-test link shims (`.devlibs`)

`cargo test -p snackpilot-app` on Linux fails to link: gpui's X11 deps request
versionless sonames (`-lxcb`, `-lxkbcommon-x11`) that Ubuntu does not ship as
symlinks. Build a gitignored `.devlibs/` at the repo root and point `RUSTFLAGS` at it:

    mkdir -p .devlibs
    ln -sf /usr/lib/x86_64-linux-gnu/libxcb.so.1        .devlibs/libxcb.so
    ln -sf /usr/lib/x86_64-linux-gnu/libxkbcommon.so.0  .devlibs/libxkbcommon.so
    # libxkbcommon-x11 is not installed system-wide; extract it once (no sudo):
    #   cd /tmp/opencode/dl && apt-get download libxkbcommon-x11-0 \
    #     && dpkg-deb -x libxkbcommon-x11-0_*.deb prefix
    ln -sf /tmp/opencode/dl/prefix/usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0.0.0 \
        .devlibs/libxkbcommon-x11.so

    RUST_FONTCONFIG_DLOPEN=on RUSTFLAGS="-L $(git rev-parse --show-toplevel)/.devlibs" \
        cargo test -p snackpilot-app

CI does not need this: it runs `make ui-check` (clippy on `snackpilot-model`), not
host app tests. Android tests run via `cargo ndk` on the emulator and are unaffected.
