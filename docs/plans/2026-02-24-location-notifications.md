# Location-Based Order Notifications Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Notify mobile users at 8:45am Vienna time if they're at the office without an order, or away from the office with an order.

**Architecture:** Geofencing via `expo-location` + `expo-task-manager` tracks office presence. `expo-background-task` periodically refreshes order cache. `expo-notifications` sends local notifications. A Zustand store persists company location and geofence state. The saved location acts as the on/off toggle — no separate setting.

**Tech Stack:** expo-location, expo-task-manager, expo-background-task, expo-notifications, Zustand, AsyncStorage

---

### Task 1: Install Dependencies

**Files:**
- Modify: `src/app/package.json`

**Step 1: Install Expo packages**

Run:
```bash
cd src/app && npx expo install expo-location expo-task-manager expo-background-task expo-notifications
```

**Step 2: Verify installation**

Run:
```bash
cd src/app && cat package.json | grep -E "expo-location|expo-task-manager|expo-background-task|expo-notifications"
```

Expected: All four packages listed in dependencies.

**Step 3: Commit**

```bash
git add src/app/package.json src/app/package-lock.json
git commit -m "feat(#25): add expo-location, task-manager, background-task, notifications deps"
```

---

### Task 2: Configure app.json Plugins and Permissions

**Files:**
- Modify: `src/app/app.json`

**Step 1: Add plugins and iOS/Android permissions**

In `app.json`, add to the `plugins` array (after the existing entries):

```json
[
  "expo-location",
  {
    "locationAlwaysAndWhenInUsePermission": "SnackPilot nutzt deinen Standort, um dich an Bestellungen zu erinnern, wenn du im Büro bist.",
    "locationWhenInUsePermission": "SnackPilot nutzt deinen Standort, um deinen Firmenstandort zu speichern.",
    "isIosBackgroundLocationEnabled": true,
    "isAndroidBackgroundLocationEnabled": true,
    "isAndroidForegroundServiceEnabled": true
  }
],
"expo-notifications",
"expo-background-task"
```

Add to `ios.infoPlist`:

```json
"UIBackgroundModes": ["location", "processing"],
"BGTaskSchedulerPermittedIdentifiers": ["com.expo.modules.backgroundtask.processing"]
```

Add `android.permissions` array:

```json
"permissions": [
  "ACCESS_COARSE_LOCATION",
  "ACCESS_FINE_LOCATION",
  "ACCESS_BACKGROUND_LOCATION",
  "FOREGROUND_SERVICE",
  "FOREGROUND_SERVICE_LOCATION"
]
```

The full updated `app.json` should look like:

```json
{
  "expo": {
    "name": "SnackPilot",
    "slug": "GourmetApp",
    "scheme": "snackpilot",
    "version": "1.3.3",
    "orientation": "portrait",
    "icon": "./assets/icons/icon-orange.png",
    "userInterfaceStyle": "automatic",
    "newArchEnabled": true,
    "splash": {
      "image": "./assets/splash-icon.png",
      "resizeMode": "contain",
      "backgroundColor": "#ffffff"
    },
    "ios": {
      "supportsTablet": false,
      "bundleIdentifier": "dev.radaiko.gourmetclient",
      "buildNumber": "1",
      "infoPlist": {
        "ITSAppUsesNonExemptEncryption": false,
        "UIBackgroundModes": ["location", "processing"],
        "BGTaskSchedulerPermittedIdentifiers": ["com.expo.modules.backgroundtask.processing"]
      }
    },
    "android": {
      "package": "dev.radaiko.gourmetclient",
      "adaptiveIcon": {
        "foregroundImage": "./assets/icons/adaptive-icon-orange.png",
        "backgroundColor": "#F0F0F2"
      },
      "edgeToEdgeEnabled": true,
      "predictiveBackGestureEnabled": false,
      "permissions": [
        "ACCESS_COARSE_LOCATION",
        "ACCESS_FINE_LOCATION",
        "ACCESS_BACKGROUND_LOCATION",
        "FOREGROUND_SERVICE",
        "FOREGROUND_SERVICE_LOCATION"
      ]
    },
    "web": {
      "output": "single",
      "bundler": "metro",
      "favicon": "./assets/favicon.png"
    },
    "plugins": [
      "expo-router",
      "expo-secure-store",
      "expo-font",
      [
        "@g9k/expo-dynamic-app-icon",
        {
          "emerald": {
            "ios": "./assets/icons/icon-emerald.png",
            "android": {
              "foregroundImage": "./assets/icons/adaptive-icon-emerald.png",
              "backgroundColor": "#F0F0F2"
            }
          },
          "berry": {
            "ios": "./assets/icons/icon-berry.png",
            "android": {
              "foregroundImage": "./assets/icons/adaptive-icon-berry.png",
              "backgroundColor": "#F0F0F2"
            }
          },
          "golden": {
            "ios": "./assets/icons/icon-golden.png",
            "android": {
              "foregroundImage": "./assets/icons/adaptive-icon-golden.png",
              "backgroundColor": "#F0F0F2"
            }
          },
          "ocean": {
            "ios": "./assets/icons/icon-ocean.png",
            "android": {
              "foregroundImage": "./assets/icons/adaptive-icon-ocean.png",
              "backgroundColor": "#F0F0F2"
            }
          }
        }
      ],
      [
        "expo-location",
        {
          "locationAlwaysAndWhenInUsePermission": "SnackPilot nutzt deinen Standort, um dich an Bestellungen zu erinnern, wenn du im Büro bist.",
          "locationWhenInUsePermission": "SnackPilot nutzt deinen Standort, um deinen Firmenstandort zu speichern.",
          "isIosBackgroundLocationEnabled": true,
          "isAndroidBackgroundLocationEnabled": true,
          "isAndroidForegroundServiceEnabled": true
        }
      ],
      "expo-notifications",
      "expo-background-task"
    ],
    "extra": {
      "router": {},
      "eas": {
        "projectId": "efb12eb3-0729-4ea2-a3db-8026d95db7d3"
      }
    },
    "owner": "radaiko"
  }
}
```

