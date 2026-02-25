import { ReactNode, useEffect, useRef } from 'react';
import { TelemetryDeckProvider } from '@typedigital/telemetrydeck-react';
import Constants from 'expo-constants';
import { getAppPlatform } from '../utils/platform';
import { td, setDefaultPayload, trackSignal } from '../utils/analytics';

function getColorScheme(): string {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'Dark' : 'Light';
}

function getOrientation(): string {
  return window.innerHeight >= window.innerWidth ? 'Portrait' : 'Landscape';
}

function getReduceMotion(): string {
  return String(window.matchMedia('(prefers-reduced-motion: reduce)').matches);
}

function getReduceTransparency(): string {
  return String(window.matchMedia('(prefers-reduced-transparency: reduce)').matches);
}

function getInvertColors(): string {
  return String(window.matchMedia('(inverted-colors: inverted)').matches);
}

function getLayoutDirection(): string {
  const dir = document.documentElement.dir || document.body.dir;
  return dir === 'rtl' ? 'rightToLeft' : 'leftToRight';
}

function PageLoadSignal() {
  const hasFired = useRef(false);

  useEffect(() => {
    if (hasFired.current) return;
    hasFired.current = true;

    const appVersion = Constants.expoConfig?.version ?? 'unknown';
    const platform = getAppPlatform();
    const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone ?? 'unknown';
    const locale = navigator.language ?? 'unknown';
    const language = locale.split('-')[0] ?? 'unknown';
    const region = locale.split('-')[1] ?? 'unknown';

    const defaults: Record<string, string> = {
      // App info
      'TelemetryDeck.AppInfo.version': appVersion,

      // Device
      'TelemetryDeck.Device.platform': platform,
      'TelemetryDeck.Device.operatingSystem': platform === 'desktop' ? 'macOS' : 'web',
      'TelemetryDeck.Device.modelName': platform === 'desktop' ? 'desktop' : 'browser',
      'TelemetryDeck.Device.screenResolutionWidth': String(window.screen.width),
      'TelemetryDeck.Device.screenResolutionHeight': String(window.screen.height),
      'TelemetryDeck.Device.screenScaleFactor': String(window.devicePixelRatio ?? 1),
      'TelemetryDeck.Device.orientation': getOrientation(),
      'TelemetryDeck.Device.timeZone': timeZone,

      // Run context
      'TelemetryDeck.RunContext.isDebug': String(__DEV__),
      'TelemetryDeck.RunContext.locale': locale,
      'TelemetryDeck.RunContext.language': language,

      // User preferences
      'TelemetryDeck.UserPreference.colorScheme': getColorScheme(),
      'TelemetryDeck.UserPreference.language': language,
      'TelemetryDeck.UserPreference.layoutDirection': getLayoutDirection(),
      'TelemetryDeck.UserPreference.region': region,

      // Accessibility
      'TelemetryDeck.Accessibility.isReduceMotionEnabled': getReduceMotion(),
      'TelemetryDeck.Accessibility.isReduceTransparencyEnabled': getReduceTransparency(),
      'TelemetryDeck.Accessibility.isInvertColorsEnabled': getInvertColors(),

      // SDK
      'TelemetryDeck.SDK.name': 'JavaScriptSDK',
    };

    setDefaultPayload(defaults);

    trackSignal('app.launched', { startType: platform === 'desktop' ? 'desktop' : 'web' });
  }, []);

  return null;
}

export function AnalyticsProvider({ children }: { children: ReactNode }) {
  return (
    <TelemetryDeckProvider telemetryDeck={td}>
      <PageLoadSignal />
      {children}
    </TelemetryDeckProvider>
  );
}
