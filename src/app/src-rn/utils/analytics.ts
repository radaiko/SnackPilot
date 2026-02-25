import './cryptoPolyfill';
import { createTelemetryDeck } from '@typedigital/telemetrydeck-react';

const TELEMETRYDECK_APP_ID = '75C7346F-3BBB-4BE8-8791-7ADF25EC3DBA';

export const td = createTelemetryDeck({
  appID: TELEMETRYDECK_APP_ID,
  clientUser: 'anonymous',
});

/** Fire-and-forget signal. Silently catches errors so analytics never crashes the app. */
export function trackSignal(type: string, payload?: Record<string, string>) {
  try {
    td.signal(type, payload).catch(() => {});
  } catch {
    // Analytics must never crash the app
  }
}
