import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useSystemTheme() {
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    let currentMode = 'system';

    const updateTheme = (isDark: boolean) => {
      if (isDark) {
        document.documentElement.classList.add('dark');
        document.documentElement.style.colorScheme = 'dark';
      } else {
        document.documentElement.classList.remove('dark');
        document.documentElement.style.colorScheme = 'light';
      }
    };

    const applyTheme = (mode: string) => {
      currentMode = mode;
      if (mode === 'dark') {
        updateTheme(true);
      } else if (mode === 'light') {
        updateTheme(false);
      } else {
        updateTheme(mediaQuery.matches);
      }
    };

    // Initial load from settings
    void invoke<Record<string, string>>('app_settings_get').then((settings) => {
      applyTheme(settings.theme_mode ?? 'system');
    });

    // Listen for manual changes from settings UI
    const unlistenPromise = listen<{ mode: string }>('theme:changed', (event) => {
      applyTheme(event.payload.mode);
    });

    // Listen for system theme changes (only matters if mode is 'system')
    const handleChange = (e: MediaQueryListEvent) => {
      if (currentMode === 'system') {
        updateTheme(e.matches);
      }
    };

    mediaQuery.addEventListener('change', handleChange);

    return () => {
      mediaQuery.removeEventListener('change', handleChange);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);
}