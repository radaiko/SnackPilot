import {
  computeFingerprints,
  detectNewMenus,
  serializeKnownMenus,
  deserializeKnownMenus,
} from '../../utils/menuFingerprint';
import { GourmetMenuCategory } from '../../types/menu';
import type { GourmetMenuItem } from '../../types/menu';

function makeItem(overrides: Partial<GourmetMenuItem> = {}): GourmetMenuItem {
  return {
    id: 'menu-001',
    day: new Date(2026, 1, 10),
    title: 'MENU I',
    subtitle: 'Schnitzel mit Reis',
    allergens: ['A', 'G'],
    available: true,
    ordered: false,
    category: GourmetMenuCategory.Menu1,
    price: '',
    ...overrides,
  };
}

describe('menuFingerprint', () => {
  describe('computeFingerprints', () => {
    it('computes fingerprint from title, subtitle, allergens', () => {
      const items = [makeItem()];
      const fp = computeFingerprints(items);
      expect(fp.size).toBe(1);
      expect(fp.get('menu-001')).toBe('MENU I|Schnitzel mit Reis|A,G');
    });

    it('uses unique menu IDs as keys', () => {
      const items = [
        makeItem({ id: 'a' }),
        makeItem({ id: 'b', title: 'MENU II' }),
      ];
      const fp = computeFingerprints(items);
      expect(fp.size).toBe(2);
      expect(fp.has('a')).toBe(true);
      expect(fp.has('b')).toBe(true);
    });

    it('deduplicates same ID (last wins)', () => {
      const items = [
        makeItem({ id: 'a', subtitle: 'Mon' }),
        makeItem({ id: 'a', subtitle: 'Tue' }),
      ];
      const fp = computeFingerprints(items);
      expect(fp.size).toBe(1);
      expect(fp.get('a')).toBe('MENU I|Tue|A,G');
    });
  });

  describe('detectNewMenus', () => {
    it('returns true when known map is empty', () => {
      const current = new Map([['a', 'fp1']]);
      expect(detectNewMenus(current, new Map())).toBe(true);
    });

    it('returns false when fingerprints match', () => {
      const known = new Map([['a', 'fp1'], ['b', 'fp2']]);
      const current = new Map([['a', 'fp1'], ['b', 'fp2']]);
      expect(detectNewMenus(current, known)).toBe(false);
    });

    it('returns true when a new ID appears', () => {
      const known = new Map([['a', 'fp1']]);
      const current = new Map([['a', 'fp1'], ['b', 'fp2']]);
      expect(detectNewMenus(current, known)).toBe(true);
    });

    it('returns true when fingerprint changes for existing ID', () => {
      const known = new Map([['a', 'fp1']]);
      const current = new Map([['a', 'fp2']]);
      expect(detectNewMenus(current, known)).toBe(true);
    });

    it('returns false when menus are removed but remaining unchanged', () => {
      const known = new Map([['a', 'fp1'], ['b', 'fp2']]);
      const current = new Map([['a', 'fp1']]);
      expect(detectNewMenus(current, known)).toBe(false);
    });

    it('returns false when both are empty', () => {
      expect(detectNewMenus(new Map(), new Map())).toBe(false);
    });
  });

  describe('serialization', () => {
    it('round-trips through JSON', () => {
      const original = new Map([['a', 'fp1'], ['b', 'fp2']]);
      const json = serializeKnownMenus(original);
      const restored = deserializeKnownMenus(json);
      expect(restored).toEqual(original);
    });

    it('handles empty map', () => {
      const json = serializeKnownMenus(new Map());
      expect(deserializeKnownMenus(json)).toEqual(new Map());
    });

    it('returns empty map for invalid JSON', () => {
      expect(deserializeKnownMenus('not json')).toEqual(new Map());
    });

    it('returns empty map for null', () => {
      expect(deserializeKnownMenus(null)).toEqual(new Map());
    });
  });
});
