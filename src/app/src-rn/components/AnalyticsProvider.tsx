import { ReactNode, useEffect, useRef } from 'react';
import { AppState, Appearance, Platform } from 'react-native';
import { TelemetryDeckProvider } from '@typedigital/telemetrydeck-react';
import Constants from 'expo-constants';
import * as Device from 'expo-device';
import { td, trackSignal } from '../utils/analytics';

function LifecycleSignals() {
  const hasLaunched = useRef(false);
  const appState = useRef(AppState.currentState);

  useEffect(() => {
    if (!hasLaunched.current) {
      hasLaunched.current = true;

      const osName = Platform.OS === 'ios' ? 'iOS' : 'Android';
      const osVersion =
        Platform.OS === 'ios'
          ? String(Platform.Version)
          : String(
              (Platform.constants as Record<string, unknown>).Release ??
                Platform.Version,
            );

      trackSignal('app.launched', {
        startType: 'cold',
        appVersion: Constants.expoConfig?.version ?? 'unknown',
        os: osName,
        osVersion,
        deviceModel: Device.modelName ?? 'unknown',
        appearance: Appearance.getColorScheme() ?? 'unknown',
      });
    }

    const subscription = AppState.addEventListener('change', (nextState) => {
      if (appState.current.match(/inactive|background/) && nextState === 'active') {
        trackSignal('app.foregrounded', {
          appVersion: Constants.expoConfig?.version ?? 'unknown',
        });
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
