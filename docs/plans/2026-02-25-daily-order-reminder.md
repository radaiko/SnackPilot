# Daily Order Reminder Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a daily local notification showing today's ordered meals at a user-configured time.

**Architecture:** Extends the existing `BACKGROUND_ORDER_SYNC_TASK` with a daily reminder check. New `reminderStorage.ts` manages settings and dedup state via AsyncStorage. Settings UI adds a toggle + 15-min-increment time picker under a unified "Benachrichtigungen" section. Native-only (iOS + Android), web stubs via platform extensions.

**Tech Stack:** expo-notifications (already installed on feature/issue-13-menu-notifications branch), AsyncStorage, Zustand stores, React Native

**Base branch:** `feature/issue-13-menu-notifications` (contains all notification infrastructure)

---

## Task 1: Reminder Storage Module

**Files:**
- Create: `src/app/src-rn/utils/reminderStorage.ts`
- Test: `src/app/src-rn/__tests__/utils/reminderStorage.test.ts`

**Step 1: Write the failing tests**

Create `src/app/src-rn/__tests__/utils/reminderStorage.test.ts`:

```typescript
jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach((k) => { delete store[k]; }); return Promise.resolve(); }),
    },
  };
});

import {
  getReminderEnabled,
  setReminderEnabled,
  getReminderTime,
  setReminderTime,
  getReminderSentDate,
  setReminderSentDate,
} from '../../utils/reminderStorage';

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
});

describe('reminderStorage', () => {
  describe('reminderEnabled', () => {
    it('returns false when nothing stored', async () => {
      expect(await getReminderEnabled()).toBe(false);
    });

    it('persists true', async () => {
      await setReminderEnabled(true);
      expect(await getReminderEnabled()).toBe(true);
    });

    it('persists false after true', async () => {
      await setReminderEnabled(true);
      await setReminderEnabled(false);
      expect(await getReminderEnabled()).toBe(false);
    });
  });

  describe('reminderTime', () => {
    it('returns null when nothing stored', async () => {
      expect(await getReminderTime()).toBeNull();
    });

    it('persists hour and minute', async () => {
      await setReminderTime(11, 30);
      expect(await getReminderTime()).toEqual({ hour: 11, minute: 30 });
    });

    it('overwrites previous time', async () => {
      await setReminderTime(11, 30);
      await setReminderTime(12, 0);
      expect(await getReminderTime()).toEqual({ hour: 12, minute: 0 });
    });
  });

  describe('reminderSentDate', () => {
    it('returns null when nothing stored', async () => {
      expect(await getReminderSentDate()).toBeNull();
    });

    it('persists a date string', async () => {
      await setReminderSentDate('2026-02-25');
      expect(await getReminderSentDate()).toBe('2026-02-25');
    });
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd src/app && npx jest src-rn/__tests__/utils/reminderStorage.test.ts --no-coverage`
Expected: FAIL — module not found

**Step 3: Write the implementation**

Create `src/app/src-rn/utils/reminderStorage.ts`:

