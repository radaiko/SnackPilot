import { ACCENT_COLORS, getColorsForAccent, AccentColorId, LightColors, DarkColors } from '../../theme/colors';

describe('ACCENT_COLORS', () => {
  it('has exactly 5 accent color entries', () => {
    expect(Object.keys(ACCENT_COLORS)).toHaveLength(5);
  });

  it('contains orange, emerald, berry, golden, ocean', () => {
    expect(ACCENT_COLORS).toHaveProperty('orange');
    expect(ACCENT_COLORS).toHaveProperty('emerald');
    expect(ACCENT_COLORS).toHaveProperty('berry');
    expect(ACCENT_COLORS).toHaveProperty('golden');
    expect(ACCENT_COLORS).toHaveProperty('ocean');
  });

  it('each accent has light and dark primary colors', () => {
    for (const id of Object.keys(ACCENT_COLORS) as AccentColorId[]) {
      const accent = ACCENT_COLORS[id];
      expect(accent.light.primary).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(accent.dark.primary).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(accent.light.primaryDark).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(accent.dark.primaryDark).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(accent.light.primarySurface).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(accent.dark.primarySurface).toMatch(/^#[0-9A-Fa-f]{6}$/);
    }
  });
});

describe('getColorsForAccent', () => {
  it('returns base LightColors for orange in light mode', () => {
    const colors = getColorsForAccent('orange', false);
    expect(colors.primary).toBe(LightColors.primary);
    expect(colors.primaryDark).toBe(LightColors.primaryDark);
    expect(colors.background).toBe(LightColors.background);
  });

  it('returns base DarkColors for orange in dark mode', () => {
    const colors = getColorsForAccent('orange', true);
    expect(colors.primary).toBe(DarkColors.primary);
    expect(colors.primaryDark).toBe(DarkColors.primaryDark);
    expect(colors.background).toBe(DarkColors.background);
  });

  it('overrides primary colors for emerald in light mode', () => {
    const colors = getColorsForAccent('emerald', false);
    expect(colors.primary).toBe(ACCENT_COLORS.emerald.light.primary);
    expect(colors.primaryDark).toBe(ACCENT_COLORS.emerald.light.primaryDark);
    expect(colors.primarySurface).toBe(ACCENT_COLORS.emerald.light.primarySurface);
    expect(colors.background).toBe(LightColors.background);
    expect(colors.success).toBe(LightColors.success);
  });

  it('overrides primary colors for berry in dark mode', () => {
    const colors = getColorsForAccent('berry', true);
    expect(colors.primary).toBe(ACCENT_COLORS.berry.dark.primary);
    expect(colors.primaryDark).toBe(ACCENT_COLORS.berry.dark.primaryDark);
    expect(colors.primarySurface).toBe(ACCENT_COLORS.berry.dark.primarySurface);
    expect(colors.background).toBe(DarkColors.background);
  });

  it('overrides glassPrimary for non-orange accents', () => {
    const colors = getColorsForAccent('ocean', false);
    expect(colors.glassPrimary).toBe(ACCENT_COLORS.ocean.light.glassPrimary);
    expect(colors.glassPrimary).not.toBe(LightColors.glassPrimary);
  });

  it('returns correct colors for all 5 accents', () => {
    const accents: AccentColorId[] = ['orange', 'emerald', 'berry', 'golden', 'ocean'];
    for (const accent of accents) {
      const light = getColorsForAccent(accent, false);
      const dark = getColorsForAccent(accent, true);
      expect(light.primary).toBe(ACCENT_COLORS[accent].light.primary);
      expect(dark.primary).toBe(ACCENT_COLORS[accent].dark.primary);
    }
  });
});

describe('flat style variants', () => {
  function loadColors(flat: boolean) {
    jest.resetModules();
    jest.doMock('../../utils/platform', () => ({ useFlatStyle: flat }));
    const mod = require('../../theme/colors');
    jest.dontMock('../../utils/platform');
    return mod as typeof import('../../theme/colors');
  }

  it('uses opaque glass colors when flat style is active (Android/desktop)', () => {
    const { LightColors: light, DarkColors: dark, ACCENT_COLORS: accents } = loadColors(true);
    expect(light.glassSurface).toBe('#ffffff');
    expect(light.glassPrimary).toBe('#FFF1EB');
    expect(dark.glassSurface).toBe('#1C1C1E');
    expect(dark.glassPrimary).toBe('#2A1A10');
    expect(accents.ocean.light.glassPrimary).toBe('#EBF2FC');
    expect(accents.ocean.dark.glassPrimary).toBe('#101A2A');
  });

  it('uses translucent rgba glass colors when flat style is off (iOS)', () => {
    const { LightColors: light, DarkColors: dark, ACCENT_COLORS: accents } = loadColors(false);
    expect(light.glassSurface).toBe('rgba(255,255,255,0.70)');
    expect(light.glassPrimary).toBe('rgba(212,80,26,0.08)');
    expect(dark.glassSurface).toBe('rgba(28,28,30,0.72)');
    expect(dark.glassPrimary).toBe('rgba(255,107,53,0.14)');
    expect(accents.ocean.light.glassPrimary).toBe('rgba(37,99,168,0.08)');
    expect(accents.ocean.dark.glassPrimary).toBe('rgba(74,144,217,0.14)');
  });
});
