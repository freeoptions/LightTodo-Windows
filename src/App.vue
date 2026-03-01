<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick } from 'vue';
import TodoList from './components/TodoList.vue';
import Settings from './components/Settings.vue';
import DatePicker from './components/DatePicker.vue';
import { useTodos } from './composables/useTodos';
import { type RepeatMode, type TodoItem as TodoItemType } from './types/todo';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { saveWindowState as saveWindowStatePlugin } from '@tauri-apps/plugin-window-state';
import type { Settings as SettingsType } from './types/settings';
// @ts-ignore - CommonJS module
import { Solar } from 'lunar-javascript';

const {
  todos,
  loading,
  error,
  loadTodos,
  setViewDate,
  addTodo,
  toggleTodo,
  deleteTodo,
  toggleExpand,
  completeAllSubtodos,
  completeEarly,
  toggleDisable,
  progress,
  progressText
} = useTodos();

// Modal state
const showModal = ref(false);
const editingTodoId = ref<string | null>(null);
const newTodoContent = ref('');
const newTodoRepeatMode = ref<RepeatMode>('daily'); // 默认每天重复
const newTodoWeekdays = ref<number[]>([]); // 选中的周几 (1-7, 周一=1, 周日=7)
const newTodoSpecificDates = ref<string[]>([]); // 选中的指定日期
const weekdayOptions = [
  { value: 1, label: '周一' },
  { value: 2, label: '周二' },
  { value: 3, label: '周三' },
  { value: 4, label: '周四' },
  { value: 5, label: '周五' },
  { value: 6, label: '周六' },
  { value: 7, label: '周日' },
];

// Delete confirmation state
const showDeleteConfirm = ref(false);
const deletingTodoId = ref<string | null>(null);
const deletingTodoContent = ref('');
const deletingTodoRepeatMode = ref<RepeatMode>('daily');

// Subtodo state
const addingSubtodoForParentId = ref<string | null>(null);
const editingSubtodo = ref(false); // true if editing a subtodo (not parent)

// Subtodos in the modal (for creating/editing parent with subtodos)
const modalSubtodos = ref<Array<{ id: string; content: string }>>([]);
const newSubtodoContent = ref('');

// Parent todo completion confirmation state
const showParentCompleteConfirm = ref(false);
const parentCompleteId = ref<string | null>(null);
const parentCompleteContent = ref('');

// Complete early confirmation state
const showCompleteEarlyConfirm = ref(false);
const completeEarlyId = ref<string | null>(null);
const completeEarlyContent = ref('');

// Settings state
const showSettings = ref(false);
const memo = ref('');
const memoHeight = ref(80);
const isSavingMemo = ref(false);

// View date state (for viewing todos on different days)
const viewDate = ref('');
const showDatePicker = ref(false);

const currentWindow = getCurrentWindow();

// Format current date
const currentDate = computed(() => {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  const weekdays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
  const weekday = weekdays[now.getDay()];

  // Lunar date
  const solar = Solar.fromDate(now);
  const lunar = solar.getLunar();
  const lunarZodiac = lunar.getYearShengXiao() + '年';
  const lunarMonthText = lunar.getMonthInChinese() + '月' + lunar.getDayInChinese();

  return {
    date: `${year}-${month}-${day}`,
    weekday: weekday,
    isoDate: `${year}-${month}-${day}`,
    lunarZodiac: lunarZodiac,
    lunarMonthText: lunarMonthText
  };
});

// Format view date
const viewDateDisplay = computed(() => {
  if (!viewDate.value) {
    return currentDate.value;
  }

  const date = new Date(viewDate.value + 'T00:00:00');
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const weekdays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
  const weekday = weekdays[date.getDay()];

  // Lunar date
  const solar = Solar.fromDate(date);
  const lunar = solar.getLunar();
  const lunarZodiac = lunar.getYearShengXiao() + '年';
  const lunarMonthText = lunar.getMonthInChinese() + '月' + lunar.getDayInChinese();

  return {
    date: `${year}-${month}-${day}`,
    weekday: weekday,
    isoDate: viewDate.value,
    lunarMonthText: lunarMonthText,
    lunarZodiac: lunarZodiac
  };
});

// Check if current view is today
const isViewingToday = computed(() => {
  return viewDate.value === '' || viewDate.value === currentDate.value.isoDate;
});

// Minimum date for date picker (today)
const minDate = computed(() => {
  return new Date().toISOString().split('T')[0];
});

// Generate week dates for the picker (starting from Monday of current week)
const weekDates = computed(() => {
  const now = new Date();
  const currentDay = now.getDay(); // 0 = Sunday, 1 = Monday, ...
  const mondayOffset = currentDay === 0 ? -6 : 1 - currentDay; // Calculate offset to Monday

  const dates = [];
  for (let i = 0; i < 7; i++) {
    const date = new Date(now);
    date.setDate(now.getDate() + mondayOffset + i);
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const isoDate = `${year}-${month}-${day}`;
    const weekdays = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
    const weekday = weekdays[date.getDay()];

    // Check if this date is in the past (before today)
    const todayDate = new Date(currentDate.value.isoDate + 'T00:00:00');
    todayDate.setHours(0, 0, 0, 0);
    const checkDate = new Date(isoDate + 'T00:00:00');
    checkDate.setHours(0, 0, 0, 0);
    const isPast = checkDate < todayDate;

    dates.push({
      isoDate,
      weekday,
      day,
      isPast
    });
  }
  return dates;
});