```typescript
import AsyncStorage from '@react-native-async-storage/async-storage';

const REMINDER_ENABLED_KEY = 'daily_reminder_enabled';
const REMINDER_TIME_KEY = 'daily_reminder_time';
const REMINDER_SENT_DATE_KEY = 'daily_reminder_sent_date';

export async function getReminderEnabled(): Promise<boolean> {
  const value = await AsyncStorage.getItem(REMINDER_ENABLED_KEY);
  return value === 'true';
}

export async function setReminderEnabled(enabled: boolean): Promise<void> {
  await AsyncStorage.setItem(REMINDER_ENABLED_KEY, String(enabled));
}

export async function getReminderTime(): Promise<{ hour: number; minute: number } | null> {
  const raw = await AsyncStorage.getItem(REMINDER_TIME_KEY);
  if (raw === null) return null;
  try {
    const parsed = JSON.parse(raw);
    if (
      parsed !== null &&
      typeof parsed === 'object' &&
      typeof parsed.hour === 'number' &&
      typeof parsed.minute === 'number'
    ) {
      return parsed as { hour: number; minute: number };
    }
    return null;
  } catch {
    return null;
  }
}

export async function setReminderTime(hour: number, minute: number): Promise<void> {
  await AsyncStorage.setItem(REMINDER_TIME_KEY, JSON.stringify({ hour, minute }));
}

export async function getReminderSentDate(): Promise<string | null> {
  return AsyncStorage.getItem(REMINDER_SENT_DATE_KEY);
}

export async function setReminderSentDate(dateString: string): Promise<void> {
  await AsyncStorage.setItem(REMINDER_SENT_DATE_KEY, dateString);
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src/app && npx jest src-rn/__tests__/utils/reminderStorage.test.ts --no-coverage`
Expected: 6 tests PASS

**Step 5: Commit**

```bash
git add src/app/src-rn/utils/reminderStorage.ts src/app/src-rn/__tests__/utils/reminderStorage.test.ts
git commit -m "feat(#31): add reminder storage module for daily order notifications"
```

---

## Task 2: Daily Reminder Check Logic

**Files:**
- Create: `src/app/src-rn/utils/dailyReminderCheck.ts`
- Test: `src/app/src-rn/__tests__/utils/dailyReminderCheck.test.ts`

This is a pure function module (no side effects at import time) that reads reminder settings, checks the time window, filters today's orders, and fires the notification. Separated from `notificationTasks.ts` for testability.

**Step 1: Write the failing tests**

Create `src/app/src-rn/__tests__/utils/dailyReminderCheck.test.ts`:

```typescript
jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach((k) => { delete store[k]; }); return Promise.resolve(); }),
    },
  };
});

jest.mock('expo-notifications', () => ({
  scheduleNotificationAsync: jest.fn(),
}));

jest.mock('../../store/orderStore', () => {
  let orders: any[] = [];
  return {
    useOrderStore: {
      getState: () => ({ orders }),
      __setOrders: (o: any[]) => { orders = o; },
    },
  };
});

jest.mock('../../utils/dateUtils', () => ({
  viennaMinutes: jest.fn(),
  viennaToday: jest.fn(),
  isSameDay: jest.fn((a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  ),
  localDateKey: jest.fn((d: Date) => {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }),
}));

import * as Notifications from 'expo-notifications';
import { useOrderStore } from '../../store/orderStore';
import { viennaMinutes, viennaToday } from '../../utils/dateUtils';
import { setReminderEnabled, setReminderTime, setReminderSentDate } from '../../utils/reminderStorage';
import { checkDailyReminder } from '../../utils/dailyReminderCheck';

const mockViennaMinutes = viennaMinutes as jest.Mock;
const mockViennaToday = viennaToday as jest.Mock;

beforeEach(async () => {
  jest.clearAllMocks();
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  (useOrderStore as any).__setOrders([]);
  mockViennaToday.mockReturnValue(new Date(2026, 1, 25)); // Feb 25, 2026
});

describe('checkDailyReminder', () => {
  it('does nothing when reminder is disabled', async () => {
    await setReminderEnabled(false);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when no time is configured', async () => {
    await setReminderEnabled(true);
    mockViennaMinutes.mockReturnValue(11 * 60);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when outside the ±15 min time window', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(9 * 60); // 9:00, way before 11:00

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when no orders for today', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 26), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('does nothing when already sent today', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    await setReminderSentDate('2026-02-25');
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });

  it('sends notification with single order', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ II', subtitle: 'Wiener Schnitzel', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith({
      content: {
        title: 'Deine Bestellung heute',
        body: 'MENÜ II — Wiener Schnitzel',
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: null,
    });
  });

  it('sends notification with multiple orders', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60 + 5); // 11:05, within window

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ II', subtitle: 'Wiener Schnitzel', approved: true },
      { positionId: '2', eatingCycleId: 'e2', date: new Date(2026, 1, 25), title: 'SUPPE & SALAT', subtitle: 'Tomatensuppe', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith({
      content: {
        title: 'Deine Bestellung heute',
        body: 'MENÜ II — Wiener Schnitzel\nSUPPE & SALAT — Tomatensuppe',
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: null,
    });
  });

  it('marks sent date after firing notification', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();
    // Second call should not send again
    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledTimes(1);
  });

  it('sends again on a new day', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    await setReminderSentDate('2026-02-24'); // yesterday
    mockViennaMinutes.mockReturnValue(11 * 60);

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledTimes(1);
  });

  it('fires at boundary of ±15 min window', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60 + 15); // exactly +15 min

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledTimes(1);
  });

  it('does not fire just outside ±15 min window', async () => {
    await setReminderEnabled(true);
    await setReminderTime(11, 0);
    mockViennaMinutes.mockReturnValue(11 * 60 + 16); // +16 min, outside

    (useOrderStore as any).__setOrders([
      { positionId: '1', eatingCycleId: 'e1', date: new Date(2026, 1, 25), title: 'MENÜ I', subtitle: 'Test', approved: true },
    ]);

    await checkDailyReminder();

    expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
  });
});
```

