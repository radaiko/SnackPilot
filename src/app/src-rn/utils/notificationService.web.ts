// Web/Desktop stub — notification features are native-only.

export async function requestLocationPermissions(): Promise<boolean> {
  return false;
}

export async function requestNotificationPermissions(): Promise<boolean> {
  return false;
}

export async function startGeofencing(): Promise<void> {}

export async function stopGeofencing(): Promise<void> {}

export async function registerBackgroundSync(): Promise<void> {}

export async function unregisterBackgroundSync(): Promise<void> {}

export async function scheduleDailyNotification(): Promise<void> {}

export async function cancelDailyNotification(): Promise<void> {}

export function setupNotificationHandler(): void {}

export async function setupAndroidChannel(): Promise<void> {}

export async function getCurrentPosition(): Promise<{ latitude: number; longitude: number }> {
  throw new Error('Location not available on web');
}

export async function enableNotifications(): Promise<void> {}

export async function disableNotifications(): Promise<void> {}