// Handle shortcuts changed from settings (shortcuts are now registered in Rust backend)
const handleShortcutsChanged = async (shortcuts: { showHideWindow: string; toggleFloating: string }) => {
  console.log('Shortcuts changed, updating in Rust backend:', shortcuts);
  try {
    // Call Rust backend to update shortcuts
    await invoke('update_shortcut', {
      action: 'showHideWindow',
      shortcut: shortcuts.showHideWindow
    });
  } catch (e) {
    console.error('Failed to update shortcuts:', e);
  }
};

// Debounced save window state function
let saveStateTimer: ReturnType<typeof setTimeout> | null = null;
const debouncedSaveWindowState = () => {
  if (saveStateTimer) clearTimeout(saveStateTimer);
  saveStateTimer = setTimeout(() => {
    console.log('Saving window state via plugin...');
    saveWindowStatePlugin();
  }, 300); // 300ms debounce
};

onMounted(async () => {
  loadTodos();
  // Set initial view date (today)
  setViewDate('');

  // Load memo (shortcuts are now registered in Rust backend)
  try {
    const settings = await invoke<SettingsType>('get_settings');
    console.log('Frontend received settings:', settings);
    memo.value = settings.memo || '';
    memoHeight.value = settings.memoHeight || 80;
  } catch (e) {
    console.error('Failed to load settings:', e);
  }

  // Listen for window events to save state
  const unlistenResize = await currentWindow.onResized(() => {
    console.log('Window resized, saving state...');
    debouncedSaveWindowState();
  });

  const unlistenMove = await currentWindow.onMoved(() => {
    console.log('Window moved, saving state...');
    debouncedSaveWindowState();
  });

  onUnmounted(() => {
    unlistenResize();
    unlistenMove();
  });
});

const handleAdd = async (content: string, repeatMode: RepeatMode, weekdays?: string, specificDates?: string, parentId?: string) => {
  try {
    await addTodo(content, repeatMode, weekdays, specificDates, parentId);
  } catch (e) {
    console.error('Failed to add todo:', e);
  }
};

// Open modal for adding
const openAddModal = async () => {
  editingTodoId.value = null;
  addingSubtodoForParentId.value = null;
  editingSubtodo.value = false;
  newTodoContent.value = '';
  newTodoRepeatMode.value = 'daily';
  newTodoWeekdays.value = [];
  newTodoSpecificDates.value = [];
  modalSubtodos.value = [];
  newSubtodoContent.value = '';
  showModal.value = true;
  await nextTick();
};

// Open modal for editing
const openEditModal = async (id: string) => {
  console.log('=== [EDIT] openEditModal called with id:', id);
  console.log('=== [EDIT] Current todos:', todos.value.map(t => ({ id: t.id, content: t.content })));

  const todo = findTodoInTree(todos.value, id);
  console.log('=== [EDIT] findTodoInTree returned:', todo);

  if (todo) {
    console.log('=== [EDIT] Opening edit modal for:', todo.content);
    console.log('=== [EDIT] Has subtodos:', todo.subtodos?.length || 0);

    // Check if this is a subtodo (has parentId) or parent todo (no parentId)
    // parentId can be undefined, null, or a string - only string means it's a subtodo
    const isSubtodo = !!todo.parentId;
    editingSubtodo.value = isSubtodo;
    console.log('=== [EDIT] parentId:', todo.parentId, 'Is subtodo:', isSubtodo);

    // 先隐藏模态框，确保 key 从 'new' 开始
    showModal.value = false;

    // 等待 DOM 更新完成
    await nextTick();

    // 然后设置所有值（此时模态框已隐藏，不会触发重新挂载）
    editingTodoId.value = id;
    newTodoContent.value = todo.content;
    newTodoRepeatMode.value = todo.repeatMode as RepeatMode;

    // Parse weekdays for weekly mode (only for parent todos)
    if (!isSubtodo && todo.repeatMode === 'weekly' && todo.weekdays) {
      newTodoWeekdays.value = todo.weekdays.split(',')
        .map(d => parseInt(d.trim()))
        .filter(d => d >= 1 && d <= 7);
    } else {
      newTodoWeekdays.value = [];
    }

    // Parse specific_dates for specific_dates mode (only for parent todos)
    if (!isSubtodo && todo.repeatMode === 'specific_dates' && todo.specificDates) {
      newTodoSpecificDates.value = todo.specificDates.split(',')
        .map(d => d.trim())
        .filter(d => d);
    } else {
      newTodoSpecificDates.value = [];
    }

    // Load subtodos if editing a parent todo
    if (!isSubtodo && todo.subtodos && todo.subtodos.length > 0) {
      modalSubtodos.value = todo.subtodos.map(st => ({
        id: st.id,
        content: st.content
      }));
      console.log('=== [EDIT] Loaded subtodos:', modalSubtodos.value.length);
    } else {
      modalSubtodos.value = [];
      console.log('=== [EDIT] No existing subtodos');
    }

    // 再次等待 DOM 更新
    await nextTick();

    // 最后显示模态框（此时 key 和值都已正确设置）
    showModal.value = true;

    console.log('=== [EDIT] Modal should now be visible, addingSubtodoForParentId=', addingSubtodoForParentId.value);
  } else {
    console.log('=== [EDIT] ERROR: Todo not found!');
  }
};

// Close modal
const closeAddModal = () => {
  showModal.value = false;
  editingTodoId.value = null;
  addingSubtodoForParentId.value = null;
  editingSubtodo.value = false;
  modalSubtodos.value = [];
  newSubtodoContent.value = '';
};

// Open delete confirmation
const openDeleteConfirm = (id: string) => {
  const todo = findTodoInTree(todos.value, id);
  if (todo) {
    deletingTodoId.value = id;
    deletingTodoContent.value = todo.content;
    deletingTodoRepeatMode.value = todo.repeatMode as RepeatMode;
    showDeleteConfirm.value = true;
  }
};

