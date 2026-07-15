<script setup lang="ts">
import type { TodoItem as TodoItemType } from '../types/todo';
import { useSettings } from '../composables/useSettings';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import ArrowIcon from './ArrowIcon.vue';

const props = defineProps<{
  todo: TodoItemType;
  subtodoIndex?: number;
  subtodoTotal?: number;
  parentIndex?: number;
  parentTotal?: number;
  isToday?: boolean;
  parentRepeatMode?: TodoItemType['repeatMode'];
}>();

const emit = defineEmits<{
  (e: 'toggle', id: string): void;
  (e: 'delete', id: string): void;
  (e: 'edit', id: string): void;
  (e: 'toggle-expand', id: string): void;
  (e: 'add-subtodo', parentId: string): void;
  (e: 'toggle-parent', id: string): void;
  (e: 'move-up', id: string): void;
  (e: 'move-down', id: string): void;
  (e: 'toggle-disable', id: string): void;
}>();
const { settings } = useSettings();
const defaultPriorityColors = {
  1: '#ef4444',
  2: '#f59e0b',
  3: '#22c55e'
};

const getPriorityColor = (priority: number) => {
  return settings.value?.priorityColors?.[priority as keyof typeof settings.value.priorityColors] || defaultPriorityColors[priority as keyof typeof defaultPriorityColors];
};

// Check if this is a subtodo (has parentId)
const isSubtodo = computed(() => !!props.todo.parentId);

// Check if this is a weekly_this_week todo
const isWeeklyThisWeek = computed(() => props.todo.repeatMode === 'weekly_this_week' && !props.todo.parentId);

// Check if this is a monthly_this_month todo
const isMonthlyThisMonth = computed(() => props.todo.repeatMode === 'monthly_this_month' && !props.todo.parentId);

// Can move up? (subtodo and not first, or parent and not first)
const canMoveUp = computed(() => {
  if (isSubtodo.value) {
    return props.subtodoIndex !== undefined && props.subtodoIndex > 0;
  }
  return props.parentIndex !== undefined && props.parentIndex > 0;
});

// Can move down? (subtodo and not last, or parent and not last)
const canMoveDown = computed(() => {
  if (isSubtodo.value) {
    return props.subtodoIndex !== undefined && props.subtodoTotal !== undefined && props.subtodoIndex < props.subtodoTotal - 1;
  }
  return props.parentIndex !== undefined && props.parentTotal !== undefined && props.parentIndex < props.parentTotal - 1;
});

// Can toggle completion? (only if it's today)
const canToggle = computed(() => {
  return props.isToday === true;
});

// Check if this todo is disabled
const isInactiveByWeekday = computed(() => props.todo.inactiveByWeekday === true);
const isDisabled = computed(() => props.todo.disabled === true || isInactiveByWeekday.value);

// Can operate? (disabled todos cannot be operated on)
const canOperate = computed(() => !isDisabled.value);

// Can edit? (not completed and can operate)
const canEdit = computed(() => !props.todo.completed && canOperate.value);
const statusBadgeText = computed(() => {
  if (isInactiveByWeekday.value) return '本日未启用';
  if (props.todo.disabled) return '已禁用';
  return '';
});

// Should disable checkbox? (same logic as handleCheckboxClick)
const shouldDisableCheckbox = computed(() => {
  if (isDisabled.value) return true;
  // Always allow undoing
  if (props.todo.completed && !isInactiveByWeekday.value) return false;
  // Allow on today's view
  if (canToggle.value) return false;
  // For subtodos, check parent's repeat mode
  if (isSubtodo.value) {
    // If parent is daily/weekly repeating, disable on non-today view
    if (props.parentRepeatMode === 'daily' || props.parentRepeatMode === 'weekly') {
      return true;
    }
    // Parent is non-repeating, allow
    return false;
  }
  // Parent todo on non-today view
  return true;
});

const weekdayNames = ['一', '二', '三', '四', '五', '六', '日'];

const repeatModeLabels: Record<TodoItemType['repeatMode'], string> = {
  daily: '每天重复',
  weekly: '每周重复',
  weekly_this_week: '本周内',
  monthly_this_month: '本月内',
  specific_dates: '指定日期',
  none: '不重复',
};

const allSubtodos = computed(() => props.todo.subtodos || []);
const COMPLETE_SINK_DELAY_MS = 3000;
const sortTick = ref(Date.now());
const disabledSubtodosExpanded = ref(false);
let pendingSortTimer: ReturnType<typeof setTimeout> | null = null;

