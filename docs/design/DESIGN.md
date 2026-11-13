# SnackPilot design system

One flat, highly usable system for iOS, Android and desktop. Desktop uses the
same components at wider widths; there is no per-platform dialect.

Implementation homes:

| Layer | File |
| --- | --- |
| Colour, spacing, radius, target and type tokens | `src/ui/model/src/tokens.rs` |
| gpui colours, recipes and scale accessors | `src/ui/app/src/theme.rs` |
| Shared components | `src/ui/app/src/components/*` |
| Shell | `src/ui/app/src/app.rs`, `src/ui/app/src/lib.rs` |
| Contrast helpers | `src/ui/model/src/desktop.rs` |

## Colour

One palette, light / dark:

| Token | Light | Dark |
| --- | --- | --- |
| `BG` | `#F4F5F7` | `#121316` |
| `SURFACE` | `#FFFFFF` | `#1B1D21` |
| `SURFACE_ALT` | `#EDEFF3` | `#24272C` |
| `BORDER` | `#DDE0E6` | `#33373D` |
| `BORDER_STRONG` | `#C3C8D0` | `#454A52` |
| `TEXT` | `#14161A` | `#F2F4F7` |
| `TEXT_SECONDARY` | `#5B6270` | `#A7AEBA` |
| `TEXT_TERTIARY` | `#7C8492` | `#6E7683` |
| `ACCENT` | `#C2450F` | `#FF7A45` |
| `ACCENT_SURFACE` | `#FDEDE6` | `#3A1E12` |
| `ON_ACCENT` | `#FFFFFF` | `#1A0D06` |
| `SUCCESS` | `#1E7D43` | `#4CC38A` |
| `SUCCESS_SURFACE` | `#E6F4EB` | `#16301F` |
| `WARNING` | `#A35C00` | `#F0A93B` |
| `WARNING_SURFACE` | `#FBF0DF` | `#33260E` |
| `ERROR` | `#B3261E` | `#F27B6B` |
| `ERROR_SURFACE` | `#FBEAE8` | `#351A17` |
| `SCRIM` | `rgba(0,0,0,0.4)` | same |

The accent is fixed; it is not selectable. `System / Hell / Dunkel` remains the
only appearance preference.

## Scale

- Space: 4, 8, 12, 16, 24, 32.
- Radius: card 12, control 10, chip 8.
- Targets: 44 minimum, 48 primary action.
- Icons: 20, meta 18, stroke 1.5.
- Focus: 2 px ring at 2 px offset.
- Content: 560 max, credential form 440 max.

## Type

Inter, weights 400 and 600 only.

| Role | Size / line |
| --- | --- |
| `TITLE` | 28 / 34 semibold |
| `HEADLINE` | 20 / 26 semibold |
| `BODY` | 17 / 24 regular |
| `BODY_STRONG` | 17 / 24 semibold |
| `META` | 13 / 18 regular |

## Components

- `card`: surface, 12 radius, 16 padding, 1 px border, no shadow.
- `button`: primary filled accent, secondary outlined, quiet accent text,
  destructive error; 48 high, 10 radius.
- `chip` / `time_chips`: 8 radius, 44 high; selected uses the accent surface
  with an accent boundary and a check, never colour alone.
- `text_field`: 48 high, 10 radius, label above, 1 px strong border, 2 px accent
  focus ring at 2 px offset.
- `switch`: accent track; 44 row with its label.
- `banner`: surface or toned semantic surface at 12 radius.
- `dialog`: surface, 12 radius, 40 % scrim, 48 px actions.
- `toast`: accent surface, 12 radius, body/meta copy.
- `tab_bar`: 56 high, hairline top, icon + label; active is the accent icon with
  a semibold label.
- `segment_tabs`: equal targets with a reserved selected rule.
- `date navigator`: contained surface, containing boundary, 48 px targets.

## Layout

- Compact shell: title at 16 px gutters, one centred column capped at 560,
  safe top applied once, bottom bar 56 in flow with the safe-area strip, content
  bottom inset equal to the navigation height.
- Desktop: a 240 px rail on `BG` closed by a hairline `BORDER`; brand 56,
  section labels `META` semibold `TEXT_SECONDARY`, tab rows 40 (`BODY`) with a
  reserved 3 px accent rule, active `SURFACE_ALT` with accent ink. The
  workspace is `SURFACE` on `BG`: 56 px header, a 1 px `BORDER` master/detail
  split (compact master 248), table headers `META` semibold `TEXT_SECONDARY`,
  rows 40 with `BODY`/`META` cells, hover a 4 % `TEXT` mix, selected
  `ACCENT_SURFACE` with accent ink and the 3 px rule, focus a 2 px `ACCENT`
  ring. One rail width and one row height at every desktop size.
- Ordering and billing behaviour is independent of these presentation tokens.