**Step 2: Run tests to verify they fail**

Run: `cd src/app && npx jest src-rn/__tests__/utils/dailyReminderCheck.test.ts --no-coverage`
Expected: FAIL — module not found

**Step 3: Write the implementation**

Create `src/app/src-rn/utils/dailyReminderCheck.ts`:

```typescript
import * as Notifications from 'expo-notifications';
import { useOrderStore } from '../store/orderStore';
import { viennaMinutes, viennaToday, isSameDay, localDateKey } from './dateUtils';
import {
  getReminderEnabled,
  getReminderTime,
  getReminderSentDate,
  setReminderSentDate,
} from './reminderStorage';

/**
 * Check if a daily order reminder notification should fire.
 * Called from BACKGROUND_ORDER_SYNC_TASK.
 *
 * Guards:
 * 1. Reminder must be enabled
 * 2. Time must be configured
 * 3. Current Vienna time must be within ±15 min of configured time
 * 4. There must be orders for today
 * 5. Notification must not have been sent today already
 */
export async function checkDailyReminder(): Promise<void> {
  const enabled = await getReminderEnabled();
  if (!enabled) return;

  const time = await getReminderTime();
  if (!time) return;

  const targetMinutes = time.hour * 60 + time.minute;
  const currentMinutes = viennaMinutes();
  if (Math.abs(currentMinutes - targetMinutes) > 15) return;

  const today = viennaToday();
  const todayKey = localDateKey(today);

  const sentDate = await getReminderSentDate();
  if (sentDate === todayKey) return;

  const orders = useOrderStore.getState().orders;
  const todayOrders = orders.filter((o) => isSameDay(o.date, today));
  if (todayOrders.length === 0) return;

  const body = todayOrders
    .map((o) => (o.subtitle ? `${o.title} — ${o.subtitle}` : o.title))
    .join('\n');

  await Notifications.scheduleNotificationAsync({
    content: {
      title: 'Deine Bestellung heute',
      body,
      sound: 'default',
      data: { screen: '/(tabs)/orders' },
    },
    trigger: null,
  });

  await setReminderSentDate(todayKey);
}
```

**Step 4: Run tests to verify they pass**

Run: `cd src/app && npx jest src-rn/__tests__/utils/dailyReminderCheck.test.ts --no-coverage`
Expected: 10 tests PASS

**Step 5: Commit**

```bash
git add src/app/src-rn/utils/dailyReminderCheck.ts src/app/src-rn/__tests__/utils/dailyReminderCheck.test.ts
git commit -m "feat(#31): add daily reminder check logic with time window and dedup"
```

