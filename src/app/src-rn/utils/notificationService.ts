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

export async function enableNotifications(): Promise<void> {
  await startGeofencing();
  await registerBackgroundSync();
  await scheduleDailyNotification();
}

export async function disableNotifications(): Promise<void> {
  await stopGeofencing();
  await unregisterBackgroundSync();
  await cancelDailyNotification();
}
