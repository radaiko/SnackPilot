import { useEffect } from 'react';
import { Platform } from 'react-native';
import { Stack } from 'expo-router';
import { StatusBar } from 'expo-status-bar';
// IMPORTANT: tauriHttp must be imported BEFORE any store modules.
// The module patches axios.create at load time so Zustand stores get
// Tauri-aware Axios instances when they call axios.create() during init.
import '../src-rn/utils/tauriHttp';
import '../src-rn/utils/notificationTasks';
import { useAuthStore } from '../src-rn/store/authStore';
import { useVentopayAuthStore } from '../src-rn/store/ventopayAuthStore';
import { useLocationStore } from '../src-rn/store/locationStore';
import { useTheme } from '../src-rn/theme/useTheme';
import { DialogProvider } from '../src-rn/components/DialogProvider';
import { AnalyticsProvider } from '../src-rn/components/AnalyticsProvider';
import { isNative } from '../src-rn/utils/platform';
import {
  setupNotificationHandler,
  setupAndroidChannel,
  enableNotifications,
  registerBackgroundSync,
} from '../src-rn/utils/notificationService';
import { getReminderEnabled } from '../src-rn/utils/reminderStorage';
import { migrateKeychainAccessibility } from '../src-rn/utils/secureStorage';

const backgroundMenuCheck = isNative()
  ? require('../src-rn/utils/backgroundMenuCheck')
  : null;

function AppContent() {
  const gourmetLoginWithSaved = useAuthStore((s) => s.loginWithSaved);
  const ventopayLoginWithSaved = useVentopayAuthStore((s) => s.loginWithSaved);
  const gourmetAuthStatus = useAuthStore((s) => s.status);
  const { colorScheme } = useTheme();

  useEffect(() => {
    void (async () => {
      if (isNative()) {
        await migrateKeychainAccessibility([
          'gourmet_username', 'gourmet_password',
          'ventopay_username', 'ventopay_password',
        ]);
      }
      gourmetLoginWithSaved();
      ventopayLoginWithSaved();
    })();
  }, [gourmetLoginWithSaved, ventopayLoginWithSaved]);

  useEffect(() => {
    if (!backgroundMenuCheck || gourmetAuthStatus !== 'authenticated') return;
    void (async () => {
      try {
        await backgroundMenuCheck.registerBackgroundMenuCheck();
        await backgroundMenuCheck.requestNotificationPermissions();
      } catch {
        // Silent — background registration is best-effort
      }
    })();
  }, [gourmetAuthStatus]);

  useEffect(() => {
    if (Platform.OS === 'web') {
      document.documentElement.style.colorScheme = colorScheme;
    }
  }, [colorScheme]);

  const hasCompanyLocation = useLocationStore((s) => s.companyLocation !== null);

  useEffect(() => {
    if (!isNative()) return;
    setupNotificationHandler();
    setupAndroidChannel();
  }, []);

  useEffect(() => {
    if (!isNative() || !hasCompanyLocation) return;
    enableNotifications();
  }, [hasCompanyLocation]);

  // Register background sync when daily reminder is enabled (even without company location)
  useEffect(() => {
    if (!isNative()) return;
    void (async () => {
      try {
        const reminderEnabled = await getReminderEnabled();
        if (reminderEnabled) {
          await registerBackgroundSync();
        }
      } catch {
        // Silent — background registration is best-effort
      }
    })();
  }, []);

  return (
    <DialogProvider>
      <Stack>
        <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
        <Stack.Screen name="kantine-login" options={{ headerShown: false, presentation: 'card' }} />
        <Stack.Screen name="automaten-login" options={{ headerShown: false, presentation: 'card' }} />
        <Stack.Screen name="notifications" options={{ headerShown: false, presentation: 'card' }} />
        <Stack.Screen name="appearance" options={{ headerShown: false, presentation: 'card' }} />
      </Stack>
      <StatusBar style="auto" />
    </DialogProvider>
  );
}

export default function RootLayout() {
  if (__DEV__) {
    return <AppContent />;
  }

  return (
    <AnalyticsProvider>
      <AppContent />
    </AnalyticsProvider>
  );
}