**Step 2: Commit**

```bash
git add src/app/app.json
git commit -m "feat(#25): configure location, notification, and background task plugins"
```

---

### Task 3: Create Location Store

**Files:**
- Create: `src/app/src-rn/store/locationStore.ts`
- Test: `src/app/src-rn/__tests__/store/locationStore.test.ts`

**Step 1: Write the failing test**

Create `src/app/src-rn/__tests__/store/locationStore.test.ts`:

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

import { useLocationStore } from '../../store/locationStore';

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
  useLocationStore.setState({
    companyLocation: null,
    isAtCompany: false,
  });
});

describe('locationStore', () => {
  it('has no company location by default', () => {
    const { companyLocation } = useLocationStore.getState();
    expect(companyLocation).toBeNull();
  });

  it('has isAtCompany false by default', () => {
    expect(useLocationStore.getState().isAtCompany).toBe(false);
  });

  it('setCompanyLocation saves lat/lng', () => {
    useLocationStore.getState().setCompanyLocation(48.2082, 16.3738);
    const { companyLocation } = useLocationStore.getState();
    expect(companyLocation).toEqual({ latitude: 48.2082, longitude: 16.3738 });
  });

  it('clearCompanyLocation resets location and isAtCompany', () => {
    useLocationStore.getState().setCompanyLocation(48.2082, 16.3738);
    useLocationStore.getState().setIsAtCompany(true);
    useLocationStore.getState().clearCompanyLocation();
    expect(useLocationStore.getState().companyLocation).toBeNull();
    expect(useLocationStore.getState().isAtCompany).toBe(false);
  });

  it('setIsAtCompany updates the flag', () => {
    useLocationStore.getState().setIsAtCompany(true);
    expect(useLocationStore.getState().isAtCompany).toBe(true);
    useLocationStore.getState().setIsAtCompany(false);
    expect(useLocationStore.getState().isAtCompany).toBe(false);
  });

  it('hasCompanyLocation returns true when location is set', () => {
    expect(useLocationStore.getState().hasCompanyLocation()).toBe(false);
    useLocationStore.getState().setCompanyLocation(48.2082, 16.3738);
    expect(useLocationStore.getState().hasCompanyLocation()).toBe(true);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd src/app && npx jest src-rn/__tests__/store/locationStore.test.ts -v`

Expected: FAIL — Cannot find module `../../store/locationStore`

**Step 3: Write the store**

Create `src/app/src-rn/store/locationStore.ts`:

```typescript
import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import AsyncStorage from '@react-native-async-storage/async-storage';

export interface CompanyLocation {
  latitude: number;
  longitude: number;
}

interface LocationState {
  companyLocation: CompanyLocation | null;
  isAtCompany: boolean;

  setCompanyLocation: (latitude: number, longitude: number) => void;
  clearCompanyLocation: () => void;
  setIsAtCompany: (value: boolean) => void;
  hasCompanyLocation: () => boolean;
}

export const useLocationStore = create<LocationState>()(
  persist(
    (set, get) => ({
      companyLocation: null,
      isAtCompany: false,

      setCompanyLocation: (latitude, longitude) =>
        set({ companyLocation: { latitude, longitude } }),

      clearCompanyLocation: () =>
        set({ companyLocation: null, isAtCompany: false }),

      setIsAtCompany: (value) =>
        set({ isAtCompany: value }),

      hasCompanyLocation: () =>
        get().companyLocation !== null,
    }),
    {
      name: 'company-location',
      storage: createJSONStorage(() => AsyncStorage),
    }
  )
);
```

**Step 4: Run test to verify it passes**

Run: `cd src/app && npx jest src-rn/__tests__/store/locationStore.test.ts -v`

Expected: All 6 tests PASS

**Step 5: Commit**

```bash
git add src/app/src-rn/store/locationStore.ts src/app/src-rn/__tests__/store/locationStore.test.ts
git commit -m "feat(#25): add location store with company location persistence"
```

---

### Task 4: Create Notification Service — Constants and Task Definitions

**Files:**
- Modify: `src/app/src-rn/utils/constants.ts`
- Create: `src/app/src-rn/utils/notificationTasks.ts`

**Step 1: Add constants**

Add to the end of `src/app/src-rn/utils/constants.ts`:

```typescript
// Location-based notifications
export const GEOFENCE_TASK_NAME = 'COMPANY_GEOFENCE_TASK';
export const BACKGROUND_ORDER_SYNC_TASK = 'BACKGROUND_ORDER_SYNC_TASK';
export const COMPANY_GEOFENCE_RADIUS_M = 500;
export const NOTIFICATION_HOUR = 8;
export const NOTIFICATION_MINUTE = 45;
export const NOTIFICATION_CHANNEL_ID = 'order-reminders';
```

**Step 2: Create task definitions file**

This file defines background tasks at module scope (required by expo-task-manager). It must be imported early in the app entry point.

Create `src/app/src-rn/utils/notificationTasks.ts`:

```typescript
import { Platform } from 'react-native';
import * as TaskManager from 'expo-task-manager';
import * as Location from 'expo-location';
import * as Notifications from 'expo-notifications';
import { useLocationStore } from '../store/locationStore';
import { useOrderStore } from '../store/orderStore';
import { useAuthStore } from '../store/authStore';
import { isSameDay } from './dateUtils';
import {
  GEOFENCE_TASK_NAME,
  BACKGROUND_ORDER_SYNC_TASK,
} from './constants';

// Only define tasks on native platforms
if (Platform.OS !== 'web') {
  // --- Geofence task: update isAtCompany flag ---
  interface GeofencingTaskData {
    eventType: Location.GeofencingEventType;
    region: Location.LocationRegion;
  }

  TaskManager.defineTask<GeofencingTaskData>(
    GEOFENCE_TASK_NAME,
    ({ data, error }) => {
      if (error) return;
      const { eventType } = data;
      if (eventType === Location.GeofencingEventType.Enter) {
        useLocationStore.getState().setIsAtCompany(true);
      } else if (eventType === Location.GeofencingEventType.Exit) {
        useLocationStore.getState().setIsAtCompany(false);
      }
    }
  );

  // --- Background order sync task ---
  // Import dynamically to avoid pulling in BackgroundTask on web
  const BackgroundTask = require('expo-background-task');

  TaskManager.defineTask(BACKGROUND_ORDER_SYNC_TASK, async () => {
    try {
      // Only sync if we have a company location configured
      if (!useLocationStore.getState().hasCompanyLocation()) {
        return BackgroundTask.BackgroundTaskResult.Success;
      }

      // Try to login if not authenticated
      const authState = useAuthStore.getState();
      if (authState.status !== 'authenticated') {
        await authState.loginWithSaved();
      }

      // Refresh orders
      if (useAuthStore.getState().status === 'authenticated') {
        await useOrderStore.getState().fetchOrders();
      }

      // Check if we should fire a notification (8:45 Vienna time window)
      await checkAndNotify();

      return BackgroundTask.BackgroundTaskResult.Success;
    } catch {
      return BackgroundTask.BackgroundTaskResult.Failed;
    }
  });
}

/**
 * Check order/location state and send notification if needed.
 * Called from the background sync task.
 */
async function checkAndNotify(): Promise<void> {
  const { isAtCompany } = useLocationStore.getState();
  const orders = useOrderStore.getState().orders;
  const today = new Date();
  const hasOrderToday = orders.some((o) => isSameDay(o.date, today));

  if (isAtCompany && !hasOrderToday) {
    await Notifications.scheduleNotificationAsync({
      content: {
        title: 'SnackPilot',
        body: 'Du bist im Büro, hast aber noch nicht bestellt!',
        sound: 'default',
      },
      trigger: null, // immediate
    });
  } else if (!isAtCompany && hasOrderToday) {
    await Notifications.scheduleNotificationAsync({
      content: {
        title: 'SnackPilot',
        body: 'Du hast heute bestellt, bist aber nicht im Büro!',
        sound: 'default',
      },
      trigger: null, // immediate
    });
  }
}

export { checkAndNotify };
```

**Step 3: Commit**

```bash
git add src/app/src-rn/utils/constants.ts src/app/src-rn/utils/notificationTasks.ts
git commit -m "feat(#25): add background task definitions for geofencing and order sync"
```

---

### Task 5: Create Notification Service — Setup and Teardown Functions

**Files:**
- Create: `src/app/src-rn/utils/notificationService.ts`
- Test: `src/app/src-rn/__tests__/utils/notificationService.test.ts`

**Step 1: Write the failing test**

Create `src/app/src-rn/__tests__/utils/notificationService.test.ts`:

```typescript
jest.mock('expo-location', () => ({
  requestForegroundPermissionsAsync: jest.fn(),
  requestBackgroundPermissionsAsync: jest.fn(),
  getCurrentPositionAsync: jest.fn(),
  startGeofencingAsync: jest.fn(),
  stopGeofencingAsync: jest.fn(),
  hasStartedGeofencingAsync: jest.fn(),
  Accuracy: { High: 4 },
  GeofencingEventType: { Enter: 1, Exit: 2 },
}));

jest.mock('expo-notifications', () => ({
  getPermissionsAsync: jest.fn(),
  requestPermissionsAsync: jest.fn(),
  setNotificationHandler: jest.fn(),
  setNotificationChannelAsync: jest.fn(),
  scheduleNotificationAsync: jest.fn(),
  cancelAllScheduledNotificationsAsync: jest.fn(),
  SchedulableTriggerInputTypes: { DAILY: 'daily' },
  AndroidImportance: { HIGH: 4 },
}));

jest.mock('expo-background-task', () => ({
  getStatusAsync: jest.fn(),
  registerTaskAsync: jest.fn(),
  unregisterTaskAsync: jest.fn(),
  BackgroundTaskStatus: { Available: 2 },
}));

jest.mock('expo-task-manager', () => ({
  isTaskRegisteredAsync: jest.fn(),
  defineTask: jest.fn(),
}));

jest.mock('react-native', () => ({
  Platform: { OS: 'ios', select: (spec: any) => spec.ios ?? spec.default },
}));

jest.mock('../../store/locationStore', () => {
  let state = { companyLocation: null, isAtCompany: false };
  return {
    useLocationStore: {
      getState: () => ({
        ...state,
        hasCompanyLocation: () => state.companyLocation !== null,
      }),
      setState: (newState: any) => { state = { ...state, ...newState }; },
    },
  };
});

import * as Location from 'expo-location';
import * as Notifications from 'expo-notifications';
import * as BackgroundTask from 'expo-background-task';
import * as TaskManager from 'expo-task-manager';
import { useLocationStore } from '../../store/locationStore';

// Must import after all mocks
import {
  requestLocationPermissions,
  requestNotificationPermissions,
  startGeofencing,
  stopGeofencing,
  registerBackgroundSync,
  unregisterBackgroundSync,
  scheduleDailyNotification,
  setupNotificationHandler,
} from '../../utils/notificationService';

beforeEach(() => {
  jest.clearAllMocks();
  useLocationStore.setState({ companyLocation: null, isAtCompany: false });
});

describe('notificationService', () => {
  describe('requestLocationPermissions', () => {
    it('returns true when both foreground and background granted', async () => {
      (Location.requestForegroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      (Location.requestBackgroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });

      const result = await requestLocationPermissions();
      expect(result).toBe(true);
    });

    it('returns false when foreground denied', async () => {
      (Location.requestForegroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'denied' });

      const result = await requestLocationPermissions();
      expect(result).toBe(false);
      expect(Location.requestBackgroundPermissionsAsync).not.toHaveBeenCalled();
    });

    it('returns false when background denied', async () => {
      (Location.requestForegroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      (Location.requestBackgroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'denied' });

      const result = await requestLocationPermissions();
      expect(result).toBe(false);
    });
  });

  describe('requestNotificationPermissions', () => {
    it('returns true when already granted', async () => {
      (Notifications.getPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });

      const result = await requestNotificationPermissions();
      expect(result).toBe(true);
      expect(Notifications.requestPermissionsAsync).not.toHaveBeenCalled();
    });

    it('requests permissions when not yet granted', async () => {
      (Notifications.getPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'undetermined' });
      (Notifications.requestPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });

      const result = await requestNotificationPermissions();
      expect(result).toBe(true);
      expect(Notifications.requestPermissionsAsync).toHaveBeenCalled();
    });
  });

  describe('startGeofencing', () => {
    it('starts geofencing with correct coordinates and radius', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(false);
      useLocationStore.setState({ companyLocation: { latitude: 48.2, longitude: 16.3 } });

      await startGeofencing();

      expect(Location.startGeofencingAsync).toHaveBeenCalledWith(
        'COMPANY_GEOFENCE_TASK',
        [{
          identifier: 'company',
          latitude: 48.2,
          longitude: 16.3,
          radius: 500,
          notifyOnEnter: true,
          notifyOnExit: true,
        }]
      );
    });

    it('does nothing when no company location is set', async () => {
      await startGeofencing();
      expect(Location.startGeofencingAsync).not.toHaveBeenCalled();
    });

    it('stops existing geofencing before restarting', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(true);
      useLocationStore.setState({ companyLocation: { latitude: 48.2, longitude: 16.3 } });

      await startGeofencing();

      expect(Location.stopGeofencingAsync).toHaveBeenCalledBefore(
        Location.startGeofencingAsync as jest.Mock
      );
    });
  });

  describe('stopGeofencing', () => {
    it('stops geofencing when running', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(true);

      await stopGeofencing();

      expect(Location.stopGeofencingAsync).toHaveBeenCalledWith('COMPANY_GEOFENCE_TASK');
    });

    it('does nothing when not running', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(false);

      await stopGeofencing();

      expect(Location.stopGeofencingAsync).not.toHaveBeenCalled();
    });
  });

  describe('registerBackgroundSync', () => {
    it('registers when not already registered', async () => {
      (BackgroundTask.getStatusAsync as jest.Mock).mockResolvedValue(BackgroundTask.BackgroundTaskStatus.Available);
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(false);

      await registerBackgroundSync();

      expect(BackgroundTask.registerTaskAsync).toHaveBeenCalledWith(
        'BACKGROUND_ORDER_SYNC_TASK',
        { minimumInterval: 15 }
      );
    });

    it('skips when already registered', async () => {
      (BackgroundTask.getStatusAsync as jest.Mock).mockResolvedValue(BackgroundTask.BackgroundTaskStatus.Available);
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(true);

      await registerBackgroundSync();

      expect(BackgroundTask.registerTaskAsync).not.toHaveBeenCalled();
    });
  });

  describe('scheduleDailyNotification', () => {
    it('schedules a daily notification at 8:45', async () => {
      await scheduleDailyNotification();

      expect(Notifications.cancelAllScheduledNotificationsAsync).toHaveBeenCalled();
      expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith({
        content: {
          title: 'SnackPilot',
          body: 'Bestellungs-Check läuft...',
          sound: 'default',
        },
        trigger: {
          type: 'daily',
          hour: 8,
          minute: 45,
          channelId: 'order-reminders',
        },
      });
    });
  });
});
```

**Step 2: Run test to verify it fails**

Run: `cd src/app && npx jest src-rn/__tests__/utils/notificationService.test.ts -v`

Expected: FAIL — Cannot find module `../../utils/notificationService`

**Step 3: Write the implementation**

Create `src/app/src-rn/utils/notificationService.ts`:

```typescript
import { Platform } from 'react-native';
import * as Location from 'expo-location';
import * as Notifications from 'expo-notifications';
import * as BackgroundTask from 'expo-background-task';
import * as TaskManager from 'expo-task-manager';
import { useLocationStore } from '../store/locationStore';
import {
  GEOFENCE_TASK_NAME,
  BACKGROUND_ORDER_SYNC_TASK,
  COMPANY_GEOFENCE_RADIUS_M,
  NOTIFICATION_HOUR,
  NOTIFICATION_MINUTE,
  NOTIFICATION_CHANNEL_ID,
} from './constants';

