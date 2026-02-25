# New Menu Notifications — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Notify the user on iOS/Android when new or changed menus appear, via background fetch + local push notification, with an in-app toast fallback for foreground detection.

**Architecture:** A shared fingerprint-comparison module (`menuFingerprint.ts`) is used by both a background fetch task and the foreground Menus screen. Background: `expo-background-fetch` + `expo-task-manager` periodically wake the app headlessly, login, scrape menus, compare fingerprints, and fire a local notification via `expo-notifications`. Foreground: after each menu fetch on the Menus tab, the same comparison runs and shows a toast if new menus are detected. Opening the Menus tab acknowledges all current menus.

**Tech Stack:** `expo-notifications`, `expo-background-fetch`, `expo-task-manager`, AsyncStorage, existing `GourmetApi`

**Design doc:** `docs/plans/2026-02-24-new-menu-notifications-design.md`

---

### Task 1: Install dependencies and configure plugins

**Files:**
- Modify: `src/app/package.json`
- Modify: `src/app/app.json`

**Step 1: Install expo packages**

Run:
```bash
cd src/app && npx expo install expo-notifications expo-background-fetch expo-task-manager
```

**Step 2: Add plugins to `app.json`**

In `src/app/app.json`, add the three plugins to the `plugins` array (after `"expo-font"`):

```json
"expo-background-fetch",
"expo-task-manager",
[
  "expo-notifications",
  {
    "icon": "./assets/icons/icon-orange.png",
    "color": "#FF6B35"
  }
]
```

**Step 3: Verify install**

Run:
```bash
cd src/app && npx expo config --type public | grep -E "notifications|background-fetch|task-manager"
```
Expected: All three plugins listed.

**Step 4: Commit**

```bash
git add src/app/package.json src/app/app.json src/app/package-lock.json
git commit -m "chore: add expo-notifications, background-fetch, task-manager deps (#13)"
```

---

### Task 2: Menu fingerprint module (TDD)

This pure module computes and compares menu fingerprints. No Expo/RN dependencies — fully testable with Jest.

**Files:**
- Create: `src/app/src-rn/utils/menuFingerprint.ts`
- Create: `src/app/src-rn/__tests__/utils/menuFingerprint.test.ts`

**Step 1: Write failing tests**

Create `src/app/src-rn/__tests__/utils/menuFingerprint.test.ts`:

```typescript
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
```

**Step 2: Run tests to verify they fail**

Run: `cd src/app && npx jest --testPathPattern="menuFingerprint" --verbose`
Expected: FAIL — module not found.

**Step 3: Implement the module**

Create `src/app/src-rn/utils/menuFingerprint.ts`:

```typescript
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
```

**Step 4: Run tests to verify they pass**

Run: `cd src/app && npx jest --testPathPattern="menuFingerprint" --verbose`
Expected: ALL PASS

**Step 5: Run full test suite**

Run: `cd src/app && npm test`
Expected: ALL PASS (178+ tests)

**Step 6: Commit**

```bash
git add src/app/src-rn/utils/menuFingerprint.ts src/app/src-rn/__tests__/utils/menuFingerprint.test.ts
git commit -m "feat: add menu fingerprint detection module with tests (#13)"
```

---

### Task 3: AsyncStorage helpers for known menus state (TDD)

Persistence layer for the known-menus map and notification-sent flag.

**Files:**
- Create: `src/app/src-rn/utils/menuChangeStorage.ts`
- Create: `src/app/src-rn/__tests__/utils/menuChangeStorage.test.ts`

**Step 1: Write failing tests**

Create `src/app/src-rn/__tests__/utils/menuChangeStorage.test.ts`:

