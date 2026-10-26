import { useEffect, useState } from 'react';

export type Appearance = 'system' | 'light' | 'dark';
export const appearanceKey = 'reclaim.appearance';

export function isAppearance(value: unknown): value is Appearance {
  return value === 'system' || value === 'light' || value === 'dark';
}

export function readAppearance(): Appearance {
  try {
    const preference = localStorage.getItem(appearanceKey);
    return isAppearance(preference) ? preference : 'system';
  } catch {
    return 'system';
  }
}

export interface AppearanceState {
  preference: Appearance;
  persistenceError: boolean;
  changeAppearance: (preference: Appearance) => void;
}

export function useAppearance(): AppearanceState {
  const [preference, setPreference] = useState<Appearance>(readAppearance);
  const [persistenceError, setPersistenceError] = useState(false);

  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)');
    function applyAppearance(): void {
      document.documentElement.dataset.appearance = preference;
      document.documentElement.dataset.theme =
        preference === 'system'
          ? query.matches
            ? 'dark'
            : 'light'
          : preference;
    }
    function receivePreference(event: StorageEvent): void {
      if (event.key === appearanceKey || event.key === null) {
        setPreference(readAppearance());
        setPersistenceError(false);
      }
    }
    applyAppearance();
    query.addEventListener('change', applyAppearance);
    window.addEventListener('storage', receivePreference);
    return () => {
      query.removeEventListener('change', applyAppearance);
      window.removeEventListener('storage', receivePreference);
    };
  }, [preference]);

  function changeAppearance(nextPreference: Appearance): void {
    setPreference(nextPreference);
    try {
      localStorage.setItem(appearanceKey, nextPreference);
      setPersistenceError(false);
    } catch {
      setPersistenceError(true);
    }
  }

  return { preference, persistenceError, changeAppearance };
}