export async function requestLocationPermissions(): Promise<boolean> {
  const fg = await Location.requestForegroundPermissionsAsync();
  if (fg.status !== 'granted') return false;
  const bg = await Location.requestBackgroundPermissionsAsync();
  return bg.status === 'granted';
}

export async function requestNotificationPermissions(): Promise<boolean> {
  const { status: existing } = await Notifications.getPermissionsAsync();
  if (existing === 'granted') return true;
  const { status } = await Notifications.requestPermissionsAsync({
    ios: { allowAlert: true, allowBadge: true, allowSound: true },
  });
  return status === 'granted';
}

export async function startGeofencing(): Promise<void> {
  const { companyLocation } = useLocationStore.getState();
  if (!companyLocation) return;

  const isRunning = await Location.hasStartedGeofencingAsync(GEOFENCE_TASK_NAME);
  if (isRunning) {
    await Location.stopGeofencingAsync(GEOFENCE_TASK_NAME);
  }

  await Location.startGeofencingAsync(GEOFENCE_TASK_NAME, [
    {
      identifier: 'company',
      latitude: companyLocation.latitude,
      longitude: companyLocation.longitude,
      radius: COMPANY_GEOFENCE_RADIUS_M,
      notifyOnEnter: true,
      notifyOnExit: true,
    },
  ]);
}