// Close delete confirmation
const closeDeleteConfirm = () => {
  showDeleteConfirm.value = false;
  deletingTodoId.value = null;
  deletingTodoContent.value = '';
};

// Confirm delete
const confirmDelete = async () => {
  if (deletingTodoId.value) {
    try {
      await deleteTodo(deletingTodoId.value);
      closeDeleteConfirm();
    } catch (e) {
      console.error('Failed to delete todo:', e);
    }
  }
};

// Open add subtodo modal
const openAddSubtodoModal = (parentId: string) => {
  addingSubtodoForParentId.value = parentId;
  editingSubtodo.value = false;
  editingTodoId.value = null;
  newTodoContent.value = '';
  newTodoRepeatMode.value = 'none';
  newTodoWeekdays.value = [];
  showModal.value = true;
};

// Add a subtodo in the modal
const addModalSubtodo = () => {
  if (newSubtodoContent.value.trim()) {
    modalSubtodos.value.push({
      id: Date.now().toString(), // Temporary ID
      content: newSubtodoContent.value.trim()
    });
    newSubtodoContent.value = '';
  }
};

// Remove a subtodo from the modal
const removeModalSubtodo = (tempId: string) => {
  modalSubtodos.value = modalSubtodos.value.filter(st => st.id !== tempId);
};

// Update a subtodo content in the modal
const updateModalSubtodo = (tempId: string, event: Event) => {
  const target = event.target as HTMLInputElement;
  const subtodo = modalSubtodos.value.find(st => st.id === tempId);
  if (subtodo) {
    subtodo.content = target.value;
  }
};

// Handle toggle-parent event (show confirmation)
const handleToggleParent = (id: string) => {
  const todo = findTodoInTree(todos.value, id);
  if (todo) {
    parentCompleteId.value = id;
    parentCompleteContent.value = todo.content;
    showParentCompleteConfirm.value = true;
  }
};

// Find todo in tree structure (recursive)
const findTodoInTree = (todoList: typeof todos.value, id: string): typeof todos.value[0] | null => {
  console.log('=== [FIND] Searching for id:', id, 'in', todoList.map(t => ({ id: t.id, content: t.content, hasSubtodos: !!t.subtodos })));
  for (const todo of todoList) {
    if (todo.id === id) {
      console.log('=== [FIND] Found todo:', todo.content);
      return todo;
    }
    if (todo.subtodos) {
      const found = findTodoInTree(todo.subtodos, id);
      if (found) return found;
    }
  }
  console.log('=== [FIND] Not found!');
  return null;
};

// Close parent completion confirmation
const closeParentCompleteConfirm = () => {
  showParentCompleteConfirm.value = false;
  parentCompleteId.value = null;
  parentCompleteContent.value = '';
};

// Confirm parent completion (complete all subtodos)
const confirmParentComplete = async () => {
  if (parentCompleteId.value) {
    try {
      await completeAllSubtodos(parentCompleteId.value);
      closeParentCompleteConfirm();
    } catch (e) {
      console.error('Failed to complete parent:', e);
    }
  }
};

// Move up/down handlers - get filtered todos to preserve correct items for current view date
const handleMoveUp = async (id: string) => {
  try {
    // First perform the move operation
    await invoke('move_todo_up', { id });
    // Then get filtered todos for current view date
    const flatTodos = await invoke<TodoItemType[]>('get_todos', {
      targetDate: viewDate.value || null
    });
    // Build tree and update directly without triggering loading state
    todos.value = buildTodoTreeFromFlat(flatTodos);
  } catch (e) {
    console.error('Failed to move up:', e);
  }
};

const handleMoveDown = async (id: string) => {
  try {
    // First perform the move operation
    await invoke('move_todo_down', { id });
    // Then get filtered todos for current view date
    const flatTodos = await invoke<TodoItemType[]>('get_todos', {
      targetDate: viewDate.value || null
    });
    // Build tree and update directly without triggering loading state
    todos.value = buildTodoTreeFromFlat(flatTodos);
  } catch (e) {
    console.error('Failed to move down:', e);
  }
};

// Handle complete early - show confirmation first
const handleCompleteEarly = (id: string) => {
  const todo = findTodoInTree(todos.value, id);
  if (todo) {
    completeEarlyId.value = id;
    completeEarlyContent.value = todo.content;
    showCompleteEarlyConfirm.value = true;
  }
};

// Close complete early confirmation
const closeCompleteEarlyConfirm = () => {
  showCompleteEarlyConfirm.value = false;
  completeEarlyId.value = null;
  completeEarlyContent.value = '';
};

// Confirm complete early
const confirmCompleteEarly = async () => {
  if (completeEarlyId.value) {
    try {
      await completeEarly(completeEarlyId.value);
      closeCompleteEarlyConfirm();
    } catch (e) {
      console.error('Failed to complete early:', e);
    }
  }
};

// Scroll to top function
const scrollToTop = () => {
  window.scrollTo({ top: 0, behavior: 'smooth' });
};

// Helper function to build tree from flat todos (copied from useTodos.ts)
const buildTodoTreeFromFlat = (flatTodos: TodoItemType[]): TodoItemType[] => {
  const todoMap = new Map<string, TodoItemType>();
  flatTodos.forEach(todo => {
    todoMap.set(todo.id, { ...todo, subtodos: [] });
  });

  const rootTodos: TodoItemType[] = [];
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
        rootTodos.push(node);
      }
    } else {
      rootTodos.push(node);
    }
  });

  return rootTodos;
};

