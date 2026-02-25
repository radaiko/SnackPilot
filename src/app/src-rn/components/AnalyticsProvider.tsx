import { ReactNode, useEffect, useRef } from 'react';
import { AccessibilityInfo, AppState, Appearance, Dimensions, PixelRatio, Platform } from 'react-native';
import { TelemetryDeckProvider } from '@typedigital/telemetrydeck-react';
import Constants from 'expo-constants';
import * as Device from 'expo-device';
import { getLocales, getCalendars } from 'expo-localization';
import { td, setDefaultPayload, trackSignal } from '../utils/analytics';

function getOsName(): string {
  return Platform.OS === 'ios' ? 'iOS' : 'Android';
}

function getSystemVersion(): string {
  if (Platform.OS === 'ios') return String(Platform.Version);
  return String(
    (Platform.constants as Record<string, unknown>).Release ?? Platform.Version,
  );
}

function getMajorVersion(version: string): string {
  return version.split('.')[0] ?? version;
}

function getMajorMinorVersion(version: string): string {
  const parts = version.split('.');
  return parts.length >= 2 ? `${parts[0]}.${parts[1]}` : version;
}

function getOrientation(): string {
  const { width, height } = Dimensions.get('window');
  return height >= width ? 'Portrait' : 'Landscape';
}

async function collectAccessibility(): Promise<Record<string, string>> {
  const params: Record<string, string> = {};
  try {
    const reduceMotion = await AccessibilityInfo.isReduceMotionEnabled();
    params['TelemetryDeck.Accessibility.isReduceMotionEnabled'] = String(reduceMotion);

    const screenReader = await AccessibilityInfo.isScreenReaderEnabled();
    params['TelemetryDeck.Accessibility.isScreenReaderEnabled'] = String(screenReader);

    if (Platform.OS === 'ios') {
      const boldText = await AccessibilityInfo.isBoldTextEnabled();
      params['TelemetryDeck.Accessibility.isBoldTextEnabled'] = String(boldText);

      const invertColors = await AccessibilityInfo.isInvertColorsEnabled();
      params['TelemetryDeck.Accessibility.isInvertColorsEnabled'] = String(invertColors);

      const reduceTransparency = await AccessibilityInfo.isReduceTransparencyEnabled();
      params['TelemetryDeck.Accessibility.isReduceTransparencyEnabled'] = String(reduceTransparency);

      const darkerColors = await AccessibilityInfo.isDarkerSystemColorsEnabled();
      params['TelemetryDeck.Accessibility.isDarkerSystemColorsEnabled'] = String(darkerColors);
    }
  } catch {
    // Accessibility queries can fail on some devices
  }
  return params;
}

function LifecycleSignals() {
  const hasLaunched = useRef(false);
  const appState = useRef(AppState.currentState);

  useEffect(() => {
    if (!hasLaunched.current) {
      hasLaunched.current = true;

      const appVersion = Constants.expoConfig?.version ?? 'unknown';
      const buildNumber = Constants.expoConfig?.ios?.buildNumber
        ?? Constants.expoConfig?.android?.versionCode?.toString()
        ?? 'unknown';
      const osName = getOsName();
      const systemVersion = getSystemVersion();
      const { width, height } = Dimensions.get('window');
      const scale = PixelRatio.get();

      const locale = getLocales()[0];
      const calendar = getCalendars()[0];

      const colorScheme = Appearance.getColorScheme();

      // Build static defaults
      const defaults: Record<string, string> = {
        // App info
        'TelemetryDeck.AppInfo.version': appVersion,
        'TelemetryDeck.AppInfo.buildNumber': buildNumber,
        'TelemetryDeck.AppInfo.versionAndBuildNumber': `${appVersion} ${buildNumber}`,

        // Device
        'TelemetryDeck.Device.platform': Platform.OS,
        'TelemetryDeck.Device.operatingSystem': osName,
        'TelemetryDeck.Device.systemVersion': systemVersion,
        'TelemetryDeck.Device.systemMajorVersion': getMajorVersion(systemVersion),
        'TelemetryDeck.Device.systemMajorMinorVersion': getMajorMinorVersion(systemVersion),
        'TelemetryDeck.Device.modelName': Device.modelName ?? 'unknown',
        'TelemetryDeck.Device.brand': Device.brand ?? 'unknown',
        'TelemetryDeck.Device.screenResolutionWidth': String(width),
        'TelemetryDeck.Device.screenResolutionHeight': String(height),
        'TelemetryDeck.Device.screenScaleFactor': String(scale),
        'TelemetryDeck.Device.orientation': getOrientation(),
        'TelemetryDeck.Device.timeZone': calendar?.timeZone ?? Intl.DateTimeFormat().resolvedOptions().timeZone ?? 'unknown',

        // Run context
        'TelemetryDeck.RunContext.isSimulator': String(!Device.isDevice),
        'TelemetryDeck.RunContext.isDebug': String(__DEV__),
        'TelemetryDeck.RunContext.locale': locale?.languageTag ?? 'unknown',
        'TelemetryDeck.RunContext.language': locale?.languageCode ?? 'unknown',

        // User preferences
        'TelemetryDeck.UserPreference.colorScheme': colorScheme === 'dark' ? 'Dark' : 'Light',
        'TelemetryDeck.UserPreference.language': locale?.languageCode ?? 'unknown',
        'TelemetryDeck.UserPreference.layoutDirection': locale?.textDirection === 'rtl' ? 'rightToLeft' : 'leftToRight',
        'TelemetryDeck.UserPreference.region': locale?.regionCode ?? 'unknown',

        // SDK
        'TelemetryDeck.SDK.name': 'JavaScriptSDK',
      };

      // Collect async accessibility data, then set defaults and fire launch signal
      collectAccessibility().then((a11y) => {
        Object.assign(defaults, a11y);
        setDefaultPayload(defaults);

        trackSignal('app.launched', { startType: 'cold' });
      });
    }

    const subscription = AppState.addEventListener('change', (nextState) => {
      if (appState.current.match(/inactive|background/) && nextState === 'active') {
        trackSignal('app.foregrounded');
      } else if (appState.current === 'active' && nextState.match(/inactive|background/)) {
        trackSignal('app.backgrounded');
      }
      appState.current = nextState;
    });

    return () => subscription.remove();
  }, []);

  return null;
}

export function AnalyticsProvider({ children }: { children: ReactNode }) {
  return (
    <TelemetryDeckProvider telemetryDeck={td}>
      <LifecycleSignals />
      {children}
    </TelemetryDeckProvider>
  );
}
