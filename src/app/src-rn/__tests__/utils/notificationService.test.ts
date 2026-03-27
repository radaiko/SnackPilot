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
  cancelScheduledNotificationAsync: jest.fn(),
  SchedulableTriggerInputTypes: { DAILY: 'daily', DATE: 'date' },
  AndroidImportance: { HIGH: 4 },
}));

jest.mock('expo-background-task', () => ({
  getStatusAsync: jest.fn(),
  registerTaskAsync: jest.fn(),
  unregisterTaskAsync: jest.fn(),
  BackgroundTaskStatus: { Available: 2, Restricted: 1 },
}));

jest.mock('expo-task-manager', () => ({
  isTaskRegisteredAsync: jest.fn(),
  defineTask: jest.fn(),
}));

jest.mock('react-native', () => ({
  Platform: { OS: 'ios', select: (spec: any) => spec.ios ?? spec.default },
}));

jest.mock('../../utils/dateUtils', () => ({
  viennaMinutes: jest.fn(() => 480), // default: 8:00
}));

jest.mock('../../store/locationStore', () => {
  let state: { companyLocation: { latitude: number; longitude: number } | null; isAtCompany: boolean } = { companyLocation: null, isAtCompany: false };
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

import { viennaMinutes } from '../../utils/dateUtils';
import {
  requestLocationPermissions,
  requestNotificationPermissions,
  startGeofencing,
  stopGeofencing,
  registerBackgroundSync,
  unregisterBackgroundSync,
  scheduleDailyNotification,
  cancelDailyNotification,
  scheduleGeofenceNotification,
  cancelGeofenceNotification,
  setupNotificationHandler,
  setupAndroidChannel,
  getCurrentPosition,
  enableNotifications,
  disableNotifications,
} from '../../utils/notificationService';

beforeEach(() => {
  jest.clearAllMocks();
  (useLocationStore as any).setState({ companyLocation: null, isAtCompany: false });
});

describe('notificationService', () => {
  describe('requestLocationPermissions', () => {
    it('returns true when both foreground and background granted', async () => {
      (Location.requestForegroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      (Location.requestBackgroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      expect(await requestLocationPermissions()).toBe(true);
    });

    it('returns false when foreground denied', async () => {
      (Location.requestForegroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'denied' });
      expect(await requestLocationPermissions()).toBe(false);
      expect(Location.requestBackgroundPermissionsAsync).not.toHaveBeenCalled();
    });

    it('returns false when background denied', async () => {
      (Location.requestForegroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      (Location.requestBackgroundPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'denied' });
      expect(await requestLocationPermissions()).toBe(false);
    });
  });

  describe('requestNotificationPermissions', () => {
    it('returns true when already granted', async () => {
      (Notifications.getPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      expect(await requestNotificationPermissions()).toBe(true);
      expect(Notifications.requestPermissionsAsync).not.toHaveBeenCalled();
    });

    it('requests permissions when not yet granted', async () => {
      (Notifications.getPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'undetermined' });
      (Notifications.requestPermissionsAsync as jest.Mock).mockResolvedValue({ status: 'granted' });
      expect(await requestNotificationPermissions()).toBe(true);
      expect(Notifications.requestPermissionsAsync).toHaveBeenCalled();
    });
  });

  describe('startGeofencing', () => {
    it('starts geofencing with correct coordinates and radius', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(false);
      (useLocationStore as any).setState({ companyLocation: { latitude: 48.2, longitude: 16.3 } });

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

    it('skips starting when geofencing is already running', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(true);
      (useLocationStore as any).setState({ companyLocation: { latitude: 48.2, longitude: 16.3 } });

      await startGeofencing();

      expect(Location.stopGeofencingAsync).not.toHaveBeenCalled();
      expect(Location.startGeofencingAsync).not.toHaveBeenCalled();
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
    it('registers when available and not already registered', async () => {
      (BackgroundTask.getStatusAsync as jest.Mock).mockResolvedValue(2); // Available
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(false);

      await registerBackgroundSync();

      expect(BackgroundTask.registerTaskAsync).toHaveBeenCalledWith(
        'BACKGROUND_ORDER_SYNC_TASK',
        { minimumInterval: 15 }
      );
    });

    it('skips when already registered', async () => {
      (BackgroundTask.getStatusAsync as jest.Mock).mockResolvedValue(2);
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(true);

      await registerBackgroundSync();

      expect(BackgroundTask.registerTaskAsync).not.toHaveBeenCalled();
    });

    it('skips when restricted', async () => {
      (BackgroundTask.getStatusAsync as jest.Mock).mockResolvedValue(1); // Restricted

      await registerBackgroundSync();

      expect(TaskManager.isTaskRegisteredAsync).not.toHaveBeenCalled();
      expect(BackgroundTask.registerTaskAsync).not.toHaveBeenCalled();
    });
  });

  describe('unregisterBackgroundSync', () => {
    it('unregisters when registered', async () => {
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(true);
      await unregisterBackgroundSync();
      expect(BackgroundTask.unregisterTaskAsync).toHaveBeenCalledWith('BACKGROUND_ORDER_SYNC_TASK');
    });

    it('does nothing when not registered', async () => {
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(false);
      await unregisterBackgroundSync();
      expect(BackgroundTask.unregisterTaskAsync).not.toHaveBeenCalled();
    });
  });

  describe('scheduleDailyNotification', () => {
    it('cancels existing and schedules at 8:45', async () => {
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

  describe('cancelDailyNotification', () => {
    it('cancels all scheduled notifications', async () => {
      await cancelDailyNotification();
      expect(Notifications.cancelAllScheduledNotificationsAsync).toHaveBeenCalled();
    });
  });

  describe('setupNotificationHandler', () => {
    it('sets the notification handler', () => {
      setupNotificationHandler();
      expect(Notifications.setNotificationHandler).toHaveBeenCalled();
    });
  });

  describe('getCurrentPosition', () => {
    it('returns lat/lng from device location', async () => {
      (Location.getCurrentPositionAsync as jest.Mock).mockResolvedValue({
        coords: { latitude: 48.2082, longitude: 16.3738, accuracy: 10 },
        timestamp: Date.now(),
      });

      const pos = await getCurrentPosition();
      expect(pos).toEqual({ latitude: 48.2082, longitude: 16.3738 });
    });
  });

  describe('enableNotifications', () => {
    it('starts geofencing and registers background sync', async () => {
      (useLocationStore as any).setState({ companyLocation: { latitude: 48.2, longitude: 16.3 } });
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(false);
      (BackgroundTask.getStatusAsync as jest.Mock).mockResolvedValue(2);
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(false);

      await enableNotifications();

      expect(Location.startGeofencingAsync).toHaveBeenCalled();
      expect(BackgroundTask.registerTaskAsync).toHaveBeenCalled();
    });
  });

  describe('disableNotifications', () => {
    it('stops geofencing, unregisters background sync, and cancels notifications', async () => {
      (Location.hasStartedGeofencingAsync as jest.Mock).mockResolvedValue(true);
      (TaskManager.isTaskRegisteredAsync as jest.Mock).mockResolvedValue(true);

      await disableNotifications();

      expect(Location.stopGeofencingAsync).toHaveBeenCalled();
      expect(BackgroundTask.unregisterTaskAsync).toHaveBeenCalled();
      expect(Notifications.cancelAllScheduledNotificationsAsync).toHaveBeenCalled();
    });
  });

  describe('scheduleGeofenceNotification', () => {
    it('schedules for target time when current time is before target', async () => {
      (viennaMinutes as jest.Mock).mockReturnValue(480); // 8:00, target is 8:45

      const result = await scheduleGeofenceNotification();

      expect(result).toBe(true);
      expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          identifier: 'geofence-no-order-reminder',
          content: expect.objectContaining({
            body: 'Du bist im Büro, hast aber noch nicht bestellt!',
          }),
          trigger: expect.objectContaining({
            type: 'date',
          }),
        })
      );
    });

    it('fires immediately when current time is past target but before 14:00', async () => {
      (viennaMinutes as jest.Mock).mockReturnValue(600); // 10:00

      const result = await scheduleGeofenceNotification();

      expect(result).toBe(true);
      expect(Notifications.scheduleNotificationAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          identifier: 'geofence-no-order-reminder',
          trigger: null,
        })
      );
    });

    it('skips when current time is past 14:00', async () => {
      (viennaMinutes as jest.Mock).mockReturnValue(840 + 1); // 14:01

      const result = await scheduleGeofenceNotification();

      expect(result).toBe(false);
      expect(Notifications.scheduleNotificationAsync).not.toHaveBeenCalled();
    });
  });

  describe('cancelGeofenceNotification', () => {
    it('cancels the geofence notification by identifier', async () => {
      await cancelGeofenceNotification();

      expect(Notifications.cancelScheduledNotificationAsync).toHaveBeenCalledWith(
        'geofence-no-order-reminder'
      );
    });
  });
});