export async function stopGeofencing(): Promise<void> {
  const isRunning = await Location.hasStartedGeofencingAsync(GEOFENCE_TASK_NAME);
  if (isRunning) {
    await Location.stopGeofencingAsync(GEOFENCE_TASK_NAME);
  }
}

export async function registerBackgroundSync(): Promise<void> {
  const status = await BackgroundTask.getStatusAsync();
  if (status === BackgroundTask.BackgroundTaskStatus.Restricted) return;

  const isRegistered = await TaskManager.isTaskRegisteredAsync(BACKGROUND_ORDER_SYNC_TASK);
  if (!isRegistered) {
    await BackgroundTask.registerTaskAsync(BACKGROUND_ORDER_SYNC_TASK, {
      minimumInterval: 15,
    });
  }
}

export async function unregisterBackgroundSync(): Promise<void> {
  const isRegistered = await TaskManager.isTaskRegisteredAsync(BACKGROUND_ORDER_SYNC_TASK);
  if (isRegistered) {
    await BackgroundTask.unregisterTaskAsync(BACKGROUND_ORDER_SYNC_TASK);
  }
}

export async function scheduleDailyNotification(): Promise<void> {
  await Notifications.cancelAllScheduledNotificationsAsync();
  await Notifications.scheduleNotificationAsync({
    content: {
      title: 'SnackPilot',
      body: 'Bestellungs-Check läuft...',
      sound: 'default',
    },
    trigger: {
      type: Notifications.SchedulableTriggerInputTypes.DAILY,
      hour: NOTIFICATION_HOUR,
      minute: NOTIFICATION_MINUTE,
      channelId: NOTIFICATION_CHANNEL_ID,
    },
  });
}

