import * as Crypto from 'expo-crypto';

if (typeof globalThis.crypto?.subtle?.digest !== 'function') {
  (globalThis as any).crypto = {
    ...globalThis.crypto,
    subtle: {
      digest: (algorithm: string, data: ArrayBuffer) =>
        Crypto.digest(
          algorithm === 'SHA-256'
            ? Crypto.CryptoDigestAlgorithm.SHA256
            : algorithm,
          data,
        ),
    },
  };
}