const normalizeTimestampMs = (timestamp?: number) => {
  if (!timestamp) return null;
  return timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp;
};

const clearPendingSortTimer = () => {
  if (pendingSortTimer) {
    clearTimeout(pendingSortTimer);
    pendingSortTimer = null;
  }
};

const isPendingSinkByCompletedAt = (todo: TodoItemType) => {
  if (!todo.completed) return false;
  const completedAtMs = normalizeTimestampMs(todo.completedAt);
  if (!completedAtMs) return false;
  return sortTick.value - completedAtMs < COMPLETE_SINK_DELAY_MS;
};

const scheduleSubtodoSortRefresh = () => {
  clearPendingSortTimer();
  const now = Date.now();
  const pendingTodos = allSubtodos.value
    .map((t) => ({ todo: t, completedAtMs: normalizeTimestampMs(t.completedAt) }))
    .filter(({ todo, completedAtMs }) => todo.completed && completedAtMs && now - completedAtMs < COMPLETE_SINK_DELAY_MS);
  if (pendingTodos.length === 0) return;
  const nextExpireMs = Math.min(...pendingTodos.map(({ completedAtMs }) => COMPLETE_SINK_DELAY_MS - (now - (completedAtMs || 0))));
  pendingSortTimer = setTimeout(() => {
    sortTick.value = Date.now();
    pendingSortTimer = null;
    scheduleSubtodoSortRefresh();
  }, Math.max(0, nextExpireMs));
};

watch(
  allSubtodos,
  () => {
    sortTick.value = Date.now();
    scheduleSubtodoSortRefresh();
  },
  { immediate: true, deep: true }
);

onBeforeUnmount(() => {
  clearPendingSortTimer();
});

watch(
  () => props.todo.id,
  () => {
    disabledSubtodosExpanded.value = false;
  },
  { immediate: true }
);

const activeSubtodos = computed(() => {
  return [...allSubtodos.value].sort((a, b) => {
    if (a.disabled !== b.disabled) return a.disabled ? 1 : -1;
    const aPending = isPendingSinkByCompletedAt(a);
    const bPending = isPendingSinkByCompletedAt(b);
    const aEffectiveCompleted = a.completed && !aPending;
    const bEffectiveCompleted = b.completed && !bPending;
    if (aEffectiveCompleted !== bEffectiveCompleted) return aEffectiveCompleted ? 1 : -1;
    return 0;
  });
});

// Check if this todo has visible subtodos
const hasSubtodos = computed(() => activeSubtodos.value.length > 0);
const isSubtodoDisabled = (todo: TodoItemType) => todo.disabled === true || todo.inactiveByWeekday === true;
const enabledSubtodos = computed(() => activeSubtodos.value.filter((todo) => !isSubtodoDisabled(todo)));
const disabledSubtodos = computed(() => activeSubtodos.value.filter((todo) => isSubtodoDisabled(todo)));

// Format weekly display with selected weekdays
const weeklyDisplayText = computed(() => {
  if (!props.todo.weekdays) return '每周重复';

  const days = props.todo.weekdays.split(',')
    .map(d => {
      const dayNum = parseInt(d.trim());
      return weekdayNames[dayNum - 1] || '';
    })
    .filter(d => d)
    .join('、');

  return days ? `每周【${days}】` : '每周重复';
});

// Format specific dates display
const specificDatesDisplay = computed(() => {
  if (!props.todo.specificDates) return '指定日期';

  const dates = props.todo.specificDates.split(',')
    .map(d => {
      const date = new Date(d.trim() + 'T00:00:00');
      return `${date.getMonth() + 1}/${date.getDate()}`;
    })
    .filter(d => d)
    .join('、');

  return dates ? `指定日期【${dates}】` : '指定日期';
});

const repeatModeDisplay = computed(() => {
  if (props.todo.repeatMode === 'weekly') {
    return weeklyDisplayText.value;
  }
  if (props.todo.repeatMode === 'specific_dates') {
    return specificDatesDisplay.value;
  }
  return repeatModeLabels[props.todo.repeatMode];
});

// Format expiry date display
const expiryDateDisplay = computed(() => {
  if (!props.todo.expiryDate) return '';
  const date = new Date(props.todo.expiryDate + 'T00:00:00');
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
});