// Submit new todo or edit
const submitAddTodo = async () => {
  if (!newTodoContent.value.trim()) return;

  try {
    // Format weekdays as comma-separated string
    const weekdaysStr = newTodoRepeatMode.value === 'weekly' && newTodoWeekdays.value.length > 0
      ? newTodoWeekdays.value.sort((a, b) => a - b).join(',')
      : undefined;

    // Format specific_dates as comma-separated string
    const specificDatesStr = newTodoRepeatMode.value === 'specific_dates' && newTodoSpecificDates.value.length > 0
      ? newTodoSpecificDates.value.sort().join(',')
      : undefined;

    console.log("=== [SUBMIT] Starting submission, modalSubtodos.length =", modalSubtodos.value.length);
    console.log("=== [SUBMIT] newSubtodoContent =", newSubtodoContent.value.trim());

    if (editingTodoId.value) {
      // Edit existing todo - first check if it's a parent todo with subtodos
      const isParent = todos.value.some(t => t.parentId === editingTodoId.value);

      // Update the parent todo content/settings
      await invoke('edit_todo', {
        id: editingTodoId.value,
        content: newTodoContent.value.trim(),
        repeatMode: newTodoRepeatMode.value,
        weekdays: weekdaysStr,
        specificDates: specificDatesStr,
      });

      // If editing a parent todo, handle subtodos
      if (isParent || modalSubtodos.value.length > 0) {
        // Get current flat todos to find existing subtodos (use current view date)
        const currentTodos = await invoke<Array<TodoItemType>>('get_todos', {
          targetDate: viewDate.value || null
        });
        const existingSubtodos = currentTodos.filter(t => t.parentId === editingTodoId.value);

        // Delete all existing subtodos
        for (const subtodo of existingSubtodos) {
          await invoke('delete_todo', { id: subtodo.id });
        }

        // Add all subtodos from modal (skip empty ones)
        if (newSubtodoContent.value.trim()) {
          modalSubtodos.value.push({
            id: Date.now().toString(),
            content: newSubtodoContent.value.trim()
          });
          newSubtodoContent.value = '';
        }

        for (const subtodo of modalSubtodos.value) {
          const content = subtodo.content.trim();
          if (content) {
            await invoke('add_todo', {
              content: content,
              repeatMode: 'none',
              weekdays: null,
              parentId: editingTodoId.value,
            });
          }
        }
      }

      // Reload data directly without triggering loading state to preserve scroll position
      const flatTodos = await invoke<TodoItemType[]>('get_todos', {
        targetDate: viewDate.value || null
      });
      todos.value = buildTodoTreeFromFlat(flatTodos);
    } else if (addingSubtodoForParentId.value) {
      // Adding a subtodo to existing parent
      await handleAdd(
        newTodoContent.value.trim(),
        'none',
        undefined,  // weekdays
        undefined,  // specificDates
        addingSubtodoForParentId.value  // parentId
      );
      // Close modal after adding subtodo
      closeAddModal();
    } else {
      // Creating new parent todo first
      let parentId: string | null = null;
      const parentContent = newTodoContent.value.trim();
      const result = await invoke<Array<{ id: string; content: string; createdAt: number }>>('add_todo', {
        content: parentContent,
        repeatMode: newTodoRepeatMode.value,
        weekdays: weekdaysStr,
        specificDates: specificDatesStr,
        parentId: null,
      });

      // Find the parent by content and get the one with latest createdAt
      const matchingTodos = result.filter(t => t.content === parentContent);
      if (matchingTodos.length > 0) {
        // Sort by createdAt descending and take the first (newest)
        matchingTodos.sort((a, b) => b.createdAt - a.createdAt);
        parentId = matchingTodos[0].id;
      }

      // Add all subtodos (skip empty ones)
      if (parentId) {
        console.log("=== [ADD] parentId found, preparing to save subtodos...");

        // First, add any content in the input box
        if (newSubtodoContent.value.trim()) {
          console.log("=== [ADD] Found content in input box, auto-adding:", newSubtodoContent.value.trim());
          modalSubtodos.value.push({
            id: Date.now().toString(),
            content: newSubtodoContent.value.trim()
          });
          console.log("=== [ADD] Auto-added input content to modalSubtodos:", newSubtodoContent.value.trim());
          newSubtodoContent.value = '';
        } else {
          console.log("=== [ADD] No content in input box to auto-add");
        }

        console.log("=== [ADD] Total subtodos to save:", modalSubtodos.value.length);
        console.log("=== [ADD] modalSubtodos contents:", modalSubtodos.value.map(st => `'${st.content}'`));

        for (let i = 0; i < modalSubtodos.value.length; i++) {
          const subtodo = modalSubtodos.value[i];
          console.log(`=== [ADD] Saving subtodo ${i+1}/${modalSubtodos.value.length}: '${subtodo.content}'`);
          const content = subtodo.content.trim();
          if (content) {
            console.log(`=== [ADD] Calling add_todo for: '${content}', parentId: ${parentId}`);
            await invoke('add_todo', {
              content: content,
              repeatMode: 'none',
              weekdays: null,
              parentId: parentId,
            });
            console.log("=== [ADD] Subtodo saved successfully");
          } else {
            console.log("=== [ADD] Skipping empty subtodo");
          }
        }

        console.log("=== [ADD] All subtodos saved, reloading...");
      } else {
        console.log("=== [ADD] No parentId, skipping subtodo save. parentId =", parentId);
      }

      // Reload to get the tree structure (use current view date)
      await loadTodos(viewDate.value);
    }
    closeAddModal();
  } catch (e) {
    console.error('Failed to submit todo:', e);
  }
};

