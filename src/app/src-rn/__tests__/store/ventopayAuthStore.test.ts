jest.mock('../../utils/secureStorage', () => ({
  getItem: jest.fn(),
  setItem: jest.fn(),
  deleteItem: jest.fn(),
}));

jest.mock('../../api/ventopayApi');

const secureStorage = require('../../utils/secureStorage');
const { VentopayApi } = require('../../api/ventopayApi');
const { useVentopayAuthStore } = require('../../store/ventopayAuthStore');

const mockLogin = jest.fn();
const mockLogout = jest.fn();

beforeEach(() => {
  jest.clearAllMocks();

  VentopayApi.mockImplementation(() => ({
    login: mockLogin,
    logout: mockLogout,
    getTransactions: jest.fn(),
    isAuthenticated: jest.fn(),
  }));

  useVentopayAuthStore.setState({
    status: 'idle',
    error: null,
    api: new VentopayApi(),
  });
});

describe('useVentopayAuthStore', () => {
  describe('initial state', () => {
    it('has idle status with no error', () => {
      const { status, error } = useVentopayAuthStore.getState();
      expect(status).toBe('idle');
      expect(error).toBeNull();
    });
  });

  describe('login', () => {
    it('sets status to authenticated on success', async () => {
      mockLogin.mockResolvedValue(undefined);

      await useVentopayAuthStore.getState().login('user', 'pass');

      expect(useVentopayAuthStore.getState().status).toBe('authenticated');
      expect(useVentopayAuthStore.getState().error).toBeNull();
    });

    it('sets status to error on failure', async () => {
      mockLogin.mockRejectedValue(new Error('Ventopay login failed'));

      await useVentopayAuthStore.getState().login('user', 'wrong');

      expect(useVentopayAuthStore.getState().status).toBe('error');
      expect(useVentopayAuthStore.getState().error).toBe('Ventopay login failed');
    });

    it('returns true on success and false on failure', async () => {
      mockLogin.mockResolvedValue(undefined);
      const successResult = await useVentopayAuthStore.getState().login('user', 'pass');
      expect(successResult).toBe(true);

      mockLogin.mockRejectedValue(new Error('fail'));
      useVentopayAuthStore.setState({ status: 'idle', error: null });
      const failResult = await useVentopayAuthStore.getState().login('user', 'wrong');
      expect(failResult).toBe(false);
    });

    it('uses generic message for non-Error exceptions', async () => {
      mockLogin.mockRejectedValue('string error');

      await useVentopayAuthStore.getState().login('user', 'pass');

      expect(useVentopayAuthStore.getState().error).toBe('Ventopay login failed');
    });
  });

  describe('loginWithSaved', () => {
    it('sets no_credentials when no saved creds exist', async () => {
      secureStorage.getItem.mockResolvedValue(null);

      const result = await useVentopayAuthStore.getState().loginWithSaved();

      expect(result).toBe(false);
      expect(useVentopayAuthStore.getState().status).toBe('no_credentials');
      expect(mockLogin).not.toHaveBeenCalled();
    });

    it('calls login with saved credentials when they exist', async () => {
      secureStorage.getItem
        .mockResolvedValueOnce('vento_user')
        .mockResolvedValueOnce('vento_pass');
      mockLogin.mockResolvedValue(undefined);

      const result = await useVentopayAuthStore.getState().loginWithSaved();

      expect(secureStorage.getItem).toHaveBeenCalledWith('ventopay_username');
      expect(secureStorage.getItem).toHaveBeenCalledWith('ventopay_password');
      expect(mockLogin).toHaveBeenCalledWith('vento_user', 'vento_pass');
      expect(result).toBe(true);
    });
  });

  describe('logout', () => {
    it('calls api.logout and resets state', async () => {
      mockLogout.mockResolvedValue(undefined);
      useVentopayAuthStore.setState({ status: 'authenticated', error: null });

      await useVentopayAuthStore.getState().logout();

      expect(mockLogout).toHaveBeenCalled();
      expect(useVentopayAuthStore.getState().status).toBe('idle');
      expect(useVentopayAuthStore.getState().error).toBeNull();
    });

    it('resets state even if api.logout throws', async () => {
      mockLogout.mockRejectedValue(new Error('network error'));
      useVentopayAuthStore.setState({ status: 'authenticated' });

      await expect(useVentopayAuthStore.getState().logout()).rejects.toThrow('network error');

      expect(useVentopayAuthStore.getState().status).toBe('idle');
    });
  });

  describe('credential management', () => {
    it('saveCredentials delegates to secureStorage.setItem', async () => {
      await useVentopayAuthStore.getState().saveCredentials('vuser', 'vpass');

      expect(secureStorage.setItem).toHaveBeenCalledWith('ventopay_username', 'vuser');
      expect(secureStorage.setItem).toHaveBeenCalledWith('ventopay_password', 'vpass');
    });

    it('clearCredentials delegates to secureStorage.deleteItem', async () => {
      await useVentopayAuthStore.getState().clearCredentials();

      expect(secureStorage.deleteItem).toHaveBeenCalledWith('ventopay_username');
      expect(secureStorage.deleteItem).toHaveBeenCalledWith('ventopay_password');
    });
  });
});

describe('demo mode', () => {
  it('logs in with demo credentials without touching the real API', async () => {
    const ok = await useVentopayAuthStore.getState().login('demo', 'demo1234!');

    expect(ok).toBe(true);
    expect(useVentopayAuthStore.getState().status).toBe('authenticated');
    expect(mockLogin).not.toHaveBeenCalled();
  });
});
