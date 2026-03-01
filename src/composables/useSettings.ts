import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Settings } from '../types/settings';

export function useSettings() {
  const settings = ref<Settings | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const loadSettings = async () => {
    loading.value = true;
    try {
      settings.value = await invoke<Settings>('get_settings');
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  };

  const updateShortcut = async (action: string, shortcut: string) => {
    try {
      settings.value = await invoke<Settings>('update_shortcut', {
        action,
        shortcut,
      });
    } catch (e) {
      throw new Error(String(e));
    }
  };

  return { settings, loading, error, loadSettings, updateShortcut };
}
