import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { LongTermTodo } from '../types/todo';

export function useLongTermTodos() {
  const longTermTodos = ref<LongTermTodo[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Load all long term todos
  const loadLongTermTodos = async () => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('get_long_term_todos');
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to load long term todos:', e);
    } finally {
      loading.value = false;
    }
  };

  // Add a new long term todo
  const addLongTermTodo = async (content: string) => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('add_long_term_todo', { content });
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to add long term todo:', e);
      throw e;
    } finally {
      loading.value = false;
    }
  };

  // Toggle long term todo completion
  const toggleLongTermTodo = async (id: string) => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('toggle_long_term_todo', { id });
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to toggle long term todo:', e);
    } finally {
      loading.value = false;
    }
  };

  // Update long term todo content
  const updateLongTermTodo = async (id: string, content: string) => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('update_long_term_todo', { id, content });
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to update long term todo:', e);
      throw e;
    } finally {
      loading.value = false;
    }
  };

  // Delete a long term todo
  const deleteLongTermTodo = async (id: string) => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('delete_long_term_todo', { id });
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to delete long term todo:', e);
    } finally {
      loading.value = false;
    }
  };

  // Move long term todo up
  const moveLongTermTodoUp = async (id: string) => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('move_long_term_todo_up', { id });
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to move long term todo up:', e);
    } finally {
      loading.value = false;
    }
  };

  // Move long term todo down
  const moveLongTermTodoDown = async (id: string) => {
    loading.value = true;
    error.value = null;
    try {
      longTermTodos.value = await invoke<LongTermTodo[]>('move_long_term_todo_down', { id });
    } catch (e) {
      error.value = (e as Error).message;
      console.error('Failed to move long term todo down:', e);
    } finally {
      loading.value = false;
    }
  };

  return {
    longTermTodos,
    loading,
    error,
    loadLongTermTodos,
    addLongTermTodo,
    toggleLongTermTodo,
    updateLongTermTodo,
    deleteLongTermTodo,
    moveLongTermTodoUp,
    moveLongTermTodoDown,
  };
}
