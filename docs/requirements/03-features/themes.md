# Themes, Appearance & App Icons

> Extracted from SnackPilot v1.4.5 (main @ 6997c44). File references point into that commit.

This document specifies the appearance system: the light/dark/system mode preference, the
five selectable accent color themes (with exact color values), how the two settings compose
into the final color palette, how they persist, and the alternate app icon feature that keeps
the home-screen icon in sync with the selected accent.

Related docs:
- `04-ui-ux` — per-component surface style recipes (card, banner, button, input, FAB) that
  consume the palette defined here (v1: src/app/src-rn/theme/platformStyles.ts).
- `03-features/settings.md` — the Settings screen that links to the Appearance screen.
- `07-release` — build-time icon/splash configuration beyond runtime switching.

---

## 1. Appearance model

Appearance is controlled by **two independent user settings** (v1: src/app/src-rn/store/themeStore.ts:9-14):

| Setting | Type | Allowed values | Default |
|---|---|---|---|
| `preference` (color scheme) | enum `ThemePreference` | `'system'`, `'light'`, `'dark'` | `'system'` |
| `accentColor` | enum `AccentColorId` | `'orange'`, `'emerald'`, `'berry'`, `'golden'`, `'ocean'` | `'orange'` |

Changing one never changes the other — verified by test for the accent→preference
direction (v1: src/app/src-rn/__tests__/store/themeStore.test.ts:32-37); the reverse
follows from `setPreference` writing only `preference` (v1: themeStore.ts:21), not
test-pinned.

### 1.1 Light/dark/system resolution

The effective color scheme is resolved as follows (v1: src/app/src-rn/theme/useTheme.ts:17-20):

- If `preference === 'system'`: use the OS-reported color scheme; **if the OS reports
  null/unknown, fall back to `'light'`**.
- Otherwise use `preference` directly.
- `isDark` is true iff the resolved scheme is `'dark'`.

The theme hook returns `{ colors, isDark, colorScheme }` where `colorScheme` is the resolved
`'light' | 'dark'` value and `colors` is the composed palette from §4
(v1: src/app/src-rn/theme/useTheme.ts:12-27). Every screen/component re-derives its styles
from this palette, so a preference or accent change re-renders the whole UI immediately
(no restart).

For the OS to report the system scheme correctly, the app must declare itself as supporting
both appearances (v1 mechanism: `"userInterfaceStyle": "automatic"` in
src/app/app.json:9).

**v1 quirk (status bar):** the root layout renders `<StatusBar style="auto" />`
(v1: src/app/app/_layout.tsx:104). expo-status-bar's `auto` follows the **system** color
scheme, and v1 never forces the OS-level appearance to match the app preference — so when
`preference` overrides the system (e.g. app forced dark while the system is light), the
status bar keeps the system styling and can mismatch the app background. Reproduce or
consciously fix in v2.

**Dropped in v2**: v1's root layout also mirrors the resolved scheme onto
`document.documentElement.style.colorScheme` on web (v1: src/app/app/_layout.tsx:63) — web
target is dropped; no equivalent needed on iOS/Android.

### 1.2 Persistence

Both settings persist together across app restarts (v1 mechanism: Zustand `persist`
middleware over AsyncStorage, v1: src/app/src-rn/store/themeStore.ts:16-39):

- Storage key: **`theme-preference`** (despite the name, it stores *both* `preference` and
  `accentColor`).
- Serialization (v1 on-disk format, relevant only if v2 migrates v1 installs): JSON of the
  shape `{"state":{"preference":"<value>","accentColor":"<value>"},"version":0}` stored as a
  string under that key in React Native AsyncStorage.
- Writes happen on every change; there is no explicit "save" action.

---

## 2. Base palettes (exact values)

There are two base palettes, `LightColors` and `DarkColors`
(v1: src/app/src-rn/theme/colors.ts:62-160). The `Colors` record has the following keys
(v1: src/app/src-rn/theme/colors.ts:3-60). All values below are copied verbatim.

