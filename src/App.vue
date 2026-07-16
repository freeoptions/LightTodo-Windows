<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick } from 'vue';
import TodoList from './components/TodoList.vue';
import Settings from './components/Settings.vue';
import DatePicker from './components/DatePicker.vue';
import LongTermTodos from './components/LongTermTodos.vue';
import { useTodos } from './composables/useTodos';
import { type RepeatMode, type TodoItem as TodoItemType } from './types/todo';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Settings as SettingsType } from './types/settings';
// @ts-ignore - CommonJS module
import { Solar } from 'lunar-javascript';

const {
  todos,
  loading,
  error,
  loadTodos,
  setViewDate,
  toggleTodo,
  deleteTodo,
  toggleExpand,
  completeAllSubtodos,
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
const newSubtodoCycleEnabled = ref(false);
const newSubtodoCycleStartDate = ref('');
const newSubtodoCycleActiveDays = ref(7);
const newSubtodoCycleIntervalWeeks = ref(4);
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
const modalSubtodos = ref<Array<{
  id: string;
  content: string;
  cycleStartDate?: string;
  cycleActiveDays?: number;
  cycleIntervalWeeks?: number;
}>>([]);
const newSubtodoContent = ref('');

// Parent todo completion confirmation state
const showParentCompleteConfirm = ref(false);
const parentCompleteId = ref<string | null>(null);
const parentCompleteContent = ref('');

// Settings state
const showSettings = ref(false);
const showLongTermTodos = ref(false);
const memo = ref('');
const memoHeight = ref(80);
const isSavingMemo = ref(false);
const backgroundWallpaper = ref<string | null>(null);

// View date state (for viewing todos on different days)
const viewDate = ref('');
const showDatePicker = ref(false);

const currentWindow = getCurrentWindow();

const applyBackgroundWallpaper = (wallpaper: string | null) => {
  backgroundWallpaper.value = wallpaper;
  document.documentElement.style.setProperty('--app-wallpaper-image', wallpaper ? `url("${wallpaper}")` : 'none');
};

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

const activeParentTabId = ref<string | null>(null);

const parentTabs = computed(() => {
  return todos.value
    .filter((todo) => !todo.parentId)
    .map((todo, index) => ({
      id: todo.id,
      title: todo.content.trim() || '未命名待办',
      completed: todo.completed,
      index,
    }))
    .sort((a, b) => {
      if (a.completed !== b.completed) return a.completed ? 1 : -1;
      return a.index - b.index;
    });
});

const handleParentTabsWheel = (event: WheelEvent) => {
  const target = event.currentTarget as HTMLElement | null;
  if (!target || target.scrollWidth <= target.clientWidth) return;

  event.preventDefault();
  target.scrollLeft += event.deltaY || event.deltaX;
};

const scrollToParentTodo = async (id: string) => {
  activeParentTabId.value = id;
  await nextTick();

  const target = document.getElementById(`todo-${id}`);
  if (!target) return;

  const tabs = document.querySelector<HTMLElement>('.parent-tabs');
  const tabsHeight = tabs?.getBoundingClientRect().height || 0;
  const targetTop = target.getBoundingClientRect().top + window.scrollY;
  const topPadding = tabsHeight + 14;

  window.scrollTo({
    top: Math.max(0, targetTop - topPadding),
    behavior: 'smooth',
  });
  target.classList.add('tab-scroll-flash');
  window.setTimeout(() => {
    target.classList.remove('tab-scroll-flash');
  }, 1200);
};

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
const saveCurrentWindowState = async () => {
  try {
    const [position, size, scaleFactor] = await Promise.all([
      currentWindow.outerPosition(),
      currentWindow.outerSize(),
      currentWindow.scaleFactor(),
    ]);

    await invoke('save_window_state', {
      x: Math.round(position.x / scaleFactor),
      y: Math.round(position.y / scaleFactor),
      width: Math.round(size.width / scaleFactor),
      height: Math.round(size.height / scaleFactor),
    });
  } catch (e) {
    console.error('Failed to save window state:', e);
  }
};

const debouncedSaveWindowState = () => {
  if (saveStateTimer) clearTimeout(saveStateTimer);
  saveStateTimer = setTimeout(() => {
    saveCurrentWindowState();
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
    applyBackgroundWallpaper(settings.backgroundWallpaper || null);
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

const resetSubtodoCycleForm = () => {
  newSubtodoCycleEnabled.value = false;
  newSubtodoCycleStartDate.value = '';
  newSubtodoCycleActiveDays.value = 7;
  newSubtodoCycleIntervalWeeks.value = 4;
};

const getSubtodoCyclePayload = () => {
  if (!newSubtodoCycleEnabled.value || !newSubtodoCycleStartDate.value) {
    return {
      cycleStartDate: null,
      cycleActiveDays: null,
      cycleIntervalWeeks: null,
    };
  }

  const intervalWeeks = Math.max(1, Math.min(52, Math.trunc(Number(newSubtodoCycleIntervalWeeks.value) || 4)));
  const activeDays = Math.max(1, Math.min(intervalWeeks * 7, Math.trunc(Number(newSubtodoCycleActiveDays.value) || 7)));

  return {
    cycleStartDate: newSubtodoCycleStartDate.value,
    cycleActiveDays: activeDays,
    cycleIntervalWeeks: intervalWeeks,
  };
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
  resetSubtodoCycleForm();
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

    // 绛夊緟 DOM 鏇存柊瀹屾垚
    await nextTick();

    // 然后设置所有值，此时模态框已隐藏，不会触发重新挂载
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

    if (isSubtodo && todo.cycleStartDate) {
      newSubtodoCycleEnabled.value = true;
      newSubtodoCycleStartDate.value = todo.cycleStartDate;
      newSubtodoCycleActiveDays.value = todo.cycleActiveDays || 7;
      newSubtodoCycleIntervalWeeks.value = todo.cycleIntervalWeeks || 4;
    } else {
      resetSubtodoCycleForm();
    }

    // Load subtodos if editing a parent todo
    if (!isSubtodo && todo.subtodos && todo.subtodos.length > 0) {
      modalSubtodos.value = todo.subtodos.map(st => ({
        id: st.id,
        content: st.content,
        cycleStartDate: st.cycleStartDate,
        cycleActiveDays: st.cycleActiveDays,
        cycleIntervalWeeks: st.cycleIntervalWeeks,
      }));
      console.log('=== [EDIT] Loaded subtodos:', modalSubtodos.value.length);
    } else {
      modalSubtodos.value = [];
      console.log('=== [EDIT] No existing subtodos');
    }

    // 鍐嶆绛夊緟 DOM 鏇存柊
    await nextTick();

    // 最后显示模态框，此时 key 和值都已正确设置
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
  resetSubtodoCycleForm();
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
  resetSubtodoCycleForm();
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
  if ((editingSubtodo.value || addingSubtodoForParentId.value) && newSubtodoCycleEnabled.value && !newSubtodoCycleStartDate.value) {
    alert('请先选择首轮启用日期');
    return;
  }

  try {
    // Format weekdays as comma-separated string
    const weekdaysStr = newTodoRepeatMode.value === 'weekly' && newTodoWeekdays.value.length > 0
      ? newTodoWeekdays.value.sort((a, b) => a - b).join(',')
      : undefined;

    // Format specific_dates as comma-separated string
    const specificDatesStr = newTodoRepeatMode.value === 'specific_dates' && newTodoSpecificDates.value.length > 0
      ? newTodoSpecificDates.value.sort().join(',')
      : undefined;
    const subtodoCyclePayload = getSubtodoCyclePayload();

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
        cycleStartDate: editingSubtodo.value ? subtodoCyclePayload.cycleStartDate : null,
        cycleActiveDays: editingSubtodo.value ? subtodoCyclePayload.cycleActiveDays : null,
        cycleIntervalWeeks: editingSubtodo.value ? subtodoCyclePayload.cycleIntervalWeeks : null,
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
              cycleStartDate: subtodo.cycleStartDate || null,
              cycleActiveDays: subtodo.cycleActiveDays || null,
              cycleIntervalWeeks: subtodo.cycleIntervalWeeks || null,
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
      await invoke('add_todo', {
        content: newTodoContent.value.trim(),
        repeatMode: 'none',
        weekdays: null,
        specificDates: null,
        parentId: addingSubtodoForParentId.value,
        cycleStartDate: subtodoCyclePayload.cycleStartDate,
        cycleActiveDays: subtodoCyclePayload.cycleActiveDays,
        cycleIntervalWeeks: subtodoCyclePayload.cycleIntervalWeeks,
      });
      const flatTodos = await invoke<TodoItemType[]>('get_todos', {
        targetDate: viewDate.value || null
      });
      todos.value = buildTodoTreeFromFlat(flatTodos);
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
            content: newSubtodoContent.value.trim(),
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
              cycleStartDate: subtodo.cycleStartDate || null,
              cycleActiveDays: subtodo.cycleActiveDays || null,
              cycleIntervalWeeks: subtodo.cycleIntervalWeeks || null,
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
        <div class="header-content">
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
          <button @click="openAddModal" class="header-action-btn add-todo-btn" title="新增待办">
            <svg class="btn-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 5v14M5 12h14" />
            </svg>
            <span class="add-text">新增</span>
          </button>
          <button @click="showSettings = true" class="header-action-btn settings-btn" title="设置" aria-label="设置">
            <svg class="btn-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 8.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7Z" />
              <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21a2 2 0 1 1-4 0v-.09A1.7 1.7 0 0 0 8.5 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1.1-.4H3a2 2 0 1 1 0-4h.09A1.7 1.7 0 0 0 4.6 8.5a1.7 1.7 0 0 0-.34-1.88l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1.1V3a2 2 0 1 1 4 0v.09A1.7 1.7 0 0 0 15.5 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.7 1.7 0 0 0 19.4 9a1.7 1.7 0 0 0 .6 1 1.7 1.7 0 0 0 1.1.4H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.51.6Z" />
            </svg>
            <span>设置</span>
          </button>
          <button @click="showLongTermTodos = true" class="header-action-btn long-term-btn" title="长期待办">
            <svg class="btn-icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 19.5V5.5A2.5 2.5 0 0 1 6.5 3H20v18H6.5A2.5 2.5 0 0 1 4 18.5" />
              <path d="M8 7h8M8 11h6M8 15h7" />
            </svg>
            <span>长期</span>
          </button>
          <div class="date-picker-wrapper">
            <button @click="showDatePicker = !showDatePicker" class="header-action-btn date-picker-btn" title="查看其他日期" aria-label="查看其他日期">
              <svg class="btn-icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M8 2v4M16 2v4M3.5 9.5h17M5.5 4.5h13A2 2 0 0 1 20.5 6.5v12A2 2 0 0 1 18.5 20.5h-13A2 2 0 0 1 3.5 18.5v-12A2 2 0 0 1 5.5 4.5Z" />
              </svg>
              <span>日期</span>
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
        </div>
      </header>

      <nav
        v-if="parentTabs.length > 0"
        class="parent-tabs"
        aria-label="父待办快速导航"
        @wheel="handleParentTabsWheel"
      >
        <button
          v-for="tab in parentTabs"
          :key="tab.id"
          type="button"
          class="parent-tab"
          :class="{ completed: tab.completed, active: activeParentTabId === tab.id }"
          :title="tab.title"
          @click="scrollToParentTodo(tab.id)"
        >
          <span class="parent-tab-status" aria-hidden="true"></span>
          <span class="parent-tab-title">{{ tab.title }}</span>
        </button>
      </nav>

      <div class="app-content">
        <div v-if="error" class="error-message">
          <svg class="error-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 8v5M12 17h.01M10.3 3.9 2.9 17.1A2 2 0 0 0 4.6 20h14.8a2 2 0 0 0 1.7-2.9L13.7 3.9a2 2 0 0 0-3.4 0Z" />
          </svg>
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
    <button @click="scrollToTop" class="scroll-to-top-btn" title="回到顶部" aria-label="回到顶部">
      <svg class="btn-icon" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 19V5M5 12l7-7 7 7" />
      </svg>
    </button>

    <!-- Add Todo Modal -->
    <div v-if="showModal" class="modal-overlay" @click.self="closeAddModal">
      <div class="modal-content" :key="editingTodoId || 'new'">
        <div class="modal-header">
          <h2 class="modal-title">
            {{ addingSubtodoForParentId ? '添加子待办' : (editingTodoId ? (editingSubtodo ? '编辑子待办' : '编辑父待办') : '新增待办') }}
          </h2>
          <button @click="closeAddModal" class="modal-close">×</button>
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
                <span>添加子待办</span>
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
                <button @click="removeModalSubtodo(subtodo.id)" class="subtodo-remove">×</button>
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

          <div v-if="addingSubtodoForParentId || editingSubtodo" class="form-group cycle-form-group">
            <label class="form-label">启用方式</label>
            <div class="cycle-mode-options">
              <button
                type="button"
                class="cycle-mode-option"
                :class="{ active: !newSubtodoCycleEnabled }"
                @click="newSubtodoCycleEnabled = false"
              >
                普通启用
              </button>
              <button
                type="button"
                class="cycle-mode-option"
                :class="{ active: newSubtodoCycleEnabled }"
                @click="newSubtodoCycleEnabled = true"
              >
                周期启用
              </button>
            </div>
            <span v-if="!newSubtodoCycleEnabled" class="form-hint">普通启用不会按固定周期自动禁用或恢复。</span>
            <div v-if="newSubtodoCycleEnabled" class="cycle-config-panel">
              <label class="cycle-config-field">
                <span>首轮启用日期</span>
                <input type="date" v-model="newSubtodoCycleStartDate" class="form-input" />
              </label>
              <label class="cycle-config-field">
                <span>启用天数</span>
                <input
                  type="number"
                  v-model.number="newSubtodoCycleActiveDays"
                  class="form-input"
                  min="1"
                  :max="newSubtodoCycleIntervalWeeks * 7"
                />
              </label>
              <label class="cycle-config-field">
                <span>间隔周数</span>
                <input
                  type="number"
                  v-model.number="newSubtodoCycleIntervalWeeks"
                  class="form-input"
                  min="1"
                  max="52"
                />
              </label>
              <span class="form-hint">启用期包含开始日；例如 2026-07-14 启用 7 天，就是启用到 2026-07-20，每 4 周再次启用。</span>
              <span v-if="!newSubtodoCycleStartDate" class="form-hint warning">开启周期启用后，请选择首轮启用日期</span>
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
          <button @click="closeDeleteConfirm" class="modal-close">×</button>
        </div>
        <div class="modal-body">
          <p class="delete-message">
            确定要删除以下待办吗？
          </p>
          <p class="delete-todo-content">{{ deletingTodoContent }}</p>
          <p v-if="deletingTodoRepeatMode === 'daily'" class="delete-warning">
            这是一个每天重复的待办，删除后将永久消失，第二天不会再出现。
          </p>
          <p v-if="deletingTodoRepeatMode === 'weekly'" class="delete-warning">
            这是一个每周重复的待办，删除后将永久消失，下周不会再出现。
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
          <button @click="closeParentCompleteConfirm" class="modal-close">×</button>
        </div>
        <div class="modal-body">
          <p class="delete-message">
            确定要完成这个父待办吗？
          </p>
          <p class="delete-todo-content">"{{ parentCompleteContent }}"</p>
          <p class="delete-warning">
            确认后，所有子待办都会被标记为完成。
          </p>
        </div>
        <div class="modal-footer">
          <button @click="closeParentCompleteConfirm" class="btn-cancel">取消</button>
          <button @click="confirmParentComplete" class="btn-confirm">确认完成</button>
        </div>
      </div>
    </div>

    <!-- Settings Modal -->
    <div v-if="showSettings" class="modal-overlay" @click.self="showSettings = false">
      <div class="modal-content modal-large">
        <div class="modal-header">
          <h2 class="modal-title">设置</h2>
          <button @click="showSettings = false" class="modal-close">×</button>
        </div>
        <div class="modal-body modal-body-scroll">
          <Settings
            @shortcuts-changed="handleShortcutsChanged"
            @wallpaper-changed="applyBackgroundWallpaper"
          />
        </div>
      </div>
    </div>

    <LongTermTodos v-if="showLongTermTodos" @close="showLongTermTodos = false" />
  </template>
</template>

<style>
:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f4f6f8;
  --bg-app: #eef2f3;
  --text-primary: #17212b;
  --text-secondary: #65717f;
  --border-color: #d8e0e7;
  --primary-color: #2563eb;
  --primary-hover: #1d4ed8;
  --accent-color: #f59e0b;
  --success-color: #16a34a;
  --shadow: 0 10px 30px rgba(23, 33, 43, 0.08);
  --app-wallpaper-image: none;
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
  font-family: "Microsoft YaHei UI", "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: var(--text-primary);
  overflow-y: auto;
  background: var(--bg-app);
}

::-webkit-scrollbar {
  width: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: transparent;
  border-radius: 999px;
}

body:hover::-webkit-scrollbar-thumb {
  background: rgba(101, 113, 127, 0.35);
}

#app {
  min-height: 100vh;
  position: relative;
  isolation: isolate;
  background: linear-gradient(rgba(238, 242, 243, 0.72), rgba(238, 242, 243, 0.72));
}

#app::before {
  content: '';
  position: fixed;
  inset: 0;
  z-index: -1;
  pointer-events: none;
  background-image: var(--app-wallpaper-image);
  background-size: cover;
  background-position: center;
  opacity: 0.55;
}

.app {
  max-width: 640px;
  margin: 0 auto;
  padding: 26px 18px;
}

.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  gap: 10px;
}

.header-content {
  flex: 1;
  min-width: 0;
}

.date-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-start;
  min-height: 80px;
  justify-content: center;
}

.date-main {
  display: flex;
  align-items: baseline;
  flex-wrap: nowrap;
  gap: 6px;
}

.lunar-info {
  display: flex;
  align-items: center;
  gap: 6px;
}

.lunar-zodiac {
  font-size: 14px;
  color: var(--primary-color);
  font-weight: 700;
}

.lunar-date {
  font-size: 14px;
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
}

.back-today-btn:hover {
  opacity: 0.9;
  transform: scale(1.05);
}

.back-today-btn:active {
  transform: scale(0.95);
}

.date-text {
  font-size: 24px;
  font-weight: 800;
  color: var(--text-primary);
  letter-spacing: 0;
  line-height: 1.1;
  white-space: nowrap;
}

.weekday-text {
  font-size: 14px;
  color: var(--text-secondary);
  font-weight: 700;
  white-space: nowrap;
}

.header-actions {
  display: grid;
  grid-template-columns: repeat(2, minmax(72px, 1fr));
  gap: 8px;
  width: 164px;
  flex: 0 0 164px;
}

.parent-tabs {
  position: sticky;
  top: 0;
  z-index: 80;
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 -4px 16px;
  padding: 10px 4px;
  overflow-x: auto;
  scrollbar-width: none;
  background: linear-gradient(180deg, rgba(238, 242, 243, 0.98), rgba(238, 242, 243, 0.88));
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
}

.parent-tabs::-webkit-scrollbar {
  display: none;
}

.parent-tab {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  max-width: 180px;
  height: 33px;
  padding: 0 11px;
  border: 1px solid rgba(216, 224, 231, 0.95);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.9);
  color: #25313d;
  font-family: "Microsoft YaHei UI", "Microsoft YaHei", "Segoe UI", sans-serif;
  box-shadow: 0 8px 18px rgba(23, 33, 43, 0.06);
  cursor: pointer;
  flex: 0 0 auto;
  transition: transform 0.18s ease, border-color 0.18s ease, box-shadow 0.18s ease, background 0.18s ease;
}