// Handle textarea keydown (Shift+Enter for newline, Enter to submit)
const handleTextareaKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    submitAddTodo();
  }
};

// Debounced save memo function
let saveMemoTimer: ReturnType<typeof setTimeout> | null = null;
const updateMemo = async (content: string) => {
  memo.value = content;
  if (saveMemoTimer) clearTimeout(saveMemoTimer);
  saveMemoTimer = setTimeout(async () => {
    try {
      isSavingMemo.value = true;
      await invoke('update_memo', { content });
      console.log('Memo saved successfully');
    } catch (e) {
      console.error('Failed to save memo:', e);
    } finally {
      isSavingMemo.value = false;
    }
  }, 500); // 500ms debounce
};

// Save memo height
let saveMemoHeightTimer: ReturnType<typeof setTimeout> | null = null;
const updateMemoHeight = async (height: number) => {
  memoHeight.value = height;
  if (saveMemoHeightTimer) clearTimeout(saveMemoHeightTimer);
  saveMemoHeightTimer = setTimeout(async () => {
    try {
      await invoke('update_memo_height', { height });
      console.log('Memo height saved:', height);
    } catch (e) {
      console.error('Failed to save memo height:', e);
    }
  }, 300); // 300ms debounce
};

// Select a date to view
const selectDate = async (isoDate: string) => {
  viewDate.value = isoDate;
  showDatePicker.value = false;
  setViewDate(isoDate);
  await loadTodos(isoDate);
};

// Go back to today
const goToToday = async () => {
  viewDate.value = '';
  showDatePicker.value = false;
  setViewDate('');
  await loadTodos();
};
</script>