Several "glass" keys are **platform-conditional**: v1 computes them from
`useFlatStyle = Platform.OS === 'android' || isDesktop()`
(v1: src/app/src-rn/utils/platform.ts:20). Desktop is dropped in v2, so in v2 this reduces
to: **flat = Android, glass = iOS**. Both value sets are given below; tests pin both
variants (v1: src/app/src-rn/__tests__/theme/colors.test.ts:78-106).

The same flat-vs-glass rule drives the shared surface/button style recipes in
v1: src/app/src-rn/theme/platformStyles.ts (each recipe branches Android-flat /
desktop / iOS-glass; the desktop branch is dropped in v2). The recipes' exact values are
specified in `04-ui-ux`; this doc owns the rule and the color tokens they consume. Two of
those recipes (`sidebarSurface`, `panelSurface`, v1: src/app/src-rn/theme/platformStyles.ts:250-283)
style only the wide/desktop layout — **Dropped in v2**.

### 2.1 LightColors (v1: src/app/src-rn/theme/colors.ts:62-110)

| Key | Value |
|---|---|
| `background` | `#F5F5F7` |
| `surface` | `#fff` |
| `surfaceVariant` | `#EDEDF0` |
| `textPrimary` | `#1D1D1F` |
| `textSecondary` | `#6E6E73` |
| `textTertiary` | `#AEAEB2` |
| `primary` | `#D4501A` |
| `primaryDark` | `#B84415` |
| `primarySurface` | `#FFF1EB` |
| `border` | `#D6D6D8` |
| `borderInput` | `#CECED0` |
| `success` | `#34A853` |
| `successSurface` | `#EBF5EE` |
| `successText` | `#1E7E34` |
| `successBorder` | `#34A853` |
| `warning` | `#F5A623` |
| `warningSurface` | `#FEF6E6` |
| `warningText` | `#B57A10` |
| `warningBorder` | `#F5C564` |
| `error` | `#D93025` |
| `errorSurface` | `#FCE8E6` |
| `errorText` | `#B3261E` |
| `overlay` | `rgba(255,255,255,0.7)` |
| `glassSurface` | iOS: `rgba(255,255,255,0.70)` — Android (flat): `#ffffff` |
| `glassSurfaceVariant` | iOS: `rgba(237,237,240,0.60)` — Android: `#EDEDF0` |
| `glassHighlight` | iOS: `rgba(255,255,255,0.85)` — Android: `#D6D6D8` |
| `glassShadowEdge` | iOS: `rgba(0,0,0,0.06)` — Android: `#D6D6D8` |
| `glassSuccess` | iOS: `rgba(52,168,83,0.10)` — Android: `#EBF5EE` |
| `glassWarning` | iOS: `rgba(245,166,35,0.10)` — Android: `#FEF6E6` |
| `glassError` | iOS: `rgba(217,48,37,0.10)` — Android: `#FCE8E6` |
| `glassPrimary` | iOS: `rgba(212,80,26,0.08)` — Android: `#FFF1EB` |
| `blurTint` | `systemThinMaterial` |
| `blurIntensity` | `40` |
| `blurIntensityStrong` | `80` |
| `blurIntensitySubtle` | `25` |

### 2.2 DarkColors (v1: src/app/src-rn/theme/colors.ts:112-160)