.parent-tab:hover,
.parent-tab.active {
  transform: translateY(-1px);
  border-color: rgba(37, 99, 235, 0.35);
  box-shadow: 0 10px 22px rgba(37, 99, 235, 0.12);
}

.parent-tab.completed {
  border-color: rgba(22, 163, 74, 0.34);
  background: linear-gradient(180deg, rgba(34, 197, 94, 0.92), rgba(22, 163, 74, 0.9));
  color: #ffffff;
  box-shadow: 0 10px 22px rgba(22, 163, 74, 0.18);
}

.parent-tab-status {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #f59e0b;
  box-shadow: 0 0 0 3px rgba(245, 158, 11, 0.14);
  flex: 0 0 auto;
}

.parent-tab.completed .parent-tab-status {
  background: #ffffff;
  box-shadow: 0 0 0 3px rgba(255, 255, 255, 0.22);
}

.parent-tab-title {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.15;
  letter-spacing: 0;
  transform: translateY(-0.5px);
}

.header-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  height: 36px;
  padding: 0 7px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.2s;
  white-space: nowrap;
}

.add-todo-btn {
  background: var(--primary-color);
  color: white;
  box-shadow: 0 8px 18px rgba(37, 99, 235, 0.22);
}

.add-todo-btn:hover {
  background: var(--primary-hover);
  transform: translateY(-1px);
}