<template>
  <template v-if="true">
    <main class="app">
      <header class="app-header">
        <div class="header-content drag-region" data-tauri-drag-region>
          <div class="date-info">
            <div class="date-main">
              <span class="date-text">{{ viewDateDisplay.date }}</span>
              <span class="weekday-text">{{ viewDateDisplay.weekday }}</span>
            </div>
            <div class="lunar-info">
              <span class="lunar-zodiac">{{ viewDateDisplay.lunarZodiac }}</span>
              <span class="lunar-date">{{ viewDateDisplay.lunarMonthText }}</span>
            </div>
            <button v-if="viewDate" @click="goToToday" class="back-today-btn" title="回到今天">
              回到今天
            </button>
          </div>
        </div>
        <div class="header-actions">
          <button @click="openAddModal" class="add-todo-btn" title="新增待办">
            <span class="add-icon">+</span>
            <span class="add-text">新增待办</span>
          </button>
          <div class="date-picker-wrapper">
            <button @click="showDatePicker = !showDatePicker" class="date-picker-btn" title="查看其他日期">
              📅
            </button>
            <div v-if="showDatePicker" class="date-picker-dropdown">
              <div class="week-dates">
                <button
                  v-for="date in weekDates"
                  :key="date.isoDate"
                  @click="!date.isPast && selectDate(date.isoDate)"
                  class="week-date-btn"
                  :class="{ 'is-past': date.isPast, 'is-selected': viewDateDisplay.isoDate === date.isoDate }"
                  :disabled="date.isPast"
                  :title="date.isoDate"
                >
                  <div class="week-date-weekday">{{ date.weekday }}</div>
                  <div class="week-date-day">{{ date.day }}</div>
                </button>
              </div>
            </div>
          </div>
          <button @click="showSettings = true" class="settings-btn" title="设置">
            <span class="settings-icon">⚙️</span>
          </button>
        </div>
      </header>

      <div class="app-content">
        <div v-if="error" class="error-message">
          <span class="error-icon">⚠️</span>
          <span>{{ error }}</span>
        </div>

        <!-- Memo section -->
        <div class="memo-section">
          <textarea
            :value="memo"
            @input="(e) => updateMemo((e.target as HTMLTextAreaElement).value)"
            @mouseup="updateMemoHeight(($event.target as HTMLTextAreaElement).offsetHeight)"
            class="memo-textarea"
            placeholder="在这里记录你的备忘...（支持多行，自动保存）"
            :style="{ height: memoHeight + 'px' }"
          ></textarea>
          <div v-if="isSavingMemo" class="memo-saving">保存中...</div>
        </div>

        <TodoList
          :todos="todos"
          :loading="loading"
          :is-today="isViewingToday"
          @toggle="toggleTodo"
          @delete="openDeleteConfirm"
          @edit="openEditModal"
          @toggle-expand="toggleExpand"
          @add-subtodo="openAddSubtodoModal"
          @toggle-parent="handleToggleParent"
          @move-up="handleMoveUp"
          @move-down="handleMoveDown"
          @complete-early="handleCompleteEarly"
          @toggle-disable="toggleDisable"
        />
      </div>
    </main>

    <!-- Progress Bar - Fixed at bottom -->
    <div class="progress-bar-fixed">
      <div class="progress-info">
        <span class="progress-text">{{ progressText }}</span>
        <span class="progress-percent">{{ progress }}%</span>
      </div>
      <div class="progress-track">
        <div class="progress-fill" :style="{ width: progress + '%' }"></div>
      </div>
    </div>

    <!-- Scroll to Top Button -->
    <button @click="scrollToTop" class="scroll-to-top-btn" title="回到顶部">
      ↑
    </button>

    <!-- Add Todo Modal -->
    <div v-if="showModal" class="modal-overlay" @click.self="closeAddModal">
      <div class="modal-content" :key="editingTodoId || 'new'">
        <div class="modal-header">
          <h2 class="modal-title">
            {{ addingSubtodoForParentId ? '添加子待办' : (editingTodoId ? (editingSubtodo ? '编辑子待办' : '编辑父待办') : '新增待办') }}
          </h2>
          <button @click="closeAddModal" class="modal-close">✕</button>
        </div>
        <div class="modal-body">
          <div class="form-group">
            <label class="form-label">待办内容</label>
            <textarea
              v-model="newTodoContent"
              class="form-textarea"
              placeholder="输入待办事项... (Shift+Enter 换行，Enter 确认)"
              rows="3"
              @keydown="handleTextareaKeydown"
            ></textarea>
          </div>

          <!-- Subtodos section (only for parent todos, not when editing/adding subtodos) -->
          <div v-if="!addingSubtodoForParentId && !editingSubtodo" class="form-group">
            <div class="subtodos-header">
              <label class="form-label">子待办</label>
              <!-- Show "add subtodo" button -->
              <button @click="addModalSubtodo" class="btn-add-subtodo-small">
                <span>➕ 添加子待办</span>
              </button>
            </div>

            <!-- Subtodos list -->
            <div v-if="modalSubtodos.length > 0" class="subtodos-list">
              <div
                v-for="subtodo in modalSubtodos"
                :key="subtodo.id"
                class="subtodo-item"
              >
                <input
                  type="text"
                  :value="subtodo.content"
                  @input="updateModalSubtodo(subtodo.id, $event)"
                  class="subtodo-input"
                  placeholder="子待办内容..."
                />
                <button @click="removeModalSubtodo(subtodo.id)" class="subtodo-remove">✕</button>
              </div>
            </div>

            <!-- Add subtodo input -->
            <div class="add-subtodo-input">
              <input
                type="text"
                v-model="newSubtodoContent"
                placeholder="输入子待办内容，回车添加..."
                class="form-input"
                @keydown.enter="addModalSubtodo"
              />
            </div>
          </div>

          <!-- Repeat mode section (only for parent todos, not when editing/adding subtodos) -->
          <div v-if="!addingSubtodoForParentId && !editingSubtodo" class="form-group">
            <label class="form-label">重复方式</label>
            <div class="repeat-options">
              <label class="repeat-option">
                <input type="radio" v-model="newTodoRepeatMode" value="none" />
                <span>不重复</span>
              </label>
              <label class="repeat-option">
                <input type="radio" v-model="newTodoRepeatMode" value="daily" />
                <span>每日</span>
              </label>
              <label class="repeat-option">
                <input type="radio" v-model="newTodoRepeatMode" value="weekly" />
                <span>每周</span>
              </label>
              <label class="repeat-option">
                <input type="radio" v-model="newTodoRepeatMode" value="specific_dates" />
                <span>指定日期</span>
              </label>
            </div>

            <!-- Date selector for specific_dates mode -->
            <div v-if="newTodoRepeatMode === 'specific_dates'" class="dates-selector">
              <DatePicker v-model="newTodoSpecificDates" :min-date="minDate" />
              <span v-if="newTodoSpecificDates.length === 0" class="form-hint warning">请至少选择一个日期</span>
            </div>

            <!-- Weekdays selector for weekly mode -->
            <div v-if="newTodoRepeatMode === 'weekly'" class="weekdays-selector">
              <div class="weekdays-grid">
                <label v-for="day in weekdayOptions" :key="day.value" class="weekday-checkbox">
                  <input
                    type="checkbox"
                    :value="day.value"
                    v-model="newTodoWeekdays"
                  />
                  <span>{{ day.label }}</span>
                </label>
              </div>
              <span v-if="newTodoWeekdays.length === 0" class="form-hint warning">请至少选择一天</span>
            </div>
          </div>
        </div>
        <div class="modal-footer">
          <button @click="closeAddModal" class="btn-cancel">取消</button>
          <button @click="submitAddTodo" class="btn-confirm">{{ editingTodoId ? '保存' : '确定' }}</button>
        </div>
      </div>
    </div>

    <!-- Delete Confirmation Modal -->
    <div v-if="showDeleteConfirm" class="modal-overlay" @click.self="closeDeleteConfirm">
      <div class="modal-content modal-small">
        <div class="modal-header">
          <h2 class="modal-title">确认删除</h2>
          <button @click="closeDeleteConfirm" class="modal-close">✕</button>
        </div>
        <div class="modal-body">
          <p class="delete-message">
            确定要删除以下待办吗？
          </p>
          <p class="delete-todo-content">{{ deletingTodoContent }}</p>
          <p v-if="deletingTodoRepeatMode === 'daily'" class="delete-warning">
            ⚠️ 这是一个每天重复的待办，删除后将永久消失，第二天不会再出现。
          </p>
          <p v-if="deletingTodoRepeatMode === 'weekly'" class="delete-warning">
            ⚠️ 这是一个每周重复的待办，删除后将永久消失，下周不会再出现。
          </p>
        </div>
        <div class="modal-footer">
          <button @click="closeDeleteConfirm" class="btn-cancel">取消</button>
          <button @click="confirmDelete" class="btn-confirm btn-danger">确认删除</button>
        </div>
      </div>
    </div>

    <!-- Parent Todo Completion Confirmation Modal -->
    <div v-if="showParentCompleteConfirm" class="modal-overlay" @click.self="closeParentCompleteConfirm">
      <div class="modal-content modal-small">
        <div class="modal-header">
          <h2 class="modal-title">完成父待办</h2>
          <button @click="closeParentCompleteConfirm" class="modal-close">✕</button>
        </div>
        <div class="modal-body">
          <p class="delete-message">
            确定要完成这个父待办吗？
          </p>
          <p class="delete-todo-content">"{{ parentCompleteContent }}"</p>
          <p class="delete-warning">
            ⚠️ 确认后，所有子待办都会被标记为完成。
          </p>
        </div>
        <div class="modal-footer">
          <button @click="closeParentCompleteConfirm" class="btn-cancel">取消</button>
          <button @click="confirmParentComplete" class="btn-confirm">确认完成</button>
        </div>
      </div>
    </div>

    <!-- Complete Early Confirmation Modal -->
    <div v-if="showCompleteEarlyConfirm" class="modal-overlay" @click.self="closeCompleteEarlyConfirm">
      <div class="modal-content modal-small">
        <div class="modal-header">
          <h2 class="modal-title">提前完成</h2>
          <button @click="closeCompleteEarlyConfirm" class="modal-close">✕</button>
        </div>
        <div class="modal-body">
          <p class="delete-message">
            确定要提前完成这个待办吗？
          </p>
          <p class="delete-todo-content">"{{ completeEarlyContent }}"</p>
          <p class="delete-warning">
            ⚠️ 提前完成后，待办将在下一个周期重置。此操作不可撤销。
          </p>
        </div>
        <div class="modal-footer">
          <button @click="closeCompleteEarlyConfirm" class="btn-cancel">取消</button>
          <button @click="confirmCompleteEarly" class="btn-confirm">确认完成</button>
        </div>
      </div>
    </div>

    <!-- Settings Modal -->
    <div v-if="showSettings" class="modal-overlay" @click.self="showSettings = false">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h2 class="modal-title">设置</h2>
          <button @click="showSettings = false" class="modal-close">✕</button>
        </div>
        <div class="modal-body modal-body-scroll">
          <Settings @shortcuts-changed="handleShortcutsChanged" />
        </div>
      </div>
    </div>
  </template>