---

## Task 3: Integrate into Background Task

**Files:**
- Modify: `src/app/src-rn/utils/notificationTasks.ts` (the `BACKGROUND_ORDER_SYNC_TASK` handler)

**Step 1: Modify the background task to call `checkDailyReminder`**

In `src/app/src-rn/utils/notificationTasks.ts`, inside the `TaskManager.defineTask(BACKGROUND_ORDER_SYNC_TASK, ...)` callback, add the daily reminder check. The existing task already calls `loadCachedOrders()`, so order data is available.

Add import at top (inside the `if (Platform.OS !== 'web')` block or at module level):

```typescript
import { checkDailyReminder } from './dailyReminderCheck';
```

Modify the `BACKGROUND_ORDER_SYNC_TASK` handler to add `await checkDailyReminder()` after `await checkAndNotify()`:

```typescript
  TaskManager.defineTask(BACKGROUND_ORDER_SYNC_TASK, async () => {
    try {
      // Load cached orders (no network calls to avoid concurrent scraping)
      await useOrderStore.getState().loadCachedOrders();

      // Location-based notification check (only if company location configured)
      if (useLocationStore.getState().hasCompanyLocation()) {
        await checkAndNotify();
      }

      // Daily order reminder check
      await checkDailyReminder();

      return BackgroundTask.BackgroundTaskResult.Success;
    } catch {
      return BackgroundTask.BackgroundTaskResult.Failed;
    }
  });
```

Note: The guard `if (!useLocationStore.getState().hasCompanyLocation())` that previously caused an early return is moved to only wrap `checkAndNotify()`. The task should always run `checkDailyReminder()` regardless of whether a company location is configured.

**Step 2: Ensure the background task also registers when reminder is enabled**

Currently in `_layout.tsx`, the background sync is only registered when `hasCompanyLocation`. We need it to also register when the daily reminder is enabled. This will be handled in Task 5 (Settings UI), which calls `registerBackgroundSync()` when enabling the reminder.

No changes to `_layout.tsx` needed here — the `enableNotifications()` call already registers the background sync, and we'll add a separate registration path from the settings toggle.

**Step 3: Run all existing tests to verify no regressions**

Run: `cd src/app && npx jest --no-coverage`
Expected: All tests PASS

**Step 4: Commit**

```bash
git add src/app/src-rn/utils/notificationTasks.ts
git commit -m "feat(#31): integrate daily reminder check into background sync task"
```

---

## Task 4: Register Background Sync for Reminder

**Files:**
- Modify: `src/app/app/_layout.tsx`

The background sync task (`BACKGROUND_ORDER_SYNC_TASK`) currently only registers when a company location is set. It must also register when the daily reminder is enabled, so the task runs even without location-based notifications.

**Step 1: Add reminder-aware registration to `_layout.tsx`**

In `_layout.tsx`, add an effect that checks reminder state and registers the background sync:

```typescript
import { getReminderEnabled } from '../src-rn/utils/reminderStorage';
```

Add after the existing `useEffect` that calls `enableNotifications()`:

```typescript
  // Register background sync when daily reminder is enabled (even without company location)
  useEffect(() => {
    if (!isNative()) return;
    (async () => {
      const reminderEnabled = await getReminderEnabled();
      if (reminderEnabled) {
        await registerBackgroundSync();
      }
    })();
  }, []);
```

Import `registerBackgroundSync` from `notificationService` (it may already be imported via `enableNotifications`; if not, add it to the import).

**Step 2: Run all tests**

Run: `cd src/app && npx jest --no-coverage`
Expected: All tests PASS

**Step 3: Commit**

```bash
git add src/app/app/_layout.tsx
git commit -m "feat(#31): register background sync when daily reminder is enabled"
```

---

## Task 5: Settings UI — Benachrichtigungen Section