.add-todo-btn:active {
  transform: translateY(0);
}

.add-text {
  font-size: 13px;
}

.btn-icon {
  width: 17px;
  height: 17px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
  flex-shrink: 0;
}

.settings-btn,
.date-picker-btn,
.long-term-btn {
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  box-shadow: 0 6px 18px rgba(23, 33, 43, 0.05);
}

.settings-btn:hover,
.date-picker-btn:hover,
.long-term-btn:hover {
  background: var(--bg-secondary);
  border-color: var(--primary-color);
  color: var(--primary-color);
}

.long-term-btn {
  font-size: 12px;
}

/* Date Picker Styles */
.date-picker-wrapper {
  position: relative;
}

.date-picker-wrapper .header-action-btn {
  height: 36px;
}

.date-picker-dropdown {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  z-index: 100;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  box-shadow: var(--shadow);
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
  padding: 14px 16px;
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
  transition: border-color 0.2s, box-shadow 0.2s;
  box-shadow: 0 8px 20px rgba(23, 33, 43, 0.04);
}

.memo-textarea:focus {
  outline: none;
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.12);
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
  padding-bottom: 128px;
}

.todo-item-wrapper.tab-scroll-flash > .todo-item {
  border-color: rgba(37, 99, 235, 0.55);
  box-shadow: 0 0 0 4px rgba(37, 99, 235, 0.12), 0 12px 26px rgba(37, 99, 235, 0.12);
}