| Key | Value |
|---|---|
| `background` | `#000000` |
| `surface` | `#1C1C1E` |
| `surfaceVariant` | `#2C2C2E` |
| `textPrimary` | `#F5F5F7` |
| `textSecondary` | `#A1A1A6` |
| `textTertiary` | `#636366` |
| `primary` | `#FF6B35` |
| `primaryDark` | `#D4501A` |
| `primarySurface` | `#2A1A10` |
| `border` | `#38383A` |
| `borderInput` | `#48484A` |
| `success` | `#34A853` |
| `successSurface` | `#142018` |
| `successText` | `#5DB075` |
| `successBorder` | `#2E7D32` |
| `warning` | `#F5A623` |
| `warningSurface` | `#2A1E0E` |
| `warningText` | `#F5C564` |
| `warningBorder` | `#B57A10` |
| `error` | `#EA4335` |
| `errorSurface` | `#2A1614` |
| `errorText` | `#F28B82` |
| `overlay` | `rgba(0,0,0,0.7)` |
| `glassSurface` | iOS: `rgba(28,28,30,0.72)` — Android (flat): `#1C1C1E` |
| `glassSurfaceVariant` | iOS: `rgba(44,44,46,0.60)` — Android: `#2C2C2E` |
| `glassHighlight` | iOS: `rgba(255,255,255,0.12)` — Android: `#38383A` |
| `glassShadowEdge` | iOS: `rgba(0,0,0,0.40)` — Android: `#000000` |
| `glassSuccess` | iOS: `rgba(52,168,83,0.16)` — Android: `#142018` |
| `glassWarning` | iOS: `rgba(245,166,35,0.16)` — Android: `#2A1E0E` |
| `glassError` | iOS: `rgba(234,67,53,0.16)` — Android: `#2A1614` |
| `glassPrimary` | iOS: `rgba(255,107,53,0.14)` — Android: `#2A1A10` |
| `blurTint` | `systemThickMaterialDark` |
| `blurIntensity` | `50` |
| `blurIntensityStrong` | `90` |
| `blurIntensitySubtle` | `30` |

Note: `blurTint`/`blurIntensity*` configure the iOS translucent-blur surfaces (expo-blur
tint names in v1). Android uses the flat opaque values and no blur. See `04-ui-ux` for
where blur is applied.

---

## 3. Accent color themes (exact values)

Five accent themes, each with a display label (German) and a light + dark variant of four
primary-related keys (v1: src/app/src-rn/theme/colors.ts:162-253). `glassPrimary` is
platform-conditional as in §2 (flat value on Android, rgba on iOS).

### orange — label `Orange` (default)

| Key | Light | Dark |
|---|---|---|
| `primary` | `#D4501A` | `#FF6B35` |
| `primaryDark` | `#B84415` | `#D4501A` |
| `primarySurface` | `#FFF1EB` | `#2A1A10` |
| `glassPrimary` (iOS) | `rgba(212,80,26,0.08)` | `rgba(255,107,53,0.14)` |
| `glassPrimary` (Android) | `#FFF1EB` | `#2A1A10` |

The orange variant is byte-identical to the base palettes' primary values, so selecting
orange reproduces `LightColors`/`DarkColors` exactly (pinned by test,
v1: src/app/src-rn/__tests__/theme/colors.test.ts:30-42).

### emerald — label `Smaragd`

| Key | Light | Dark |
|---|---|---|
| `primary` | `#2E7D4F` | `#4CAF7D` |
| `primaryDark` | `#236B3F` | `#2E7D4F` |
| `primarySurface` | `#E8F5ED` | `#102A1A` |
| `glassPrimary` (iOS) | `rgba(46,125,79,0.08)` | `rgba(76,175,125,0.14)` |
| `glassPrimary` (Android) | `#E8F5ED` | `#102A1A` |

### berry — label `Beere`

| Key | Light | Dark |
|---|---|---|
| `primary` | `#A62547` | `#E04868` |
| `primaryDark` | `#8C1E3B` | `#A62547` |
| `primarySurface` | `#FCEEF2` | `#2A1018` |
| `glassPrimary` (iOS) | `rgba(166,37,71,0.08)` | `rgba(224,72,104,0.14)` |
| `glassPrimary` (Android) | `#FCEEF2` | `#2A1018` |

### golden — label `Gold`

