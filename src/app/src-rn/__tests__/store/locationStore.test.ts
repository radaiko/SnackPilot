jest.mock('@react-native-async-storage/async-storage', () => {
  const store: Record<string, string> = {};
  return {
    __esModule: true,
    default: {
      getItem: jest.fn((key: string) => Promise.resolve(store[key] ?? null)),
      setItem: jest.fn((key: string, value: string) => { store[key] = value; return Promise.resolve(); }),
      removeItem: jest.fn((key: string) => { delete store[key]; return Promise.resolve(); }),
      clear: jest.fn(() => { Object.keys(store).forEach(k => delete store[k]); return Promise.resolve(); }),
    },
  };
});

import { useLocationStore } from '../../store/locationStore';

beforeEach(async () => {
  const AsyncStorage = require('@react-native-async-storage/async-storage').default;
  await AsyncStorage.clear();
  jest.clearAllMocks();
  useLocationStore.setState({
    companyLocation: null,
    isAtCompany: false,
  });
});

describe('locationStore', () => {
  it('has no company location by default', () => {
    const { companyLocation } = useLocationStore.getState();
    expect(companyLocation).toBeNull();
  });

  it('has isAtCompany false by default', () => {
    expect(useLocationStore.getState().isAtCompany).toBe(false);
  });

  it('setCompanyLocation saves lat/lng', () => {
    useLocationStore.getState().setCompanyLocation(48.2082, 16.3738);
    const { companyLocation } = useLocationStore.getState();
    expect(companyLocation).toEqual({ latitude: 48.2082, longitude: 16.3738 });
  });

  it('clearCompanyLocation resets location and isAtCompany', () => {
    useLocationStore.getState().setCompanyLocation(48.2082, 16.3738);
    useLocationStore.getState().setIsAtCompany(true);
    useLocationStore.getState().clearCompanyLocation();
    expect(useLocationStore.getState().companyLocation).toBeNull();
    expect(useLocationStore.getState().isAtCompany).toBe(false);
  });

  it('setIsAtCompany updates the flag', () => {
    useLocationStore.getState().setIsAtCompany(true);
    expect(useLocationStore.getState().isAtCompany).toBe(true);
    useLocationStore.getState().setIsAtCompany(false);
    expect(useLocationStore.getState().isAtCompany).toBe(false);
  });

  it('hasCompanyLocation returns true when location is set', () => {
    expect(useLocationStore.getState().hasCompanyLocation()).toBe(false);
    useLocationStore.getState().setCompanyLocation(48.2082, 16.3738);
    expect(useLocationStore.getState().hasCompanyLocation()).toBe(true);
  });
});
