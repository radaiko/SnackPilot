import * as Crypto from 'expo-crypto';

/**
 * Polyfill for crypto.subtle.digest using expo-crypto.
 *
 * Uses digestStringAsync (fully async native path) instead of digest()
 * because the sync native TurboModule path crashes in release builds
 * when receiving ArrayBuffer data through the bridge.
 */
if (typeof globalThis.crypto?.subtle?.digest !== 'function') {
  (globalThis as any).crypto = {
    ...globalThis.crypto,
    subtle: {
      digest: async (algorithm: string, data: ArrayBuffer): Promise<ArrayBuffer> => {
        const algo =
          algorithm === 'SHA-256'
            ? Crypto.CryptoDigestAlgorithm.SHA256
            : (algorithm as Crypto.CryptoDigestAlgorithm);

        // Decode ArrayBuffer back to string for digestStringAsync
        const str = new TextDecoder().decode(data);
        const hex = await Crypto.digestStringAsync(algo, str, {
          encoding: Crypto.CryptoEncoding.HEX,
        });

        // Convert hex string back to ArrayBuffer (matching Web Crypto API return type)
        const bytes = new Uint8Array(hex.length / 2);
        for (let i = 0; i < hex.length; i += 2) {
          bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
        }
        return bytes.buffer;
      },
    },
  };
}