**Files:**
- Modify: `src/app/app/(tabs)/settings.tsx`

Restructure the settings screen: rename the location notification section and add the daily reminder toggle + time picker above it, all under a unified "Benachrichtigungen" heading.

**Step 1: Add state and handlers for daily reminder**

Add imports:

```typescript
import {
  getReminderEnabled,
  setReminderEnabled,
  getReminderTime,
  setReminderTime,
} from '../../src-rn/utils/reminderStorage';
import { registerBackgroundSync } from '../../src-rn/utils/notificationService';
```

Add state variables inside the component:

```typescript
  // Daily reminder state (mobile only)
  const [reminderEnabled, setReminderEnabledState] = useState(false);
  const [reminderHour, setReminderHour] = useState(11);
  const [reminderMinute, setReminderMinute] = useState(0);
```

Load saved reminder state in the existing `useEffect` that loads credentials (or a new one):

```typescript
  useEffect(() => {
    if (!isNative()) return;
    (async () => {
      const enabled = await getReminderEnabled();
      setReminderEnabledState(enabled);
      const time = await getReminderTime();
      if (time) {
        setReminderHour(time.hour);
        setReminderMinute(time.minute);
      }
    })();
  }, []);
```

Add handlers:

```typescript
  const handleReminderToggle = async () => {
    const newValue = !reminderEnabled;
    if (newValue) {
      const granted = await requestNotificationPermissions();
      if (!granted) {
        alert('Berechtigung fehlt', 'Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.');
        return;
      }
      await setReminderTime(reminderHour, reminderMinute);
      await registerBackgroundSync();
    }
    await setReminderEnabled(newValue);
    setReminderEnabledState(newValue);
  };

  const handleReminderTimeChange = async (hour: number, minute: number) => {
    setReminderHour(hour);
    setReminderMinute(minute);
    await setReminderTime(hour, minute);
  };
```

**Step 2: Build time picker options**

Generate 15-min increment time options:

```typescript
const TIME_OPTIONS: { hour: number; minute: number; label: string }[] = [];
for (let h = 0; h < 24; h++) {
  for (let m = 0; m < 60; m += 15) {
    TIME_OPTIONS.push({
      hour: h,
      minute: m,
      label: `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`,
    });
  }
}
```

Place this outside the component (module-level constant).

**Step 3: Replace `locationCard` with unified `notificationCard`**

Replace the existing `locationCard` variable with a `notificationCard` that contains both sub-features. Use a `Switch` for the reminder toggle and a horizontal `ScrollView` or a picker for the time. For simplicity and cross-platform consistency, use a row of `Pressable` buttons showing the selected time with left/right arrows or a dropdown-style selector.

Recommended approach — a simple `Pressable` that cycles through times, or a pair of Pressable for hour and minute. Simplest: show the current time as a tappable element that opens a scrollable list.

For MVP, use a horizontal `ScrollView` of time chips (similar to the theme selector pattern already in the settings):