.todo-item-wrapper[id^="todo-"] {
  scroll-margin-top: 68px;
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
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
  flex-shrink: 0;
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
  max-width: 430px;
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

.cycle-form-group {
  gap: 10px;
}

.cycle-mode-options {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.cycle-mode-option {
  height: 36px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: border-color 0.2s, background 0.2s, color 0.2s, box-shadow 0.2s;
}

.cycle-mode-option:hover {
  border-color: var(--primary-color);
  color: var(--primary-color);
}

.cycle-mode-option.active {
  border-color: var(--primary-color);
  background: var(--primary-color);
  color: #ffffff;
  box-shadow: 0 8px 18px rgba(37, 99, 235, 0.16);
}

.cycle-config-panel {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  padding: 12px;
  background: var(--bg-secondary);
  border: 1px solid rgba(79, 70, 229, 0.16);
  border-radius: 8px;
}

.cycle-config-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}

.cycle-config-field:first-child {
  grid-column: 1 / -1;
}

.cycle-config-panel .form-hint {
  grid-column: 1 / -1;
  margin-top: 0;
  line-height: 1.45;
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
  padding: 12px max(24px, calc((100vw - 640px) / 2 + 24px));
  box-shadow: 0 -10px 24px rgba(23, 33, 43, 0.06);
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
  background: linear-gradient(90deg, var(--primary-color), var(--success-color));
  border-radius: 4px;
  transition: width 0.3s ease;
}

/* Scroll to Top Button */
.scroll-to-top-btn {
  position: fixed;
  bottom: 88px;
  right: max(18px, calc((100vw - 640px) / 2 + 18px));
  width: 38px;
  height: 38px;
  border: 1px solid rgba(37, 99, 235, 0.18);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.92);
  color: var(--primary-color);
  cursor: pointer;
  box-shadow: 0 8px 22px rgba(23, 33, 43, 0.14);
  transition: all 0.3s ease;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
}

.scroll-to-top-btn:hover {
  transform: translateY(-2px);
  background: var(--primary-color);
  color: white;
  box-shadow: 0 10px 26px rgba(37, 99, 235, 0.22);
}

.scroll-to-top-btn:active {
  transform: translateY(0);
}
</style>
