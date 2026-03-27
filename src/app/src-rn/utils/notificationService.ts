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
  GEOFENCE_NOTIFICATION_ID,
  DAILY_REMINDER_NOTIFICATION_ID,
} from './constants';
import { viennaMinutes } from './dateUtils';

export async function requestLocationPermissions(): Promise<boolean> {
  const fg = await Location.requestForegroundPermissionsAsync();
  if (fg.status !== 'granted') return false;
  const bg = await Location.requestBackgroundPermissionsAsync();
  if (bg.status === 'granted') return true;
  // iOS may not show the "Always Allow" prompt — return 'needs-settings'
  return false;
}

/**
 * Check if background location is already granted (without prompting).
 * Use this to detect if the user needs to go to Settings.
 */
export async function hasBackgroundLocationPermission(): Promise<boolean> {
  const bg = await Location.getBackgroundPermissionsAsync();
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

  // Skip if already running — restarting causes iOS to re-fire Enter events
  // when the device is already inside the zone, which triggers spurious notifications.
  const isRunning = await Location.hasStartedGeofencingAsync(GEOFENCE_TASK_NAME);
  if (isRunning) return;

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

/**
 * Schedule the geofence "no order" notification for today at the configured time.
 * If the target time has already passed (but before 14:00), fires immediately.
 * Returns true if a notification was scheduled/fired, false if skipped.
 */
export async function scheduleGeofenceNotification(): Promise<boolean> {
  const currentMin = viennaMinutes();
  const targetMin = NOTIFICATION_HOUR * 60 + NOTIFICATION_MINUTE;

  // Too late in the day (past 14:00) — skip
  if (currentMin >= 14 * 60) return false;

  if (currentMin < targetMin) {
    // Schedule for the target time
    const deltaMs = (targetMin - currentMin) * 60 * 1000;
    const targetDate = new Date(Date.now() + deltaMs);
    await Notifications.scheduleNotificationAsync({
      identifier: GEOFENCE_NOTIFICATION_ID,
      content: {
        title: 'SnackPilot',
        body: 'Du bist im Büro, hast aber noch nicht bestellt!',
        sound: 'default',
      },
      trigger: {
        type: Notifications.SchedulableTriggerInputTypes.DATE,
        date: targetDate,
        channelId: NOTIFICATION_CHANNEL_ID,
      },
    });
  } else {
    // Already past target time — fire immediately
    await Notifications.scheduleNotificationAsync({
      identifier: GEOFENCE_NOTIFICATION_ID,
      content: {
        title: 'SnackPilot',
        body: 'Du bist im Büro, hast aber noch nicht bestellt!',
        sound: 'default',
      },
      trigger: null,
    });
  }
  return true;
}

/**
 * Cancel a previously scheduled geofence notification.
 */
export async function cancelGeofenceNotification(): Promise<void> {
  await Notifications.cancelScheduledNotificationAsync(GEOFENCE_NOTIFICATION_ID);
}

/**
 * Schedule the daily reminder notification at the given time with order content.
 * If the target time has already passed, fires immediately.
 * Re-scheduling with the same identifier replaces any previous notification.
 */
export async function scheduleDailyReminderNotification(
  targetHour: number,
  targetMinute: number,
  body: string,
): Promise<boolean> {
  const currentMin = viennaMinutes();
  const targetMin = targetHour * 60 + targetMinute;

  if (currentMin < targetMin) {
    const deltaMs = (targetMin - currentMin) * 60 * 1000;
    const targetDate = new Date(Date.now() + deltaMs);
    await Notifications.scheduleNotificationAsync({
      identifier: DAILY_REMINDER_NOTIFICATION_ID,
      content: {
        title: 'Deine Bestellung heute',
        body,
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: {
        type: Notifications.SchedulableTriggerInputTypes.DATE,
        date: targetDate,
        channelId: NOTIFICATION_CHANNEL_ID,
      },
    });
  } else {
    // Already past target time — fire immediately
    await Notifications.scheduleNotificationAsync({
      identifier: DAILY_REMINDER_NOTIFICATION_ID,
      content: {
        title: 'Deine Bestellung heute',
        body,
        sound: 'default',
        data: { screen: '/(tabs)/orders' },
      },
      trigger: null,
    });
  }
  return true;
}

/**
 * Cancel a previously scheduled daily reminder notification.
 */
export async function cancelDailyReminderNotification(): Promise<void> {
  await Notifications.cancelScheduledNotificationAsync(DAILY_REMINDER_NOTIFICATION_ID);
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

export async function enableNotifications(): Promise<void> {
  await startGeofencing();
  await registerBackgroundSync();
}

export async function disableNotifications(): Promise<void> {
  await stopGeofencing();
  await unregisterBackgroundSync();
  await cancelDailyNotification();
}
