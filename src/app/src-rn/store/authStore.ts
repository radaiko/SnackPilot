import { create } from 'zustand';
import * as secureStorage from '../utils/secureStorage';
import { GourmetApi } from '../api/gourmetApi';
import { DemoGourmetApi } from '../api/demoGourmetApi';
import { GourmetUserInfo } from '../types/menu';
import { isDemoCredentials, CREDENTIALS_KEY_USER, CREDENTIALS_KEY_PASS } from '../utils/constants';
import { useMenuStore } from './menuStore';
import { trackSignal } from '../utils/analytics';

type AuthStatus = 'idle' | 'loading' | 'authenticated' | 'error' | 'no_credentials';

interface AuthState {
  status: AuthStatus;
  error: string | null;
  userInfo: GourmetUserInfo | null;
  api: GourmetApi;

  login: (username: string, password: string) => Promise<boolean>;
  loginWithSaved: () => Promise<boolean>;
  logout: () => Promise<void>;
  saveCredentials: (username: string, password: string) => Promise<void>;
  getSavedCredentials: () => Promise<{ username: string; password: string } | null>;
  clearCredentials: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  status: 'idle',
  error: null,
  userInfo: null,
  api: new GourmetApi(),

  login: async (username: string, password: string) => {
    set({ status: 'loading', error: null });
    try {
      if (isDemoCredentials(username, password)) {
        const demoApi = new DemoGourmetApi();
        const userInfo = await demoApi.login(username, password);
        useMenuStore.setState({ items: [], lastFetched: null });
        set({ status: 'authenticated', userInfo, error: null, api: demoApi as unknown as GourmetApi });
        trackSignal('auth.loginSuccess', { service: 'gourmet' });
        return true;
      }
      const userInfo = await get().api.login(username, password);
      set({ status: 'authenticated', userInfo, error: null });
      trackSignal('auth.loginSuccess', { service: 'gourmet' });
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Login failed';
      set({ status: 'error', error: message, userInfo: null });
      trackSignal('auth.loginFailed', { service: 'gourmet' });
      return false;
    }
  },

  loginWithSaved: async () => {
    const creds = await get().getSavedCredentials();
    if (!creds) {
      set({ status: 'no_credentials', error: null });
      return false;
    }
    return get().login(creds.username, creds.password);
  },

  logout: async () => {
    trackSignal('auth.logout', { service: 'gourmet' });
    try {
      await get().api.logout();
    } finally {
      set({ status: 'idle', userInfo: null, error: null });
    }
  },

  saveCredentials: async (username: string, password: string) => {
    await secureStorage.setItem(CREDENTIALS_KEY_USER, username);
    await secureStorage.setItem(CREDENTIALS_KEY_PASS, password);
  },

  getSavedCredentials: async () => {
    const username = await secureStorage.getItem(CREDENTIALS_KEY_USER);
    const password = await secureStorage.getItem(CREDENTIALS_KEY_PASS);
    if (!username || !password) return null;
    return { username, password };
  },

  clearCredentials: async () => {
    await secureStorage.deleteItem(CREDENTIALS_KEY_USER);
    await secureStorage.deleteItem(CREDENTIALS_KEY_PASS);
  },
}));