// Check if expiry date is in the past (expired)
const isExpired = computed(() => {
  if (!props.todo.expiryDate) return false;
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const expiryDate = new Date(props.todo.expiryDate + 'T00:00:00');
  return expiryDate < today;
});

// Handle checkbox click - emit toggle-parent for parent todos with subtodos
const handleCheckboxClick = (e: Event) => {
  // Prevent default checkbox behavior - we'll control state manually
  e.preventDefault();
  e.stopPropagation();

  // Disabled todos cannot be toggled
  if (isDisabled.value) {
    return;
  }

  // Allow toggling in these cases:
  // 1. Always allow undoing (completed -> uncompleted)
  // 2. Allow completing on today's view
  // 3. For subtodos, check parent's repeat mode:
  //    - If parent is daily/weekly repeating, only allow on today's view
  //    - If parent is non-repeating, allow anytime
  let canToggleNow = props.todo.completed || canToggle.value;

  if (isSubtodo.value && !canToggleNow) {
    // Subtodo: check if parent is repeating
    if (props.parentRepeatMode === 'daily' || props.parentRepeatMode === 'weekly') {
      // Parent is repeating, only allow on today's view
      canToggleNow = false;
    } else {
      // Parent is non-repeating, allow toggling
      canToggleNow = true;
    }
  }

  if (!canToggleNow) {
    return;
  }

  if (hasSubtodos.value && !props.todo.completed) {
    // Parent todo with subtodos - emit toggle-parent for confirmation
    emit('toggle-parent', props.todo.id);
  } else {
    // Regular toggle (allows undoing completed todos anytime, and subtodos anytime)
    emit('toggle', props.todo.id);
  }
};

// Handle clicking on the todo item (not on buttons/checkbox)
const handleTodoItemClick = (e: Event) => {
  // Only handle for parent todos with subtodos
  if (!isSubtodo.value && hasSubtodos.value) {
    // Check if the click target is a button or checkbox
    const target = e.target as HTMLElement;
    if (target.closest('button') || target.closest('.todo-item-checkbox') || target.closest('.toggle-switch')) {
      return; // Don't toggle if clicking on buttons/checkbox
    }
    // Toggle expand/collapse
    emit('toggle-expand', props.todo.id);
  }
};
</script>

