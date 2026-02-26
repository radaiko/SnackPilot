import './cryptoPolyfill';
import { createTelemetryDeck } from '@typedigital/telemetrydeck-react';

const TELEMETRYDECK_APP_ID = 'BA25F62D-0154-4A92-BF85-29FC5FDDA3EC';

export const td = createTelemetryDeck({
  appID: TELEMETRYDECK_APP_ID,
  clientUser: 'anonymous',
  testMode: false,
});

/** Default payload merged into every signal. Set once at app launch by AnalyticsProvider. */
let defaultPayload: Record<string, string> = {};

/** Called by AnalyticsProvider to set platform-specific default parameters. */
export function setDefaultPayload(params: Record<string, string>) {
  defaultPayload = params;
}

/** Fire-and-forget signal. Silently catches errors so analytics never crashes the app. */
export function trackSignal(type: string, payload?: Record<string, string>) {
  try {
    td.signal(type, {
      ...defaultPayload,
      ...payload,
    }).catch(() => {});
  } catch {
    // Analytics must never crash the app
  }
}