export async function cancelDailyNotification(): Promise<void> {
  await Notifications.cancelAllScheduledNotificationsAsync();
}

export function setupNotificationHandler(): void {
  Notifications.setNotificationHandler({
    handleNotification: async () => ({
      shouldShowBanner: true,
      shouldShowList: true,
      shouldPlaySound: true,
      shouldSetBadge: false,
    }),
  });
}

export async function setupAndroidChannel(): Promise<void> {
  if (Platform.OS === 'android') {
    await Notifications.setNotificationChannelAsync(NOTIFICATION_CHANNEL_ID, {
      name: 'Bestellungs-Erinnerungen',
      importance: Notifications.AndroidImportance.HIGH,
      vibrationPattern: [0, 250, 250, 250],
    });
  }
}

export async function getCurrentPosition(): Promise<{ latitude: number; longitude: number }> {
  const location = await Location.getCurrentPositionAsync({
    accuracy: Location.Accuracy.High,
  });
  return {
    latitude: location.coords.latitude,
    longitude: location.coords.longitude,
  };
}

/**
 * Full setup: called when a company location is saved.
 * Starts geofencing, registers background sync, schedules daily notification.
 */
export async function enableNotifications(): Promise<void> {
  await startGeofencing();
  await registerBackgroundSync();
  await scheduleDailyNotification();
}

