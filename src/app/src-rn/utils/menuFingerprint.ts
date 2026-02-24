import type { GourmetMenuItem } from '../types/menu';

/**
 * Compute a fingerprint map from menu items.
 * Key: menu ID, Value: "title|subtitle|allergens" string.
 * If multiple items share the same ID (same category, different days), last wins.
 */
export function computeFingerprints(items: GourmetMenuItem[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const item of items) {
    map.set(item.id, `${item.title}|${item.subtitle}|${item.allergens.join(',')}`);
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
