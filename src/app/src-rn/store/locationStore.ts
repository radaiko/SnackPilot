import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import AsyncStorage from '@react-native-async-storage/async-storage';

export interface CompanyLocation {
  latitude: number;
  longitude: number;
}

interface LocationState {
  companyLocation: CompanyLocation | null;
  isAtCompany: boolean;

  setCompanyLocation: (latitude: number, longitude: number) => void;
  clearCompanyLocation: () => void;
  setIsAtCompany: (value: boolean) => void;
  hasCompanyLocation: () => boolean;
}

export const useLocationStore = create<LocationState>()(
  persist(
    (set, get) => ({
      companyLocation: null,
      isAtCompany: false,

      setCompanyLocation: (latitude, longitude) =>
        set({ companyLocation: { latitude, longitude } }),

      clearCompanyLocation: () =>
        set({ companyLocation: null, isAtCompany: false }),

      setIsAtCompany: (value) =>
        set({ isAtCompany: value }),

      hasCompanyLocation: () =>
        get().companyLocation !== null,
    }),
    {
      name: 'company-location',
      storage: createJSONStorage(() => AsyncStorage),
    }
  )
);