<template>
  <div class="todo-item-wrapper" :id="!isSubtodo ? `todo-${todo.id}` : undefined" :data-priority="todo.priority" :class="{ 'has-priority': todo.priority, 'weekly-this-week': isWeeklyThisWeek, 'monthly-this-month': isMonthlyThisMonth }">
    <div
      class="todo-item"
      :class="{
        completed: todo.completed,
        'is-subtodo': isSubtodo,
        'non-today': !canToggle,
        'is-disabled': isDisabled,
        'has-subtodos': hasSubtodos
      }"
      @click="handleTodoItemClick"
    >
      <label v-if="!isDisabled" class="todo-item-checkbox" :class="{ 'checkbox-disabled': shouldDisableCheckbox }">
        <input
          type="checkbox"
          :checked="todo.completed"
          @click.prevent.stop="handleCheckboxClick"
        />
        <span class="checkbox-icon" @click.prevent.stop="handleCheckboxClick">
          <span class="icon-unchecked">⃞</span>
          <span class="icon-checked">✅</span>
        </span>
      </label>
      <span v-if="statusBadgeText" class="todo-item-status-badge">{{ statusBadgeText }}</span>
      <!-- Expiry date icon (for both parent todos and subtodos) - separate column -->
      <span v-if="todo.expiryDate" class="todo-item-expiry-icon-wrapper">
        <span class="todo-item-expiry-icon" :class="{ 'is-expired': isExpired }">📅</span>
        <span class="expiry-tooltip">
          截止日期：{{ expiryDateDisplay }}<span v-if="isExpired">（已过期）</span>
        </span>
      </span>
      <div class="todo-item-content">
        <span class="todo-item-text" :style="todo.priority && !isSubtodo ? { color: getPriorityColor(todo.priority) } : undefined">{{ todo.content }}</span>
        <span v-if="!isSubtodo" class="todo-item-meta">
          <span v-if="todo.priority" class="priority-mark" :style="{ background: getPriorityColor(todo.priority) }">
            P{{ todo.priority }}
          </span>
          <span class="todo-item-repeat">{{ repeatModeDisplay }}</span>
          <span v-if="hasSubtodos" class="todo-item-subtodo-count">
            {{ activeSubtodos.length }} 个子待办
          </span>
        </span>
      </div>
      <div class="todo-item-actions">
        <!-- Move up button -->
        <button
          v-if="canMoveUp"
          type="button"
          class="todo-item-move-up"
          @click.prevent.stop="emit('move-up', todo.id)"
          aria-label="上移"
          title="上移"
        >
          <ArrowIcon direction="up" />
        </button>

        <!-- Move down button -->
        <button
          v-if="canMoveDown"
          type="button"
          class="todo-item-move-down"
          @click.prevent.stop="emit('move-down', todo.id)"
          aria-label="下移"
          title="下移"
        >
          <ArrowIcon direction="down" />
        </button>

        <!-- Disable/Enable toggle switch -->
        <button
          type="button"
          class="toggle-switch"
          :class="{ 'active': !isDisabled }"
          @click.prevent.stop="emit('toggle-disable', todo.id)"
          :aria-label="isDisabled ? '启用' : '禁用'"
          :title="isDisabled ? '启用（恢复操作）' : '禁用（标记完成并禁止操作）'"
        >
          <span class="toggle-slider"></span>
        </button>

        <!-- Add subtodo button for parent todos -->
        <button
          v-if="!isSubtodo && !todo.completed && canOperate"
          class="todo-item-add-sub"
          @click.prevent.stop="emit('add-subtodo', todo.id)"
          title="添加子待办"
        >
          ➕
        </button>

        <button
          v-if="canEdit"
          class="todo-item-edit"
          @click.prevent.stop="emit('edit', todo.id)"
          aria-label="编辑"
          title="编辑"
        >
          ✏️
        </button>
        <button
          v-if="canOperate"
          class="todo-item-delete"
          @click.prevent.stop="emit('delete', todo.id)"
          aria-label="删除"
          title="删除"
        >
          ❎
        </button>
      </div>
    </div>

    <!-- Recursive subtodos rendering -->
    <div v-if="hasSubtodos && todo.expanded" class="subtodos-container">
      <TodoItem
        v-for="(subtodo, index) in enabledSubtodos"
        :key="subtodo.id"
        :todo="subtodo"
        :subtodo-index="index"
        :subtodo-total="enabledSubtodos.length"
        :parent-index="parentIndex"
        :parent-total="parentTotal"
        :is-today="isToday"
        :parent-repeat-mode="todo.repeatMode"
        @toggle="(id) => emit('toggle', id)"
        @delete="(id) => emit('delete', id)"
        @edit="(id) => emit('edit', id)"
        @toggle-expand="(id) => emit('toggle-expand', id)"
        @add-subtodo="(parentId) => emit('add-subtodo', parentId)"
        @toggle-parent="(id) => emit('toggle-parent', id)"
        @move-up="(id) => emit('move-up', id)"
        @move-down="(id) => emit('move-down', id)"
        @toggle-disable="(id) => emit('toggle-disable', id)"
      />

      <div
        v-if="disabledSubtodos.length > 0"
        class="disabled-subtodos-group"
      >
        <button
          type="button"
          class="disabled-subtodos-toggle"
          :class="{ expanded: disabledSubtodosExpanded }"
          @click.prevent.stop="disabledSubtodosExpanded = !disabledSubtodosExpanded"
        >
          <span class="disabled-subtodos-toggle-left">
            <span class="disabled-subtodos-badge">已禁用</span>
            <span class="disabled-subtodos-title">
              {{ disabledSubtodosExpanded ? '收起禁用子待办' : `展开禁用子待办（${disabledSubtodos.length}项）` }}
            </span>
          </span>
          <span class="disabled-subtodos-arrow">{{ disabledSubtodosExpanded ? '▾' : '▸' }}</span>
        </button>

        <div v-if="disabledSubtodosExpanded" class="disabled-subtodos-list">
          <TodoItem
            v-for="(subtodo, index) in disabledSubtodos"
            :key="subtodo.id"
            :todo="subtodo"
            :subtodo-index="enabledSubtodos.length + index"
            :subtodo-total="activeSubtodos.length"
            :parent-index="parentIndex"
            :parent-total="parentTotal"
            :is-today="isToday"
            :parent-repeat-mode="todo.repeatMode"
            @toggle="(id) => emit('toggle', id)"
            @delete="(id) => emit('delete', id)"
            @edit="(id) => emit('edit', id)"
            @toggle-expand="(id) => emit('toggle-expand', id)"
            @add-subtodo="(parentId) => emit('add-subtodo', parentId)"
            @toggle-parent="(id) => emit('toggle-parent', id)"
            @move-up="(id) => emit('move-up', id)"
            @move-down="(id) => emit('move-down', id)"
            @toggle-disable="(id) => emit('toggle-disable', id)"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.todo-item-wrapper {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.todo-item-wrapper.weekly-this-week {
  border-left: 4px solid #9333ea;
  padding-left: 8px;
  margin-left: 0;
}

.todo-item-wrapper.monthly-this-month {
  border-left: 4px solid #ec4899;
  padding-left: 8px;
  margin-left: 0;
}

.priority-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 2px 8px;
  border-radius: 4px;
  color: white;
  font-size: 12px;
  font-weight: 500;
  flex-shrink: 0;
  white-space: nowrap;
}

