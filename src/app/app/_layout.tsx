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
} from '../src-rn/utils/notificationService';

const backgroundMenuCheck = isNative()
  ? require('../src-rn/utils/backgroundMenuCheck')
  : null;

function AppContent() {
  const gourmetLoginWithSaved = useAuthStore((s) => s.loginWithSaved);
  const ventopayLoginWithSaved = useVentopayAuthStore((s) => s.loginWithSaved);
  const { colorScheme } = useTheme();

  useEffect(() => {
    gourmetLoginWithSaved();
    ventopayLoginWithSaved();
  }, [gourmetLoginWithSaved, ventopayLoginWithSaved]);

  useEffect(() => {
    if (!backgroundMenuCheck) return;
    backgroundMenuCheck.registerBackgroundMenuCheck();
    backgroundMenuCheck.requestNotificationPermissions();
  }, []);

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

  return (
    <DialogProvider>
      <Stack>
        <Stack.Screen name="(tabs)" options={{ headerShown: false }} />
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