```typescript
jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach(k => delete store[k]); return Promise.resolve(); }),
    },
  };
});

import {
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from '../../utils/menuChangeStorage';

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
});

describe('menuChangeStorage', () => {
  describe('knownMenus', () => {
    it('returns empty map when nothing stored', async () => {
      const result = await getKnownMenus();
      expect(result).toEqual(new Map());
    });

    it('persists and retrieves a map', async () => {
      const map = new Map([['a', 'fp1'], ['b', 'fp2']]);
      await setKnownMenus(map);
      const result = await getKnownMenus();
      expect(result).toEqual(map);
    });
  });

  describe('notificationSent', () => {
    it('returns false when nothing stored', async () => {
      expect(await getNotificationSent()).toBe(false);
    });

    it('persists true', async () => {
      await setNotificationSent(true);
      expect(await getNotificationSent()).toBe(true);
    });

    it('persists false after true', async () => {
      await setNotificationSent(true);
      await setNotificationSent(false);
      expect(await getNotificationSent()).toBe(false);
    });
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd src/app && npx jest --testPathPattern="menuChangeStorage" --verbose`
Expected: FAIL — module not found.

**Step 3: Implement**

Create `src/app/src-rn/utils/menuChangeStorage.ts`:

```typescript
import AsyncStorage from '@react-native-async-storage/async-storage';
import { serializeKnownMenus, deserializeKnownMenus } from './menuFingerprint';

const KNOWN_MENUS_KEY = 'known_menu_fingerprints';
const NOTIFICATION_SENT_KEY = 'menu_notification_sent';

export async function getKnownMenus(): Promise<Map<string, string>> {
  const json = await AsyncStorage.getItem(KNOWN_MENUS_KEY);
  return deserializeKnownMenus(json);
}

export async function setKnownMenus(map: Map<string, string>): Promise<void> {
  await AsyncStorage.setItem(KNOWN_MENUS_KEY, serializeKnownMenus(map));
}

export async function getNotificationSent(): Promise<boolean> {
  const value = await AsyncStorage.getItem(NOTIFICATION_SENT_KEY);
  return value === 'true';
}

export async function setNotificationSent(sent: boolean): Promise<void> {
  await AsyncStorage.setItem(NOTIFICATION_SENT_KEY, String(sent));
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src/app && npx jest --testPathPattern="menuChangeStorage" --verbose`
Expected: ALL PASS

**Step 5: Run full test suite**