| Key | Light | Dark |
|---|---|---|
| `primary` | `#C08B1A` | `#E8B03E` |
| `primaryDark` | `#A07415` | `#C08B1A` |
| `primarySurface` | `#FDF5E3` | `#2A2210` |
| `glassPrimary` (iOS) | `rgba(192,139,26,0.08)` | `rgba(232,176,62,0.14)` |
| `glassPrimary` (Android) | `#FDF5E3` | `#2A2210` |

### ocean — label `Ozean`

| Key | Light | Dark |
|---|---|---|
| `primary` | `#2563A8` | `#4A90D9` |
| `primaryDark` | `#1E528C` | `#2563A8` |
| `primarySurface` | `#EBF2FC` | `#101A2A` |
| `glassPrimary` (iOS) | `rgba(37,99,168,0.08)` | `rgba(74,144,217,0.14)` |
| `glassPrimary` (Android) | `#EBF2FC` | `#101A2A` |

---

## 4. Palette composition

The effective palette is computed as: take the base palette for the resolved scheme
(`DarkColors` if `isDark` else `LightColors`), then overwrite exactly these four keys with
the selected accent's variant for that scheme: `primary`, `primaryDark`, `primarySurface`,
`glassPrimary` (v1: src/app/src-rn/theme/colors.ts:255-265). **All other keys — including
success/warning/error, surfaces, text, borders, blur config — are never affected by the
accent** (pinned by test, v1: src/app/src-rn/__tests__/theme/colors.test.ts:44-59).

---

## 5. Appearance screen ("Darstellung")

A dedicated screen, pushed from the Settings tab via a navigation row that routes to
`/appearance` (v1: src/app/app/(tabs)/settings.tsx:92; screen registered as a card-style
push with no navigation header, v1: src/app/app/_layout.tsx:102). The Settings row is
titled `Darstellung` with a chevron-forward and a hint line showing the current mode label —
`system` → `System`, `light` → `Hell`, `dark` → `Dunkel`
(v1: src/app/app/(tabs)/settings.tsx:29-33,89-102; row layout owned by
`03-features/settings.md`). Full source: v1: src/app/app/appearance.tsx.

Layout (top to bottom):

1. **Back control**: chevron + text `Einstellungen`, tinted `primary`; navigates back
   (v1: src/app/app/appearance.tsx:41-44).
2. **Page title**: `Darstellung` (28pt, weight 700) (v1: src/app/app/appearance.tsx:46).
3. **Card "Design"** — the scheme preference picker (v1: src/app/app/appearance.tsx:48-77).
   Three equal-width options in a row, each an icon above a label
   (v1: src/app/app/appearance.tsx:18-22):

   | Value | Label | Icon (Ionicons name) |
   |---|---|---|
   | `system` | `System` | `phone-portrait-outline` |
   | `light` | `Hell` | `sunny-outline` |
   | `dark` | `Dunkel` | `moon-outline` |

   Selected option: background `primarySurface` (Android/flat) or `glassPrimary` (iOS);
   on flat platforms the whole border is colored `primary`, on glass platforms only the
   bottom border is `primary` (the rest keeps the banner-surface border); icon and label
   colored `primary`. Unselected icon/label use `textSecondary`
   (v1: src/app/app/appearance.tsx:60-74,162-177). Tapping applies immediately.
4. **Card "Akzentfarbe"** — the accent picker (v1: src/app/app/appearance.tsx:79-110).
   One row of five circular swatches in map order `orange, emerald, berry, golden, ocean`,
   each with its label beneath. Swatch spec (mobile values;
   v1: src/app/app/appearance.tsx:88-107,187-200):
   - Circle 40×40, border radius 20.
   - Fill: the accent's **light-mode** `primary` (always the light value, even in dark mode).
   - Selected: border width 3 in the same light-mode `primary`, plus a white (`#fff`)
     checkmark icon (Ionicons `checkmark`, size 20) centered in the circle.
   - Unselected: 2px transparent border (keeps layout stable).
   - Label: the German label from §3; colored current `primary` when selected, else
     `textTertiary`.
   - Tapping applies the accent immediately **and** triggers the app icon switch (§6).

