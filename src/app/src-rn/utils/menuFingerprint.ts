import type { GourmetMenuItem } from '../types/menu';
import { localDateKey } from './dateUtils';

/**
 * Compute a fingerprint map from menu items.
 * Key: "menuId|dateKey" composite, Value: "title|subtitle|allergens" string.
 * Uses composite key because menu IDs are per-category, not per-item —
 * the same ID appears on different days with different content.
 */
export function computeFingerprints(items: GourmetMenuItem[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const item of items) {
    const key = `${item.id}|${localDateKey(item.day)}`;
    map.set(key, `${item.title}|${item.subtitle}|${item.allergens.join(',')}`);
  }
  return map;
}

/**
 * Detect whether any menu in `current` is new or changed compared to `known`.
 * Returns true if there's at least one new ID or changed fingerprint.
 */
export function detectNewMenus(
  current: Map<string, string>,
  known: Map<string, string>,
): boolean {
  if (current.size === 0) return false;
  if (known.size === 0) return true;

  for (const [id, fingerprint] of current) {
    const knownFp = known.get(id);
    if (knownFp === undefined || knownFp !== fingerprint) {
      return true;
    }
  }
  return false;
}

/** Serialize a fingerprint map to JSON for AsyncStorage. */
export function serializeKnownMenus(map: Map<string, string>): string {
  return JSON.stringify(Array.from(map.entries()));
}

/** Deserialize a fingerprint map from AsyncStorage JSON. */
export function deserializeKnownMenus(json: string | null): Map<string, string> {
  if (!json) return new Map();
  try {
    return new Map(JSON.parse(json));
  } catch {
    return new Map();
  }
}