</template>

<style>
:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --bg-app: #f5f7fa;
  --text-primary: #1a1a1a;
  --text-secondary: #6c757d;
  --border-color: #dee2e6;
  --primary-color: #4f46e5;
  --shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #2a2a2a;
    --bg-app: #121212;
    --text-primary: #e9ecef;
    --text-secondary: #adb5bd;
    --border-color: #495057;
    --primary-color: #6366f1;
    --shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen,
    Ubuntu, Cantarell, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#app {
  min-height: 100vh;
  background: var(--bg-app);
}

.app {
  max-width: 600px;
  margin: 0 auto;
  padding: 32px 24px;
}

.app-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  gap: 12px;
}

.header-content {
  flex: 1;
  min-width: 0;
}

.drag-region {
  cursor: move;
  user-select: none;
  -webkit-user-select: none;
}

.drag-region:active {
  cursor: grabbing;
}

/* Ensure all children of drag region don't block drag */
.drag-region > * {
  pointer-events: none;
}

.date-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-start;
}

.date-main {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.lunar-info {
  display: flex;
  align-items: center;
  gap: 6px;
}

.lunar-zodiac {
  font-size: 16px;
  color: var(--primary-color);
  font-weight: 500;
}

.lunar-date {
  font-size: 16px;
  color: var(--text-secondary);
}

.back-today-btn {
  margin-top: 2px;
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  background: var(--primary-color);
  color: white;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  pointer-events: auto; /* Override parent's pointer-events: none */
}

.back-today-btn:hover {
  opacity: 0.9;
  transform: scale(1.05);
}

.back-today-btn:active {
  transform: scale(0.95);
}

.date-text {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 1px;
}

.weekday-text {
  font-size: 18px;
  color: var(--text-secondary);
  font-weight: 500;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.add-todo-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 16px;
  border: none;
  border-radius: 8px;
  background: var(--primary-color);
  color: white;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.add-todo-btn:hover {
  opacity: 0.9;
  transform: translateY(-1px);
}

.add-todo-btn:active {
  transform: translateY(0);
}

.add-icon {
  font-size: 18px;
  line-height: 1;
}

.add-text {
  font-size: 14px;
}

.settings-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 16px;
  cursor: pointer;
  transition: all 0.2s;
}

.settings-btn:hover {
  background: var(--bg-secondary);
  border-color: var(--primary-color);
}

/* Date Picker Styles */
.date-picker-wrapper {
  position: relative;
}

.date-picker-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 16px;
  cursor: pointer;
  transition: all 0.2s;
}

.date-picker-btn:hover {
  background: var(--bg-secondary);
  border-color: var(--primary-color);
}

.date-picker-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 100;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  padding: 12px;
  min-width: 280px;
}

.week-dates {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 6px;
}

.week-date-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 6px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.2s;
}

.week-date-btn:hover:not(.is-past):not(:disabled) {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.week-date-btn.is-past {
  opacity: 0.4;
  cursor: not-allowed;
  background: var(--bg-secondary);
}

.week-date-btn.is-selected {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.week-date-weekday {
  font-size: 11px;
  font-weight: 500;
}

.week-date-day {
  font-size: 16px;
  font-weight: 600;
}

/* Memo Section Styles */
.memo-section {
  position: relative;
  margin-bottom: 16px;
}

.memo-textarea {
  width: 100%;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  min-height: 80px;
  max-height: 200px;
  line-height: 1.5;
  transition: border-color 0.2s;
}

.memo-textarea:focus {
  outline: none;
  border-color: var(--primary-color);
}

.memo-textarea::placeholder {
  color: var(--text-secondary);
  opacity: 0.7;
}

.memo-saving {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 4px 8px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 12px;
  border-radius: 4px;
  pointer-events: none;
}

.app-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-bottom: 70px;
}

.error-message {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: #fee2e2;
  color: #dc2626;
  border-radius: 8px;
  font-size: 14px;
}

@media (prefers-color-scheme: dark) {
  .error-message {
    background: #7f1d1d;
    color: #fca5a5;
  }
}

.error-icon {
  font-size: 16px;
}

/* Modal Styles */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.modal-content {
  background: var(--bg-primary);
  border-radius: 12px;
  width: 100%;
  max-width: 400px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
}

.modal-content.modal-large {
  max-width: 500px;
}

.modal-content.modal-small {
  max-width: 360px;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px;
  border-bottom: 1px solid var(--border-color);
}

.modal-title {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary);
}