.todo-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  transition: all 0.2s;
}

.todo-item.is-subtodo {
  margin-left: 24px;
  padding: 12px 16px;
  background: var(--bg-secondary);
}

.todo-item:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.todo-item.completed {
  opacity: 0.7;
}

.todo-item.completed .todo-item-text {
  text-decoration: line-through;
  color: var(--text-secondary);
}

.todo-item.is-disabled {
  opacity: 1;
  background: repeating-linear-gradient(
    45deg,
    rgba(251, 146, 60, 0.08),
    rgba(251, 146, 60, 0.08) 10px,
    rgba(251, 146, 60, 0.15) 10px,
    rgba(251, 146, 60, 0.15) 20px
  );
  border: 2px dashed #fb923c;
  position: relative;
}

.todo-item.is-disabled::before {
  content: '已禁用';
  position: static;
  order: -1;
  flex-shrink: 0;
  font-size: 10px;
  padding: 2px 6px;
  background: #fb923c;
  color: white;
  border-radius: 4px;
  font-weight: 500;
}

.todo-item.is-disabled .todo-item-text {
  color: #c2410c;
}

/* Override the legacy disabled pseudo-badge so it no longer occupies layout space. */
.todo-item.is-disabled::before {
  content: none !important;
  display: none !important;
}

.todo-item-status-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  background: #fb923c;
  color: #ffffff;
  white-space: nowrap;
}

.todo-item.has-subtodos {
  cursor: pointer;
}

.todo-item.has-subtodos:hover {
  background: var(--bg-secondary);
}

.todo-item-subtodo-count {
  font-size: 12px;
  color: #6366f1;
  font-weight: 600;
  white-space: nowrap;
}

.todo-item-expiry-icon-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex-shrink: 0;
  margin-right: 4px;
  margin-left: 4px;
}

.todo-item-expiry-icon {
  font-size: 14px;
  cursor: help;
  opacity: 0.7;
  transition: opacity 0.2s;
}

.todo-item-expiry-icon:hover {
  opacity: 1;
}

.todo-item-expiry-icon.is-expired {
  color: #ef4444;
  opacity: 0.8;
}

.todo-item-expiry-icon.is-expired:hover {
  opacity: 1;
}

/* Custom tooltip - shows immediately on hover */
.expiry-tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.9);
  color: white;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 14px;
  white-space: nowrap;
  pointer-events: none;
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.15s, visibility 0.15s;
  z-index: 1000;
}

.todo-item-expiry-icon-wrapper:hover .expiry-tooltip {
  opacity: 1;
  visibility: visible;
}

.todo-item-repeat {
  font-size: 12px;
  padding: 2px 8px;
  background: rgba(79, 70, 229, 0.08);
  border-radius: 4px;
  color: var(--text-secondary);
  font-weight: 500;
  white-space: nowrap;
}

.todo-item-checkbox {
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  z-index: 100;
}

.todo-item-checkbox input[type="checkbox"] {
  position: absolute;
  opacity: 0;
  width: 100%;
  height: 100%;
  cursor: pointer;
}

.checkbox-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  font-size: 20px;
  line-height: 1;
  transition: all 0.2s;
  position: relative;
  z-index: 101;
}

.icon-unchecked {
  display: block;
  color: var(--border-color);
  font-size: 26px;
}

.icon-checked {
  display: none;
  color: #22c55e;
}

.todo-item-checkbox input:checked ~ .checkbox-icon .icon-unchecked {
  display: none;
}

.todo-item-checkbox input:checked ~ .checkbox-icon .icon-checked {
  display: block;
}

/* Non-today checkbox styles */
.checkbox-disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.todo-item.non-today .checkbox-icon {
  opacity: 0.4;
}