/**
 * Full teardown: called when company location is removed.
 * Stops geofencing, unregisters background sync, cancels notifications.
 */
export async function disableNotifications(): Promise<void> {
  await stopGeofencing();
  await unregisterBackgroundSync();
  await cancelDailyNotification();
}
```

**Step 4: Run test to verify it passes**

Run: `cd src/app && npx jest src-rn/__tests__/utils/notificationService.test.ts -v`

Expected: All tests PASS

**Step 5: Run full test suite**

Run: `cd src/app && npm test`

Expected: All tests pass (existing + new).

**Step 6: Commit**

```bash
git add src/app/src-rn/utils/notificationService.ts src/app/src-rn/__tests__/utils/notificationService.test.ts
git commit -m "feat(#25): add notification service with geofencing, background sync, and daily schedule"
```

---

### Task 6: Integrate Task Definitions into App Entry Point

**Files:**
- Modify: `src/app/app/_layout.tsx`

**Step 1: Import task definitions and initialize notification handler**

Add this import right after the `tauriHttp` import (must be early, at module scope):

```typescript
import '../src-rn/utils/notificationTasks';
```

Inside `AppContent`, add a new `useEffect` for notification initialization. After the existing login useEffect, add:

```typescript
import { Platform } from 'react-native';
import { isNative } from '../src-rn/utils/platform';
import { useLocationStore } from '../src-rn/store/locationStore';
import {
  setupNotificationHandler,
  setupAndroidChannel,
  enableNotifications,
} from '../src-rn/utils/notificationService';
```

And inside `AppContent`, add:

```typescript
useEffect(() => {
  if (!isNative()) return;
  setupNotificationHandler();
  setupAndroidChannel();
}, []);

