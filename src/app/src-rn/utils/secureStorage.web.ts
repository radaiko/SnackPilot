import { isDesktop } from './platform';

function getInvoke(): ((cmd: string, args?: unknown) => Promise<unknown>) | null {
  if (typeof window === 'undefined') return null;

  if ('__TAURI_INTERNALS__' in window) {
    const invoke = (window as any).__TAURI_INTERNALS__?.invoke;
    if (typeof invoke === 'function') return invoke;
  }

  const tauriInvoke = (window as any).__TAURI__?.core?.invoke;
  if (typeof tauriInvoke === 'function') return tauriInvoke;

  return null;
}

async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const invoke = getInvoke();
  if (!invoke) {
    throw new Error('Tauri secure storage is unavailable');
  }
  return invoke(cmd, args) as Promise<T>;
}

export async function getItem(key: string): Promise<string | null> {
  if (isDesktop()) {
    return tauriInvoke<string | null>('secure_get_item', { key });
  }
  return localStorage.getItem(key);
}

export async function setItem(key: string, value: string): Promise<void> {
  if (isDesktop()) {
    await tauriInvoke<void>('secure_set_item', { key, value });
    return;
  }
  localStorage.setItem(key, value);
}

export async function deleteItem(key: string): Promise<void> {
  if (isDesktop()) {
    await tauriInvoke<void>('secure_delete_item', { key });
    return;
  }
  localStorage.removeItem(key);
}

// No-op on web — keychain migration is iOS/Android only.
export async function migrateKeychainAccessibility(_keys: string[]): Promise<void> {}