.todo-item.non-today:hover .checkbox-icon {
  opacity: 0.4;
}

.todo-item-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  align-items: flex-start;
}

.todo-item-text {
  font-size: 15px;
  color: var(--text-primary);
  word-break: break-word;
  white-space: pre-wrap;
}

.todo-item-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  min-width: 0;
}

.todo-item-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  max-width: none;
  flex-wrap: nowrap;
}

/* 默认隐藏所有操作按钮 */
.todo-item-actions button {
  opacity: 0;
  transition: opacity 0.2s ease;
}

/* 鼠标悬停在待办项上时显示按钮 */
.todo-item:hover .todo-item-actions button {
  opacity: 1;
}

.todo-item-move-up,
.todo-item-move-down,
.todo-item-edit,
.todo-item-delete,
.todo-item-add-sub,
.todo-item-disable {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.2s;
  flex-shrink: 0;
}

.todo-item-move-up,
.todo-item-move-down,
.todo-item-edit,
.todo-item-delete,
.todo-item-add-sub {
  opacity: 0;
}

.todo-item-move-up:hover,
.todo-item-move-down:hover {
  background: var(--bg-secondary);
  color: var(--primary-color);
}

.todo-item-move-up,
.todo-item-move-down {
  font-size: 17px;
}

.todo-item-add-sub {
  font-size: 14px;
}

.todo-item-edit:hover {
  background: var(--bg-secondary);
  color: var(--primary-color);
}

.todo-item-delete:hover {
  background: var(--danger-bg);
  color: var(--danger-color);
}

.todo-item-add-sub:hover {
  background: var(--bg-secondary);
  color: var(--primary-color);
}
/* 安卓风格开关 */
.toggle-switch {
  position: relative;
  width: 36px;
  height: 20px;
  padding: 0;
  border: none;
  border-radius: 10px;
  background: #cbd5e1;
  cursor: pointer;
  transition: background 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  flex-shrink: 0;
}

.toggle-switch .toggle-slider {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #ffffff;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.toggle-switch.active {
  background: #22c55e;
}

.toggle-switch.active .toggle-slider {
  transform: translateX(16px);
}

.toggle-switch:hover {
  background: #94a3b8;
}

.toggle-switch.active:hover {
  background: #16a34a;
}

.subtodos-container {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.disabled-subtodos-group {
  margin-left: 24px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.disabled-subtodos-toggle {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border: 1px dashed #fb923c;
  border-radius: 8px;
  background: rgba(251, 146, 60, 0.08);
  color: #c2410c;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
}

.disabled-subtodos-toggle:hover {
  background: rgba(251, 146, 60, 0.14);
}

.disabled-subtodos-toggle-left {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.disabled-subtodos-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  background: #fb923c;
  color: #ffffff;
  white-space: nowrap;
}

.disabled-subtodos-title {
  font-size: 13px;
  font-weight: 600;
  color: #c2410c;
  min-width: 0;
}

.disabled-subtodos-arrow {
  flex-shrink: 0;
  font-size: 14px;
  color: #c2410c;
}

.disabled-subtodos-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

@media (max-width: 520px) {
  .todo-item {
    gap: 8px;
    padding: 13px 14px;
  }

  .todo-item.is-subtodo {
    margin-left: 20px;
    padding: 11px 14px;
  }

  .disabled-subtodos-group {
    margin-left: 20px;
  }

  .todo-item-content {
    min-width: 0;
  }

  .todo-item-actions {
    gap: 4px;
    max-width: none;
    flex-wrap: nowrap;
  }

  .todo-item-move-up,
  .todo-item-move-down,
  .todo-item-edit,
  .todo-item-delete,
  .todo-item-add-sub,
  .todo-item-disable {
    width: 18px;
    height: 18px;
    font-size: 14px;
  }

  .todo-item-repeat,
  .todo-item-subtodo-count,
  .priority-mark {
    font-size: 11px;
    padding: 2px 6px;
  }
}

:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f1f3f5;
  --text-primary: #1a1a1a;
  --text-secondary: #6c757d;
  --border-color: #dee2e6;
  --danger-color: #ef4444;
  --danger-bg: #fee2e2;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #2a2a2a;
    --text-primary: #e9ecef;
    --text-secondary: #adb5bd;
    --border-color: #495057;
    --danger-color: #f87171;
    --danger-bg: #7f1d1d;
  }
}

</style>