// Restore geofencing on app start if company location is saved
const hasCompanyLocation = useLocationStore((s) => s.companyLocation !== null);

useEffect(() => {
  if (!isNative() || !hasCompanyLocation) return;
  enableNotifications();
}, [hasCompanyLocation]);
```

The final `AppContent` should look like:

```typescript
function AppContent() {
  const gourmetLoginWithSaved = useAuthStore((s) => s.loginWithSaved);
  const ventopayLoginWithSaved = useVentopayAuthStore((s) => s.loginWithSaved);
  const { colorScheme } = useTheme();
  const hasCompanyLocation = useLocationStore((s) => s.companyLocation !== null);

  useEffect(() => {
    gourmetLoginWithSaved();
    ventopayLoginWithSaved();
  }, [gourmetLoginWithSaved, ventopayLoginWithSaved]);

  useEffect(() => {
    if (Platform.OS === 'web') {
      document.documentElement.style.colorScheme = colorScheme;
    }
  }, [colorScheme]);

  useEffect(() => {
    if (!isNative()) return;
    setupNotificationHandler();
    setupAndroidChannel();
  }, []);

  useEffect(() => {
    if (!isNative() || !hasCompanyLocation) return;
    enableNotifications();
  }, [hasCompanyLocation]);

  return (
    <DialogProvider>
      <Stack>
        <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
      </Stack>
      <StatusBar style="auto" />
    </DialogProvider>
  );
}
```

**Step 2: Run existing tests to check nothing is broken**

Run: `cd src/app && npm test`

Expected: All tests pass.

**Step 3: Commit**

```bash
git add src/app/app/_layout.tsx
git commit -m "feat(#25): integrate notification tasks and restore geofencing on app start"
```

---

### Task 7: Add Location Section to Settings Screen

**Files:**
- Modify: `src/app/app/(tabs)/settings.tsx`

**Step 1: Add imports**

Add at the top of `settings.tsx`:

```typescript
import { isNative } from '../../src-rn/utils/platform';
import { useLocationStore } from '../../src-rn/store/locationStore';
import {
  requestLocationPermissions,
  requestNotificationPermissions,
  getCurrentPosition,
  enableNotifications,
  disableNotifications,
} from '../../src-rn/utils/notificationService';
```

**Step 2: Add state and handlers inside SettingsScreen**

After the existing store subscriptions (around line 84), add:

```typescript
// Location notifications (mobile only)
const companyLocation = useLocationStore((s) => s.companyLocation);
const setCompanyLocation = useLocationStore((s) => s.setCompanyLocation);
const clearCompanyLocation = useLocationStore((s) => s.clearCompanyLocation);
const [locationSaving, setLocationSaving] = useState(false);

