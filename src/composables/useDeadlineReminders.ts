import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { DeadlineReminder } from '../types/todo';

export function useDeadlineReminders() {
  const reminders = ref<DeadlineReminder[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const loadReminders = async () => {
    loading.value = true;
    error.value = null;
    try {
      reminders.value = await invoke<DeadlineReminder[]>('get_deadline_reminders');
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      console.error('Failed to load deadline reminders:', e);
    } finally {
      loading.value = false;
    }
  };

  const addReminder = async (title: string, dueDate: string) => {
    error.value = null;
    reminders.value = await invoke<DeadlineReminder[]>('add_deadline_reminder', {
      title,
      dueDate,
    });
  };

  const updateReminder = async (id: string, title: string, dueDate: string) => {
    error.value = null;
    reminders.value = await invoke<DeadlineReminder[]>('update_deadline_reminder', {
      id,
      title,
      dueDate,
    });
  };

  const deleteReminder = async (id: string) => {
    error.value = null;
    reminders.value = await invoke<DeadlineReminder[]>('delete_deadline_reminder', { id });
  };

  return {
    reminders,
    loading,
    error,
    loadReminders,
    addReminder,
    updateReminder,
    deleteReminder,
  };
}