.modal-close {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 18px;
  cursor: pointer;
  transition: all 0.2s;
}

.modal-close:hover {
  background: var(--bg-secondary);
}

.modal-body {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow-y: auto;
  min-height: 0;
}

.modal-body-scroll {
  max-height: 60vh;
  overflow-y: auto;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border-color);
}

.form-group:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.form-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.form-textarea {
  width: 100%;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  resize: none;
}

.form-textarea:focus {
  outline: none;
  border-color: var(--primary-color);
}

.form-hint {
  display: block;
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-secondary);
}

.repeat-options {
  display: flex;
  gap: 12px;
}

.repeat-option {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  flex: 1;
  justify-content: center;
}

.repeat-option:hover {
  background: var(--bg-secondary);
}

.repeat-option input[type="radio"] {
  cursor: pointer;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.repeat-option span {
  font-size: 14px;
  color: var(--text-primary);
  user-select: none;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
}

.btn-cancel,
.btn-confirm {
  padding: 10px 20px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-cancel {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.btn-cancel:hover {
  background: var(--border-color);
}

.btn-confirm {
  background: var(--primary-color);
  color: white;
}

.btn-confirm:hover {
  opacity: 0.9;
}

:root {
  --danger-color: #ef4444;
  --danger-bg: #fee2e2;
}

@media (prefers-color-scheme: dark) {
  :root {
    --danger-color: #f87171;
    --danger-bg: #7f1d1d;
  }
}

/* Delete Confirmation Modal Styles */
.delete-message {
  font-size: 14px;
  color: var(--text-primary);
  margin: 0 0 12px 0;
}

.delete-todo-content {
  font-size: 15px;
  color: var(--text-primary);
  font-weight: 500;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 8px;
  margin: 0 0 16px 0;
  word-break: break-word;
}

.delete-warning {
  font-size: 13px;
  color: var(--danger-color);
  background: var(--danger-bg);
  padding: 10px 12px;
  border-radius: 6px;
  margin: 0;
}

.btn-danger {
  background: var(--danger-color);
}

.btn-danger:hover {
  opacity: 0.9;
}

/* Weekdays Selector Styles */
.weekdays-selector {
  margin-top: 12px;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 8px;
}

.weekdays-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.weekday-checkbox {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  background: var(--bg-primary);
}

.weekday-checkbox:hover {
  border-color: var(--primary-color);
}

.weekday-checkbox input[type="checkbox"] {
  cursor: pointer;
  width: 16px;
  height: 16px;
}

.weekday-checkbox span {
  font-size: 13px;
  color: var(--text-primary);
  user-select: none;
}

.weekday-checkbox input:checked + span {
  color: var(--primary-color);
  font-weight: 500;
}

.form-hint.warning {
  color: var(--danger-color);
}

.dates-selector {
  margin-top: 12px;
  padding: 12px;
  background: var(--bg-secondary);
  border-radius: 8px;
}

/* Subtodos in Modal Styles */
.subtodos-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
}

.subtodos-header .form-label {
  margin-bottom: 0;
}

.btn-add-subtodo-small {
  padding: 6px 12px;
  border: 1px solid var(--primary-color);
  border-radius: 6px;
  background: transparent;
  color: var(--primary-color);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.btn-add-subtodo-small:hover {
  background: var(--primary-color);
  color: white;
}

.subtodos-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 8px;
}

.subtodo-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 6px 0;
}

.subtodo-input {
  flex: 1;
  font-size: 14px;
  color: var(--text-primary);
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
}

.subtodo-input::placeholder {
  color: var(--text-secondary);
}

.subtodo-remove {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 4px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
}

.subtodo-remove:hover {
  background: var(--danger-bg);
  color: var(--danger-color);
}

.add-subtodo-input {
  margin-top: 8px;
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
}

.form-input:focus {
  outline: none;
  border-color: var(--primary-color);
}

/* Progress Bar - Fixed at bottom */
.progress-bar-fixed {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  background: var(--bg-primary);
  border-top: 1px solid var(--border-color);
  padding: 12px 24px;
  box-shadow: 0 -2px 10px rgba(0, 0, 0, 0.05);
  z-index: 100;
}

.progress-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.progress-text {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 500;
}

.progress-percent {
  font-size: 14px;
  color: var(--primary-color);
  font-weight: 600;
}

.progress-track {
  width: 100%;
  height: 8px;
  background: var(--bg-secondary);
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--primary-color), #818cf8);
  border-radius: 4px;
  transition: width 0.3s ease;
}

/* Scroll to Top Button */
.scroll-to-top-btn {
  position: fixed;
  bottom: 80px;
  right: 24px;
  width: 44px;
  height: 44px;
  border: none;
  border-radius: 50%;
  background: var(--primary-color);
  color: white;
  font-size: 20px;
  font-weight: bold;
  cursor: pointer;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  transition: all 0.3s ease;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
}

.scroll-to-top-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(0, 0, 0, 0.2);
  opacity: 0.9;
}

.scroll-to-top-btn:active {
  transform: translateY(0);
}
</style>