Both cards use the standard card surface recipe (see `04-ui-ux`). **Dropped in v2**: the
screen has compact-desktop size variants (`isCompactDesktop` paths,
v1: src/app/app/appearance.tsx:12 et passim); only the mobile values above apply.

Discrepancy note (code wins): the design doc places the accent picker inside the Settings
screen with 32px circles (v1: docs/plans/2026-02-21-themes-app-icon-design.md:39-48), and
the implementation plan uses 36px circles and a `✓` text glyph
(v1: docs/plans/2026-02-21-themes-app-icon-impl.md:533-544); shipped code moved it to the
dedicated `/appearance` screen with 40px circles and an Ionicons `checkmark` icon.

---

## 6. Alternate app icons

The home-screen app icon tracks the selected accent color. Five icon variants exist; the
**orange icon is the app's primary/default icon**, the other four are alternates.

### 6.1 Setting-to-icon mapping

On every accent change, after updating state, v1 switches the icon
(v1: src/app/src-rn/store/themeStore.ts:22-32):

- `accentColor === 'orange'` → reset to the primary icon (v1 calls `setAppIcon(null, false)`).
- any other accent → activate the alternate icon with the **same name as the accent id**
  (v1 calls `setAppIcon('<accentId>', false)`; e.g. `setAppIcon('emerald', false)`).

The second argument is `isInBackground: false` (pinned by test,
v1: src/app/src-rn/__tests__/store/themeStore.test.ts:39-52). Discrepancy note (code wins):
the design doc and impl plan show a single-argument call
(v1: docs/plans/2026-02-21-themes-app-icon-design.md:77-86); shipped code passes `false`
explicitly, and v1's patched iOS module ignores the flag anyway (§6.2).

Failures of the icon-switch call are swallowed silently — the accent state change always
succeeds even if icon switching is unavailable
(v1: src/app/src-rn/store/themeStore.ts:25-30). The switch is skipped entirely on web
(`Platform.OS === 'web'` guard, v1: src/app/src-rn/store/themeStore.ts:24) — **Dropped in
v2** (no web target); v2 only needs the iOS/Android paths.

**No startup reconciliation:** the icon switch happens *only* inside `setAccentColor`
(this is the only call site of the icon API in v1 app code). On launch, v1 does NOT read
the persisted accent and re-apply the icon, and `getAppIcon` is never called — so if the
installed icon and the persisted accent ever diverge (e.g. reinstall resets the icon to
default while a restored backup keeps `accentColor` non-orange), they stay divergent until
the user picks an accent again.

### 6.2 v1 mechanism: `@g9k/expo-dynamic-app-icon` 2.0.8 + local patch

v1 uses `@g9k/expo-dynamic-app-icon` `^2.0.8` (v1: src/app/package.json:18) with a
`patch-package` patch applied on postinstall
(v1: src/app/patches/@g9k+expo-dynamic-app-icon+2.0.8.patch; v1: src/app/package.json:14).
v2 replaces this with native platform APIs; the patched v1 behavior to reproduce:

**iOS** (patched module, v1: patches/@g9k+expo-dynamic-app-icon+2.0.8.patch:12-101):
- Alternate icons are registered under the names `AppIcon-emerald`, `AppIcon-berry`,
  `AppIcon-golden`, `AppIcon-ocean` (the module prefixes the JS name with `AppIcon-`).
- If the device does not support alternate icons, resolve `"NO_SUPPORT"` and do nothing.
- Switching calls `UIApplication.setAlternateIconName(_:completionHandler:)` (the public
  API; the pre-patch private "background" API path was removed) with **retry logic**:
  first attempt after a 0.5 s delay on the main queue; on any error retry with the delay
  doubled each time (0.5, 1, 2, 4, 8, 16 s), up to 5 retries (6 attempts total). On success
  resolve `"OK:<currentAlternateIconName or DEFAULT>"`; after exhausting retries resolve
  `"FAIL:<localizedDescription>|domain=<domain> code=<code> retries=0"`. (v1's JS caller
  ignores the resolved value.)
