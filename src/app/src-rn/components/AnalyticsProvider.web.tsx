import { ReactNode, useEffect, useRef } from 'react';
import { TelemetryDeckProvider } from '@typedigital/telemetrydeck-react';
import Constants from 'expo-constants';
import { td, trackSignal } from '../utils/analytics';

function PageLoadSignal() {
  const hasFired = useRef(false);

  useEffect(() => {
    if (hasFired.current) return;
    hasFired.current = true;

    const appearance = window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';

    trackSignal('app.launched', {
      startType: 'web',
      appVersion: Constants.expoConfig?.version ?? 'unknown',
      os: 'web',
      osVersion: navigator.userAgent,
      deviceModel: 'browser',
      appearance,
    });
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