```tsx
  const notificationCard = isNative() ? (
    <View style={isWideLayout ? styles.desktopCard : undefined}>
      {!isWideLayout && <View style={styles.divider} />}
      <Text style={styles.sectionTitle}>Benachrichtigungen</Text>

      {/* Daily Reminder */}
      <View style={styles.reminderRow}>
        <View style={{ flex: 1 }}>
          <Text style={styles.label}>Bestell-Erinnerung</Text>
          <Text style={styles.reminderHint}>
            Tägliche Erinnerung an deine Bestellung
          </Text>
        </View>
        <Switch
          value={reminderEnabled}
          onValueChange={handleReminderToggle}
          trackColor={{ false: colors.border, true: colors.primary }}
        />
      </View>

      {reminderEnabled && (
        <View style={styles.timePickerSection}>
          <Text style={styles.label}>Uhrzeit</Text>
          <ScrollView
            horizontal
            showsHorizontalScrollIndicator={false}
            style={styles.timeScroll}
            contentContainerStyle={styles.timeScrollContent}
          >
            {TIME_OPTIONS.map((opt) => {
              const isSelected = opt.hour === reminderHour && opt.minute === reminderMinute;
              return (
                <Pressable
                  key={opt.label}
                  style={[styles.timeChip, isSelected && styles.timeChipActive]}
                  onPress={() => handleReminderTimeChange(opt.hour, opt.minute)}
                >
                  <Text style={[styles.timeChipText, isSelected && styles.timeChipTextActive]}>
                    {opt.label}
                  </Text>
                </Pressable>
              );
            })}
          </ScrollView>
        </View>
      )}

      <View style={styles.divider} />

      {/* Location Notifications (existing) */}
      <Text style={styles.label}>Standort-Benachrichtigungen</Text>
      <Text style={styles.reminderHint}>
        Erinnerung um 8:45 basierend auf deinem Standort
      </Text>

      {companyLocation ? (
        <View>
          <Text style={styles.sessionInfo}>
            Firmenstandort gesetzt
          </Text>
          <Pressable style={styles.buttonDanger} onPress={handleRemoveLocation}>
            <Text style={styles.buttonDangerText}>Standort entfernen</Text>
          </Pressable>
        </View>
      ) : (
        <Pressable
          style={[styles.button, styles.buttonPrimary]}
          onPress={handleSetLocation}
          disabled={locationSaving}
        >
          <Text style={styles.buttonPrimaryText}>
            {locationSaving ? 'Standort wird ermittelt...' : 'Aktuellen Standort als Firmenstandort setzen'}
          </Text>
        </Pressable>
      )}
    </View>
  ) : null;
```

Add `Switch` to the React Native imports at the top of the file.

**Step 4: Add styles**

Add to `createStyles`:

```typescript
    reminderRow: {
      flexDirection: 'row',
      alignItems: 'center',
      marginBottom: isCompactDesktop ? 8 : 12,
    },
    reminderHint: {
      fontSize: isCompactDesktop ? 11 : 12,
      color: c.textTertiary,
      marginTop: 2,
    },
    timePickerSection: {
      marginBottom: isCompactDesktop ? 10 : 16,
    },
    timeScroll: {
      marginTop: isCompactDesktop ? 4 : 8,
    },
    timeScrollContent: {
      gap: isCompactDesktop ? 6 : 8,
    },
    timeChip: {
      paddingHorizontal: isCompactDesktop ? 10 : 14,
      paddingVertical: isCompactDesktop ? 6 : 10,
      ...bannerSurface(c),
    },
    timeChipActive: {
      backgroundColor: useFlatStyle ? c.primarySurface : c.glassPrimary,
      borderColor: useFlatStyle ? c.primary : undefined,
      borderBottomColor: c.primary,
    },
    timeChipText: {
      fontSize: isCompactDesktop ? 12 : 14,
      fontWeight: '600',
      color: c.textSecondary,
    },
    timeChipTextActive: {
      color: c.primary,
    },
```

**Step 5: Replace `locationCard` references in the JSX**

In both the `isWideLayout` and mobile render paths, replace `{locationCard}` with `{notificationCard}`.

**Step 6: Run all tests**

Run: `cd src/app && npx jest --no-coverage`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add src/app/app/\(tabs\)/settings.tsx
git commit -m "feat(#31): add daily reminder settings with time picker to Benachrichtigungen section"
```

---

## Task 6: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cd src/app && npx jest --no-coverage`
Expected: All tests PASS (existing + new)

**Step 2: Verify new test count**

The new tests add:
- `reminderStorage.test.ts`: 6 tests
- `dailyReminderCheck.test.ts`: 10 tests
- Total new: 16 tests

**Step 3: Commit if any fixes were needed**

If any fixes were required, commit them:
```bash
git commit -m "fix(#31): address test issues"
```