- Passing `nil`/empty name resets to the primary icon.
- `getAppIcon` returns the current alternate icon name with the `AppIcon-` prefix stripped,
  or `"DEFAULT"` (v1: patches/@g9k+expo-dynamic-app-icon+2.0.8.patch:39-43). (Unused by
  v1 app code.)
- UX side effect: iOS shows its standard system alert when the icon changes via the public
  API (v1: docs/plans/2026-02-21-themes-app-icon-design.md:35).
- The patch also narrows the library's TS icon-name type to
  `"emerald" | "berry" | "golden" | "ocean"`
  (v1: patches/@g9k+expo-dynamic-app-icon+2.0.8.patch:5-9).

**Android**: the library's Expo config plugin generates one `activity-alias` per configured
icon and switches by enabling/disabling aliases; the switch is silent (no system alert)
(v1: docs/plans/2026-02-21-themes-app-icon-design.md:36-37). The patch does not modify the
Android implementation; its exact runtime behavior lives in the library, not in v1 source
(see §10 Open questions). In v2 the equivalent is the standard Android activity-alias +
`PackageManager.setComponentEnabledSetting` approach.

### 6.3 Build-time icon configuration (v1: src/app/app.json)

- Primary app icon: `./assets/icons/icon-orange.png` (v1: src/app/app.json:8).
- Android adaptive icon: foreground `./assets/icons/adaptive-icon-orange.png`, background
  color `#F0F0F2` (v1: src/app/app.json:33-36).
- Alternate icons registered via the `@g9k/expo-dynamic-app-icon` plugin config
  (v1: src/app/app.json:66-97) — for each of `emerald`, `berry`, `golden`, `ocean`:
  - iOS asset: `./assets/icons/icon-<name>.png`
  - Android: `foregroundImage: ./assets/icons/adaptive-icon-<name>.png`,
    `backgroundColor: "#F0F0F2"`
- `orange` is deliberately **not** in the alternate-icon list; it is the default
  (v1: docs/plans/2026-02-21-themes-app-icon-impl.md:822).
- Cross-reference: the Android notification small icon is configured from
  `./assets/icons/icon-orange.png` with accent color `#FF6B35`
  (v1: src/app/app.json:58-64); see the notifications docs.

---

## 7. Icon asset inventory & design spec

### 7.1 Inventory (v1: src/app/assets/)

All icon PNGs are 1024×1024. Each PNG has a matching source SVG (PNGs are rendered from the
SVGs at 1024×1024; generator described in
v1: docs/plans/2026-02-21-themes-app-icon-impl.md Task 6). Binary assets are copied
verbatim from v1 `main` during v2 implementation (see appendix-source-map.md).

| Asset | Purpose |
|---|---|
| `assets/icons/icon-{orange,emerald,berry,golden,ocean}.png` (+ `.svg`) | Full iOS-style icons (background + glyph), one per accent |
| `assets/icons/adaptive-icon-{orange,emerald,berry,golden,ocean}.png` (+ `.svg`) | Android adaptive-icon foregrounds (glyph only, transparent background), one per accent |
| `assets/icon.png` / `assets/icon.svg` | Copy of the orange full icon (default) |
| `assets/adaptive-icon.png` / `assets/adaptive-icon.svg` | Copy of the orange adaptive foreground |
| `assets/splash-icon.png` (512×512) / `assets/splash-icon.svg` | Splash image; splash config: `resizeMode: "contain"`, background `#ffffff` (v1: src/app/app.json:11-15) — see `07-release` |
| `assets/favicon.png` (48×48) | Web favicon — **Dropped in v2** |

### 7.2 Icon design (per-accent, from the SVG sources)

