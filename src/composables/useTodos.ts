import { invoke } from '@tauri-apps/api/core';
import type { TodoItem } from '../types/todo';
import type { Ref } from 'vue';
import { ref, computed } from 'vue';

/**
 * Build a tree structure from a flat list of todos
 * Top-level todos will have their subtodos nested under them
 */
function buildTodoTree(flatTodos: TodoItem[]): TodoItem[] {
  // Create a map for quick lookup
  const todoMap = new Map<string, TodoItem>();
  flatTodos.forEach(todo => {
    todoMap.set(todo.id, { ...todo, subtodos: [] });
  });

  // Build the tree structure
  const rootTodos: TodoItem[] = [];
  flatTodos.forEach(todo => {
    const node = todoMap.get(todo.id)!;
    if (todo.parentId) {
      const parent = todoMap.get(todo.parentId);
      if (parent) {
        if (!parent.subtodos) {
          parent.subtodos = [];
        }
        parent.subtodos.push(node);
      } else {
        // Parent not found, treat as root todo (orphan)
        rootTodos.push(node);
      }
    } else {
      rootTodos.push(node);
    }
  });

  return rootTodos;
}

/**
 * Composable for managing todos
 * Provides reactive state and methods for CRUD operations
 */
export function useTodos() {
  const todos: Ref<TodoItem[]> = ref([]);
  const loading: Ref<boolean> = ref(false);
  const error: Ref<string | null> = ref(null);
  const currentViewDate: Ref<string> = ref('');

  /**
   * Load all todos from the backend and build tree structure
   */
  const loadTodos = async (targetDate?: string) => {
    // Update current view date
    if (targetDate !== undefined) {
      currentViewDate.value = targetDate;
    }

    loading.value = true;
    error.value = null;
    try {
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to load todos:', e);
    } finally {
      loading.value = false;
    }
  };

  /**
   * Add a new todo (can be a parent or subtodo)
   */
  const addTodo = async (
    content: string,
    repeatMode: TodoItem['repeatMode'],
    weekdays?: string,
    specificDates?: string,
    parentId?: string
  ) => {
    // Don't show loading state to avoid page jump (like toggleTodo)
    error.value = null;
    try {
      await invoke<TodoItem[]>('add_todo', {
        content,
        repeatMode,
        weekdays,
        specificDates,
        parentId,
      });
      // Reload with current view date instead of using returned list
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to add todo:', e);
      throw e;
    }
  };

  /**
   * Toggle the completion status of a todo
   */
  const toggleTodo = async (id: string) => {
    // Don't show loading state for toggle to avoid page jump
    error.value = null;
    try {
      await invoke<TodoItem[]>('toggle_todo', { id });
      // Reload with current view date instead of using returned list
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to toggle todo:', e);
    }
  };

  /**
   * Delete a todo (and all its subtodos if it's a parent)
   */
  const deleteTodo = async (id: string) => {
    // Don't show loading state for delete to avoid page jump
    error.value = null;
    try {
      await invoke<TodoItem[]>('delete_todo', { id });
      // Reload with current view date instead of using returned list
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to delete todo:', e);
    }
  };

  /**
   * Toggle expanded state of a parent todo (local only, no backend call)
   */
  const toggleExpand = async (id: string) => {
    // Find the todo in the tree and toggle its expanded state
    const findAndToggle = (todoList: TodoItem[]): boolean => {
      for (const todo of todoList) {
        if (todo.id === id) {
          todo.expanded = !todo.expanded;
          return true;
        }
        if (todo.subtodos && findAndToggle(todo.subtodos)) {
          return true;
        }
      }
      return false;
    };
    findAndToggle(todos.value);
    // Trigger reactivity
    todos.value = [...todos.value];
  };

  /**
   * Complete all subtodos of a parent todo (with user confirmation)
   */
  const completeAllSubtodos = async (id: string) => {
    loading.value = true;
    error.value = null;
    try {
      await invoke<TodoItem[]>('complete_all_subtodos', { id });
      // Reload with current view date instead of using returned list
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to complete all subtodos:', e);
    } finally {
      loading.value = false;
    }
  };

  /**
   * Complete a weekly todo early (mark as completed without affecting reset logic)
   */
  const completeEarly = async (id: string) => {
    // Don't show loading state to avoid page jump
    error.value = null;
    try {
      await invoke<TodoItem[]>('complete_early', { id });
      // Reload with current view date instead of using returned list
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to complete early:', e);
    }
  };

  /**
   * Toggle disable status of a todo
   * When disabled, the todo is marked as completed and cannot be operated on
   */
  const toggleDisable = async (id: string) => {
    // Don't show loading state to avoid page jump
    error.value = null;
    try {
      await invoke<TodoItem[]>('toggle_disable', { id });
      // Reload with current view date instead of using returned list
      const flatTodos = await invoke<TodoItem[]>('get_todos', {
        targetDate: currentViewDate.value || null
      });
      todos.value = buildTodoTree(flatTodos);
    } catch (e) {
      error.value = e as string;
      console.error('Failed to toggle disable:', e);
    }
  };

  /**
   * Get incomplete todos (top-level only)
   */
  const incompleteTodos = computed(() =>
    todos.value.filter((t) => !t.completed)
  );

  /**
   * Get completed todos (top-level only)
   */
  const completedTodos = computed(() =>
    todos.value.filter((t) => t.completed)
  );

  /**
   * Count all todos recursively.
   * - If a parent todo has no subtodos, count it.
   * - If a parent todo has subtodos, don't count the parent, only count subtodos.
   */
  const countAllTodos = (todoList: TodoItem[]): { total: number; completed: number } => {
    let total = 0;
    let completed = 0;

    for (const todo of todoList) {
      // If this todo has subtodos, only count subtodos (not the parent)
      if (todo.subtodos && todo.subtodos.length > 0) {
        const subCounts = countAllTodos(todo.subtodos);
        total += subCounts.total;
        completed += subCounts.completed;
      } else {
        // No subtodos, count this todo
        total++;
        if (todo.completed) {
          completed++;
        }
      }
    }

    return { total, completed };
  };

  /**
   * Progress percentage (0-100)
   */
  const progress = computed(() => {
    const { total, completed } = countAllTodos(todos.value);
    if (total === 0) return 0;
    return Math.round((completed / total) * 100);
  });

  /**
   * Progress text (e.g., "3/5 已完成")
   */
  const progressText = computed(() => {
    const { total, completed } = countAllTodos(todos.value);
    return `${completed}/${total} 已完成`;
  });

  return {
    todos,
    loading,
    error,
    loadTodos,
    setViewDate: (date: string) => {
      currentViewDate.value = date;
    },
    addTodo,
    toggleTodo,
    deleteTodo,
    toggleExpand,
    completeAllSubtodos,
    completeEarly,
    toggleDisable,
    incompleteTodos,
    completedTodos,
    progress,
    progressText,
  };
}