const handleSetLocation = async () => {
  setLocationSaving(true);
  try {
    const locGranted = await requestLocationPermissions();
    if (!locGranted) {
      alert('Berechtigung fehlt', 'Standortzugriff (immer) wird für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.');
      setLocationSaving(false);
      return;
    }
    const notifGranted = await requestNotificationPermissions();
    if (!notifGranted) {
      alert('Berechtigung fehlt', 'Benachrichtigungen werden für diese Funktion benötigt. Bitte in den Einstellungen aktivieren.');
      setLocationSaving(false);
      return;
    }
    const position = await getCurrentPosition();
    setCompanyLocation(position.latitude, position.longitude);
    await enableNotifications();
    alert('Gespeichert', 'Firmenstandort gesetzt. Du wirst um 8:45 benachrichtigt, wenn du im Büro bist und nicht bestellt hast.');
  } catch {
    alert('Fehler', 'Standort konnte nicht ermittelt werden.');
  }
  setLocationSaving(false);
};

const handleRemoveLocation = async () => {
  clearCompanyLocation();
  await disableNotifications();
};
```

**Step 3: Add the location card JSX**

After the `updatesCard` definition (before the `return`), add:

```typescript
const locationCard = isNative() ? (
  <View style={isWideLayout ? styles.desktopCard : undefined}>
    {!isWideLayout && <View style={styles.divider} />}
    <Text style={styles.sectionTitle}>Standort-Benachrichtigungen</Text>
    <Text style={styles.sectionSubtitle}>
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

**Step 4: Add locationCard to the render tree**

In the `return` JSX, insert `{locationCard}` after `{appearanceCard}` in both layouts:

For the wide layout:
```jsx
<View style={styles.desktopRow}>
  {appearanceCard}
  {updatesCard ?? locationCard}
</View>
{updatesCard && locationCard}
```

For the mobile layout:
```jsx
{appearanceCard}
{locationCard}
{updatesCard}
```

**Step 5: Run existing tests**

Run: `cd src/app && npm test`

Expected: All tests pass.

**Step 6: Commit**

```bash
git add src/app/app/(tabs)/settings.tsx
git commit -m "feat(#25): add location notification section to settings (mobile only)"
```

---

### Task 8: Rebuild Native Apps and Test

This task requires a physical device or simulator.

**Step 1: Rebuild iOS**

Run: `cd src/app && npx expo run:ios`

This is needed because we added native modules (expo-location, expo-notifications, etc.) that require a native rebuild.

**Step 2: Verify settings UI**

1. Open the app on iOS simulator
2. Navigate to Settings tab
3. Verify "Standort-Benachrichtigungen" section appears
4. Verify it shows "Aktuellen Standort als Firmenstandort setzen" button

**Step 3: Test set location flow**

1. Tap "Aktuellen Standort als Firmenstandort setzen"
2. Grant location permission (foreground then background)
3. Grant notification permission
4. Verify success alert appears
5. Verify button changes to "Firmenstandort gesetzt" with "Standort entfernen"

**Step 4: Test remove location flow**

1. Tap "Standort entfernen"
2. Verify it returns to the "Aktuellen Standort als Firmenstandort setzen" button

**Step 5: Verify the section does NOT appear on web/desktop**

Run: `cd src/app && npx expo start --web`

Navigate to Settings — the location section should not be visible.

**Step 6: Run full test suite one final time**

Run: `cd src/app && npm test`

Expected: All tests pass.

**Step 7: Commit any fixes needed**

Only if adjustments were needed during testing.

---

### Task 9: Final Verification and Summary Commit

**Step 1: Run full test suite**

Run: `cd src/app && npm test`

Expected: All tests pass.

**Step 2: Review all changes**

Run: `git diff main --stat`

Verify the following files were changed:
- `src/app/package.json` — new dependencies
- `src/app/app.json` — plugins and permissions
- `src/app/src-rn/store/locationStore.ts` — new file
- `src/app/src-rn/utils/constants.ts` — new constants
- `src/app/src-rn/utils/notificationTasks.ts` — new file
- `src/app/src-rn/utils/notificationService.ts` — new file
- `src/app/app/_layout.tsx` — initialization
- `src/app/app/(tabs)/settings.tsx` — UI
- `src/app/src-rn/__tests__/store/locationStore.test.ts` — new test
- `src/app/src-rn/__tests__/utils/notificationService.test.ts` — new test

**Step 3: No other files should have been modified**

Verify no web scraping files were touched. The API layer, parsers, and client files must be untouched.