Full icon (`icon-<accent>.svg`, e.g. v1: src/app/assets/icons/icon-orange.svg):
- 1024×1024, rounded square `rx=224`.
- Background: diagonal linear gradient `#FFFFFF` → `#F0F0F2` (top-left → bottom-right).
  Same for all accents.
- Two subtle "plate" circles centered at (512,512): r=320 stroked in the accent's light
  `primary` at opacity 0.08, width 6; r=260 at opacity 0.05, width 3.
- Glyph: crossed fork (rotated −30° about center) and knife (rotated +30° about center),
  filled with a diagonal gradient from the accent's **light** `primary` to its **light**
  `primaryDark` (e.g. orange: `#D4501A` → `#B84415`).

Adaptive foreground (`adaptive-icon-<accent>.svg`, e.g.
v1: src/app/assets/icons/adaptive-icon-orange.svg): the same crossed fork-and-knife glyph
and accent gradient only — no background rect, no plate circles (the `#F0F0F2` background
comes from the adaptive-icon `backgroundColor` in §6.3).

Discrepancy note (code/assets win): the design doc describes a "system-adaptive"
background with iOS 18+ light/dark/tinted variants
(v1: docs/plans/2026-02-21-themes-app-icon-design.md:27). The shipped assets have a fixed
light gradient background and no dark/tinted variants.

---

## 8. Dropped in v2 (summary)

- Web `colorScheme` DOM sync (v1: src/app/app/_layout.tsx:60-65).
- `Platform.OS === 'web'` guard around icon switching (v1: src/app/src-rn/store/themeStore.ts:24).
- Desktop/compact-desktop sizing variants on the Appearance screen and the
  `useFlatStyle`-includes-desktop rule — in v2, flat styling applies to Android only
  (v1: src/app/src-rn/utils/platform.ts:20-23).
- Web favicon asset.
- Desktop (Tauri) icon switching was already out of scope in v1
  (v1: docs/plans/2026-02-21-themes-app-icon-design.md:106-109).

## 9. Behavior checklist (from v1 tests, except where marked source-derived)

From v1: src/app/src-rn/__tests__/store/themeStore.test.ts and
v1: src/app/src-rn/__tests__/theme/colors.test.ts:

- Default state is `preference='system'`, `accentColor='orange'` (the `accentColor`
  default is test-asserted, themeStore.test.ts:15-17; the `preference` default is
  source-derived — the test file force-sets state in `beforeEach`, themeStore.ts:19-21).
- Setting any of the 5 accents updates state to that accent.
- Changing accent never changes `preference` (test-pinned, themeStore.test.ts:32-37); the
  reverse — `setPreference` never changes `accentColor` — is source-derived
  (themeStore.ts:21), not test-pinned.
- Switching to `orange` resets the app icon to the default; switching to any other accent
  activates the alternate icon named after the accent.
- `orange` composition returns the base palettes unchanged; non-orange accents override
  exactly `primary`, `primaryDark`, `primarySurface`, `glassPrimary` and nothing else.
- Every accent's `primary`/`primaryDark`/`primarySurface` (light and dark) is a 6-digit hex
  color.
- Flat (Android) palettes use the opaque hex glass values; iOS palettes use the rgba glass
  values (exact pairs pinned in §2/§3 tables).

## 10. Open questions

- **Android alternate-icon runtime behavior of `@g9k/expo-dynamic-app-icon` 2.0.8** is not
  determinable from v1 source: the v1 patch
  (`src/app/patches/@g9k+expo-dynamic-app-icon+2.0.8.patch`) modifies only `types.d.ts` and
  the iOS Swift module, never the Android implementation. Specifics a v2 Android
  implementation must decide/verify: the exact `activity-alias` names the config plugin
  generates, the enable/disable sequencing, and whether switching while the app is
  foregrounded kills/relaunches the task (the common Android caveat). See §6.2.
- Whether the checked-in 1024×1024 icon PNGs still byte-match a fresh render of their SVGs
  (PNG mtimes are newer than the SVGs); v2 treats the PNGs as the authoritative binaries.