Run: `cd src/app && npm test`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add src/app/src-rn/utils/menuChangeStorage.ts src/app/src-rn/__tests__/utils/menuChangeStorage.test.ts
git commit -m "feat: add AsyncStorage helpers for menu change tracking (#13)"
```

---

### Task 4: Background menu check module

This module defines the background fetch task and notification logic. It runs headlessly (no React, no hooks). It must be registered at module load time.

**Files:**
- Create: `src/app/src-rn/utils/backgroundMenuCheck.ts`

**Step 1: Create the background menu check module**

Create `src/app/src-rn/utils/backgroundMenuCheck.ts`:

```typescript
import * as BackgroundFetch from 'expo-background-fetch';
import * as TaskManager from 'expo-task-manager';
import * as Notifications from 'expo-notifications';
import { Platform } from 'react-native';
import { GourmetApi } from '../api/gourmetApi';
import * as secureStorage from './secureStorage';
import { computeFingerprints, detectNewMenus } from './menuFingerprint';
import {
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from './menuChangeStorage';

const TASK_NAME = 'BACKGROUND_MENU_CHECK';

const CREDENTIALS_KEY_USER = 'gourmet_username';
const CREDENTIALS_KEY_PASS = 'gourmet_password';

// Android notification channel
if (Platform.OS === 'android') {
  Notifications.setNotificationChannelAsync('menu-updates', {
    name: 'Neue Menüs',
    importance: Notifications.AndroidImportance.DEFAULT,
  });
}

/**
 * The background task. Runs headlessly — no React, no hooks.
 * Logs in, fetches menus, compares fingerprints, fires notification if new.
 */
async function backgroundMenuCheckTask(): Promise<BackgroundFetch.BackgroundFetchResult> {
  try {
    // Read credentials
    const username = await secureStorage.getItem(CREDENTIALS_KEY_USER);
    const password = await secureStorage.getItem(CREDENTIALS_KEY_PASS);
    if (!username || !password) {
      return BackgroundFetch.BackgroundFetchResult.NoData;
    }

    // Check if we already sent a notification for the current batch
    const alreadySent = await getNotificationSent();
    if (alreadySent) {
      return BackgroundFetch.BackgroundFetchResult.NoData;
    }

    // Login and fetch menus
    const api = new GourmetApi();
    await api.login(username, password);
    const items = await api.getMenus();

    // Compare fingerprints
    const currentFingerprints = computeFingerprints(items);
    const knownMenus = await getKnownMenus();
    const hasNew = detectNewMenus(currentFingerprints, knownMenus);

    if (hasNew) {
      await Notifications.scheduleNotificationAsync({
        content: {
          title: 'Neue Menüs verfügbar',
          body: 'Es gibt neue Menüs. Öffne SnackPilot um sie anzusehen.',
          data: { screen: '/(tabs)' },
          ...(Platform.OS === 'android' ? { channelId: 'menu-updates' } : {}),
        },
        trigger: null, // Fire immediately
      });
      await setNotificationSent(true);
      return BackgroundFetch.BackgroundFetchResult.NewData;
    }

    return BackgroundFetch.BackgroundFetchResult.NoData;
  } catch {
    return BackgroundFetch.BackgroundFetchResult.Failed;
  }
}

// Register the task at module load time (required by expo-task-manager)
TaskManager.defineTask(TASK_NAME, async () => {
  return backgroundMenuCheckTask();
});

/**
 * Register the background fetch task with the OS.
 * Call once from the root layout after login.
 * Safe to call multiple times — re-registration is a no-op if already registered.
 */
export async function registerBackgroundMenuCheck(): Promise<void> {
  if (Platform.OS === 'web') return;

  const isRegistered = await TaskManager.isTaskRegisteredAsync(TASK_NAME);
  if (isRegistered) return;

  await BackgroundFetch.registerTaskAsync(TASK_NAME, {
    minimumInterval: 15 * 60, // 15 minutes (OS may choose longer)
    stopOnTerminate: false,
    startOnBoot: true,
  });
}

/**
 * Request notification permissions. Call once after first login.
 * Returns true if permissions granted.
 */
export async function requestNotificationPermissions(): Promise<boolean> {
  if (Platform.OS === 'web') return false;

  const { status: existing } = await Notifications.getPermissionsAsync();
  if (existing === 'granted') return true;

  const { status } = await Notifications.requestPermissionsAsync();
  return status === 'granted';
}
```

**Step 2: Verify no type errors**

Run: `cd src/app && npx tsc --noEmit --pretty 2>&1 | head -20`
Expected: No errors related to `backgroundMenuCheck.ts` (there may be pre-existing warnings).

**Step 3: Run full test suite (ensure no side effects)**

Run: `cd src/app && npm test`
Expected: ALL PASS (background module isn't imported in tests yet, so no impact)

**Step 4: Commit**

```bash
git add src/app/src-rn/utils/backgroundMenuCheck.ts
git commit -m "feat: add background menu check task with notifications (#13)"
```

---

### Task 5: NewMenuToast component

A simple animated toast that slides in from the top and auto-dismisses.

**Files:**
- Create: `src/app/src-rn/components/NewMenuToast.tsx`

**Step 1: Create the toast component**

Create `src/app/src-rn/components/NewMenuToast.tsx`:

```tsx
import { useEffect, useRef } from 'react';
import { Animated, StyleSheet, Text } from 'react-native';
import { useTheme } from '../theme/useTheme';
import { tintedBanner } from '../theme/platformStyles';
import type { Colors } from '../theme/colors';

interface NewMenuToastProps {
  visible: boolean;
  onDismiss: () => void;
}

const DISPLAY_DURATION = 4000;
const ANIMATION_DURATION = 300;

export function NewMenuToast({ visible, onDismiss }: NewMenuToastProps) {
  const { colors } = useTheme();
  const translateY = useRef(new Animated.Value(-100)).current;
  const opacity = useRef(new Animated.Value(0)).current;

  useEffect(() => {
    if (visible) {
      Animated.parallel([
        Animated.timing(translateY, {
          toValue: 0,
          duration: ANIMATION_DURATION,
          useNativeDriver: true,
        }),
        Animated.timing(opacity, {
          toValue: 1,
          duration: ANIMATION_DURATION,
          useNativeDriver: true,
        }),
      ]).start();

      const timer = setTimeout(() => {
        Animated.parallel([
          Animated.timing(translateY, {
            toValue: -100,
            duration: ANIMATION_DURATION,
            useNativeDriver: true,
          }),
          Animated.timing(opacity, {
            toValue: 0,
            duration: ANIMATION_DURATION,
            useNativeDriver: true,
          }),
        ]).start(() => onDismiss());
      }, DISPLAY_DURATION);

      return () => clearTimeout(timer);
    }
  }, [visible, translateY, opacity, onDismiss]);

  if (!visible) return null;

  const styles = createStyles(colors);

  return (
    <Animated.View style={[styles.container, { transform: [{ translateY }], opacity }]}>
      <Text style={styles.text}>Neue Menüs verfügbar!</Text>
    </Animated.View>
  );
}

const createStyles = (c: Colors) =>
  StyleSheet.create({
    container: {
      position: 'absolute',
      top: 0,
      left: 16,
      right: 16,
      zIndex: 100,
      padding: 12,
      alignItems: 'center',
      ...tintedBanner(c, c.glassPrimary),
    },
    text: {
      fontSize: 14,
      fontWeight: '600',
      color: c.primary,
    },
  });
```

**Step 2: Verify no type errors**

Run: `cd src/app && npx tsc --noEmit --pretty 2>&1 | head -20`
Expected: No errors related to `NewMenuToast.tsx`.

**Step 3: Commit**

```bash
git add src/app/src-rn/components/NewMenuToast.tsx
git commit -m "feat: add NewMenuToast component (#13)"
```

---

### Task 6: Wire up root layout — registration and permissions

**Files:**
- Modify: `src/app/app/_layout.tsx`

**Step 1: Add imports**

At the top of `src/app/app/_layout.tsx`, add after the existing imports:

```typescript
import { isNative } from '../src-rn/utils/platform';
```

Conditionally import the background module only on native (it uses expo-task-manager which is native-only):

```typescript
const backgroundMenuCheck = isNative()
  ? require('../src-rn/utils/backgroundMenuCheck')
  : null;
```

**Step 2: Add registration effect**

Inside the `AppContent` component, after the existing `useEffect` that calls `loginWithSaved`, add:

```typescript
useEffect(() => {
  if (!backgroundMenuCheck) return;
  backgroundMenuCheck.registerBackgroundMenuCheck();
  backgroundMenuCheck.requestNotificationPermissions();
}, []);
```

**Step 3: Verify the app still runs**

Run: `cd src/app && npm test`
Expected: ALL PASS (the require is conditional, tests run in a jsdom/node env where `isNative()` returns false, so the native module won't be loaded)

**Step 4: Commit**

```bash
git add src/app/app/_layout.tsx
git commit -m "feat: register background menu check on app start (#13)"
```

---

### Task 7: Wire up foreground detection in Menus screen

**Files:**
- Modify: `src/app/app/(tabs)/index.tsx`

**Step 1: Add imports**

At the top of `src/app/app/(tabs)/index.tsx`, add:

```typescript
import { computeFingerprints, detectNewMenus } from '../../src-rn/utils/menuFingerprint';
import {
  getKnownMenus,
  setKnownMenus,
  getNotificationSent,
  setNotificationSent,
} from '../../src-rn/utils/menuChangeStorage';
import { NewMenuToast } from '../../src-rn/components/NewMenuToast';
```

Add `useState` to the existing React import:

```typescript
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
```

**Step 2: Add toast state and detection logic**

Inside the `MenusScreen` component, after the `const { orders, fetchOrders } = useOrderStore();` line, add:

```typescript
const [showToast, setShowToast] = useState(false);
```

**Step 3: Add detection function to `triggerRefresh`**

Replace the existing `triggerRefresh` callback with:

```typescript
const triggerRefresh = useCallback(() => {
  const auth = useAuthStore.getState().status;
  if (auth !== 'authenticated') return;

  const { loadCachedMenus } = useMenuStore.getState();
  const { loadCachedOrders } = useOrderStore.getState();

  // Load cache first for instant display
  Promise.all([loadCachedMenus(), loadCachedOrders()]).catch(() => {}).finally(() => {
    const cached = useMenuStore.getState().items.length > 0;
    const refreshPromise = cached ? refreshAvailability() : fetchMenus();

    // After menu fetch completes, check for new menus
    refreshPromise?.then(async () => {
      try {
        const currentItems = useMenuStore.getState().items;
        if (currentItems.length === 0) return;

        const currentFingerprints = computeFingerprints(currentItems);
        const knownMenus = await getKnownMenus();
        const notificationSent = await getNotificationSent();

        if (detectNewMenus(currentFingerprints, knownMenus) && !notificationSent) {
          setShowToast(true);
          await setNotificationSent(true);
        }

        // Acknowledge: update known menus and reset notification flag
        await setKnownMenus(currentFingerprints);
        await setNotificationSent(false);
      } catch {
        // Silent — don't break menu loading for fingerprint errors
      }
    });

    fetchOrders();
  });
}, [fetchMenus, refreshAvailability, fetchOrders]);
```

Note: `refreshAvailability()` and `fetchMenus()` both return `Promise<void>`. The detection runs after the menu data is available in the store.

**Step 4: Add toast to the render output**

In the mobile layout return (the non-`isWideLayout` branch), add the toast inside the container, just before the `DayNavigator`:

```tsx
<NewMenuToast visible={showToast} onDismiss={() => setShowToast(false)} />
```

Also add it in the `isWideLayout` branch, inside the container just before `desktopRow`:

```tsx
<NewMenuToast visible={showToast} onDismiss={() => setShowToast(false)} />
```

**Step 5: Run full test suite**

Run: `cd src/app && npm test`
Expected: ALL PASS

**Step 6: Commit**

```bash
git add src/app/app/(tabs)/index.tsx
git commit -m "feat: add foreground new-menu detection with toast (#13)"
```

---

### Task 8: Verify on iOS Simulator

**Step 1: Rebuild and run**

Run: `cd src/app && npx expo run:ios`

**Step 2: Verify notification permission prompt**

On first launch after rebuild, iOS should prompt for notification permissions. Accept it.

**Step 3: Verify toast on fresh data**

1. Login with credentials
2. Navigate to the Menus tab
3. On the very first load (no known menus stored yet), the toast "Neue Menüs verfügbar!" should appear briefly
4. Navigate away and back — toast should NOT appear (menus are now known)

**Step 4: Verify background task registration**

Check that the background task is registered by looking at console output in Xcode. The task `BACKGROUND_MENU_CHECK` should be registered.

**Step 5: Verify no regressions**

- Menu ordering still works
- Menu display still works
- Swipe gestures still work
- All tabs are accessible

---

### Task 9: Verify on Android Emulator

**Step 1: Rebuild and run**

Run: `cd src/app && npx expo run:android`

**Step 2: Verify notification channel**

In Android Settings > Apps > SnackPilot > Notifications, the "Neue Menüs" channel should be listed.

**Step 3: Verify same behavior as iOS**

- Toast appears on first load
- Toast does not reappear on subsequent navigations
- Menu ordering and display work normally
