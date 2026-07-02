jest.mock('../../utils/secureStorage', () => ({
  getItem: jest.fn(),
  setItem: jest.fn(),
  deleteItem: jest.fn(),
}));

jest.mock('../../api/gourmetApi');

const secureStorage = require('../../utils/secureStorage');
const { GourmetApi } = require('../../api/gourmetApi');
const { useAuthStore } = require('../../store/authStore');

const mockLogin = jest.fn();
const mockLogout = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();

  GourmetApi.mockImplementation(() => ({
    login: mockLogin,
    logout: mockLogout,
    getMenus: jest.fn(),
    getOrders: jest.fn(),
    addToCart: jest.fn(),
    confirmOrders: jest.fn(),
    cancelOrders: jest.fn(),
    getBillings: jest.fn(),
    getUserInfo: jest.fn(),
    isAuthenticated: jest.fn(),
  }));

  useAuthStore.setState({
    status: 'idle',
    error: null,
    userInfo: null,
    api: new GourmetApi(),
  });
});

describe('useAuthStore', () => {
  describe('initial state', () => {
    it('has idle status with no error and no userInfo', () => {
      const { status, error, userInfo } = useAuthStore.getState();
      expect(status).toBe('idle');
      expect(error).toBeNull();
      expect(userInfo).toBeNull();
    });
  });

  describe('login', () => {
    const mockUserInfo = {
      username: 'testuser',
      shopModelId: 'shop1',
      eaterId: 'eater1',
      staffGroupId: 'staff1',
    };

    it('sets status to loading then authenticated on success', async () => {
      mockLogin.mockResolvedValue(mockUserInfo);

      const statusLog: string[] = [];
      const unsub = useAuthStore.subscribe((state: { status: string }) => {
        statusLog.push(state.status);
      });

      await useAuthStore.getState().login('user', 'pass');
      unsub();

      expect(statusLog).toContain('loading');
      expect(useAuthStore.getState().status).toBe('authenticated');
      expect(useAuthStore.getState().userInfo).toEqual(mockUserInfo);
      expect(useAuthStore.getState().error).toBeNull();
    });

    it('sets status to error on failure', async () => {
      mockLogin.mockRejectedValue(new Error('Invalid credentials'));

      await useAuthStore.getState().login('user', 'wrong');

      expect(useAuthStore.getState().status).toBe('error');
      expect(useAuthStore.getState().error).toBe('Invalid credentials');
      expect(useAuthStore.getState().userInfo).toBeNull();
    });

    it('returns true on success and false on failure', async () => {
      mockLogin.mockResolvedValue(mockUserInfo);
      const successResult = await useAuthStore.getState().login('user', 'pass');
      expect(successResult).toBe(true);

      mockLogin.mockRejectedValue(new Error('fail'));
      useAuthStore.setState({ status: 'idle', error: null, userInfo: null });
      const failResult = await useAuthStore.getState().login('user', 'wrong');
      expect(failResult).toBe(false);
    });

    it('uses generic message for non-Error exceptions', async () => {
      mockLogin.mockRejectedValue('string error');

      await useAuthStore.getState().login('user', 'pass');

      expect(useAuthStore.getState().error).toBe('Login failed');
    });
  });

  describe('loginWithSaved', () => {
    it('calls getSavedCredentials then login', async () => {
      const mockUserInfo = {
        username: 'saved_user',
        shopModelId: 's1',
        eaterId: 'e1',
        staffGroupId: 'g1',
      };
      secureStorage.getItem
        .mockResolvedValueOnce('saved_user')
        .mockResolvedValueOnce('saved_pass');
      mockLogin.mockResolvedValue(mockUserInfo);

      const result = await useAuthStore.getState().loginWithSaved();

      expect(secureStorage.getItem).toHaveBeenCalledWith('gourmet_username');
      expect(secureStorage.getItem).toHaveBeenCalledWith('gourmet_password');
      expect(mockLogin).toHaveBeenCalledWith('saved_user', 'saved_pass');
      expect(result).toBe(true);
      expect(useAuthStore.getState().status).toBe('authenticated');
    });

    it('sets no_credentials when no saved creds exist', async () => {
      secureStorage.getItem.mockResolvedValue(null);

      const result = await useAuthStore.getState().loginWithSaved();

      expect(result).toBe(false);
      expect(useAuthStore.getState().status).toBe('no_credentials');
      expect(mockLogin).not.toHaveBeenCalled();
    });
  });

  describe('logout', () => {
    it('calls api.logout and resets state', async () => {
      mockLogout.mockResolvedValue(undefined);
      useAuthStore.setState({
        status: 'authenticated',
        userInfo: { username: 'u', shopModelId: 's', eaterId: 'e', staffGroupId: 'g' },
        error: null,
      });

      await useAuthStore.getState().logout();

      expect(mockLogout).toHaveBeenCalled();
      expect(useAuthStore.getState().status).toBe('idle');
      expect(useAuthStore.getState().userInfo).toBeNull();
      expect(useAuthStore.getState().error).toBeNull();
    });

    it('resets state even if api.logout throws', async () => {
      mockLogout.mockRejectedValue(new Error('network error'));
      useAuthStore.setState({
        status: 'authenticated',
        userInfo: { username: 'u', shopModelId: 's', eaterId: 'e', staffGroupId: 'g' },
      });

      await expect(useAuthStore.getState().logout()).rejects.toThrow('network error');

      expect(useAuthStore.getState().status).toBe('idle');
      expect(useAuthStore.getState().userInfo).toBeNull();
    });
  });

  describe('credential management', () => {
    it('saveCredentials delegates to secureStorage.setItem', async () => {
      await useAuthStore.getState().saveCredentials('myuser', 'mypass');

      expect(secureStorage.setItem).toHaveBeenCalledWith('gourmet_username', 'myuser');
      expect(secureStorage.setItem).toHaveBeenCalledWith('gourmet_password', 'mypass');
    });

    it('getSavedCredentials returns credentials from secureStorage', async () => {
      secureStorage.getItem
        .mockResolvedValueOnce('stored_user')
        .mockResolvedValueOnce('stored_pass');

      const creds = await useAuthStore.getState().getSavedCredentials();

      expect(creds).toEqual({ username: 'stored_user', password: 'stored_pass' });
    });

    it('getSavedCredentials returns null when no credentials stored', async () => {
      secureStorage.getItem.mockResolvedValue(null);

      const creds = await useAuthStore.getState().getSavedCredentials();

      expect(creds).toBeNull();
    });

    it('clearCredentials delegates to secureStorage.deleteItem', async () => {
      await useAuthStore.getState().clearCredentials();

      expect(secureStorage.deleteItem).toHaveBeenCalledWith('gourmet_username');
      expect(secureStorage.deleteItem).toHaveBeenCalledWith('gourmet_password');
    });
  });
});

describe('demo mode', () => {
  it('logs in with demo credentials without touching the real API', async () => {
    const ok = await useAuthStore.getState().login('demo', 'demo1234!');

    expect(ok).toBe(true);
    expect(useAuthStore.getState().status).toBe('authenticated');
    expect(useAuthStore.getState().userInfo).not.toBeNull();
    expect(mockLogin).not.toHaveBeenCalled();
  });
});
