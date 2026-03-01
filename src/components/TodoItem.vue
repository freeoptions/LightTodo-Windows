<script setup lang="ts">
import type { TodoItem as TodoItemType } from '../types/todo';
import { computed } from 'vue';

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
  (e: 'complete-early', id: string): void;
  (e: 'toggle-disable', id: string): void;
}>();

// Check if this is a subtodo (has parentId)
const isSubtodo = computed(() => !!props.todo.parentId);

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
const canToggle = computed(() => props.isToday !== false);

// Check if this todo is disabled
const isDisabled = computed(() => props.todo.disabled === true);

// Can operate? (disabled todos cannot be operated on)
const canOperate = computed(() => !isDisabled.value);

// Check if this todo should show the complete early button
const showCompleteEarlyButton = computed(() => {
  if (props.todo.completed) return false;
  // Parent weekly todo
  if (!isSubtodo.value && props.todo.repeatMode === 'weekly') return true;
  // Subtodo with weekly parent
  if (isSubtodo.value && props.parentRepeatMode === 'weekly') return true;
  return false;
});

const weekdayNames = ['一', '二', '三', '四', '五', '六', '日'];

const repeatModeLabels: Record<TodoItemType['repeatMode'], string> = {
  daily: '每天重复',
  weekly: '每周重复',
  specific_dates: '指定日期',
  none: '不重复',
};

// Check if this todo has subtodos
const hasSubtodos = computed(() => props.todo.subtodos && props.todo.subtodos.length > 0);

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

// Handle checkbox click - emit toggle-parent for parent todos with subtodos
const handleCheckboxClick = (e: Event) => {
  // Prevent default checkbox behavior - we'll control state manually
  e.preventDefault();

  // Disabled todos cannot be toggled
  if (isDisabled.value) {
    return;
  }

  // Allow toggling in these cases:
  // 1. Always allow undoing (completed -> uncompleted)
  // 2. Allow completing on today's view
  // 3. For subtodos (isSubtodo), always allow toggling since they don't have repeat mode
  const canToggleNow = props.todo.completed || canToggle.value || isSubtodo.value;

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
</script>

<template>
  <div class="todo-item-wrapper">
    <div
      class="todo-item"
      :class="{
        completed: todo.completed,
        'is-subtodo': isSubtodo,
        'non-today': !canToggle,
        'is-disabled': isDisabled
      }"
    >
      <label class="todo-item-checkbox" :class="{ 'checkbox-disabled': !canToggle || isDisabled }">
        <input
          type="checkbox"
          :checked="todo.completed"
          @click="handleCheckboxClick"
          :disabled="!canToggle || isDisabled"
        />
        <span class="checkbox-icon">
          <span class="icon-unchecked">⃞</span>
          <span class="icon-checked">✅</span>
        </span>
      </label>
      <div class="todo-item-content">
        <span class="todo-item-text">{{ todo.content }}</span>
        <span v-if="!isSubtodo" class="todo-item-meta">
          <span class="todo-item-repeat">{{ repeatModeDisplay }}</span>
        </span>
      </div>
      <div class="todo-item-actions">
        <!-- Move up button -->
        <button
          v-if="canMoveUp"
          type="button"
          class="todo-item-move-up"
          @click.prevent="emit('move-up', todo.id)"
          aria-label="上移"
          title="上移"
        >
          ↑
        </button>

        <!-- Move down button -->
        <button
          v-if="canMoveDown"
          type="button"
          class="todo-item-move-down"
          @click.prevent="emit('move-down', todo.id)"
          aria-label="下移"
          title="下移"
        >
          ↓
        </button>

        <!-- Disable/Enable toggle switch -->
        <button
          type="button"
          class="toggle-switch"
          :class="{ 'active': !isDisabled }"
          @click.prevent="emit('toggle-disable', todo.id)"
          :aria-label="isDisabled ? '启用' : '禁用'"
          :title="isDisabled ? '启用（恢复操作）' : '禁用（标记完成并禁止操作）'"
        >
          <span class="toggle-slider"></span>
        </button>

        <!-- Complete early button for weekly todos and their subtodos (always visible) -->
        <button
          v-if="showCompleteEarlyButton"
          type="button"
          class="todo-item-complete-early"
          @click.prevent="emit('complete-early', todo.id)"
          aria-label="提前完成"
          title="提前完成"
        >
          ⏩
        </button>

        <!-- Expand/collapse button for parent todos -->
        <button
          v-if="hasSubtodos"
          class="todo-item-expand"
          @click="emit('toggle-expand', todo.id)"
          :title="todo.expanded ? '收起' : '展开'"
        >
          {{ todo.expanded ? '🔽' : '🔼' }}
        </button>

        <!-- Add subtodo button for parent todos -->
        <button
          v-if="!isSubtodo && !todo.completed && canOperate"
          class="todo-item-add-sub"
          @click="emit('add-subtodo', todo.id)"
          title="添加子待办"
        >
          ➕
        </button>

        <button
          v-if="!todo.completed && canOperate"
          class="todo-item-edit"
          @click="emit('edit', todo.id)"
          aria-label="编辑"
          title="编辑"
        >
          ✏️
        </button>
        <button
          v-if="canOperate"
          class="todo-item-delete"
          @click="emit('delete', todo.id)"
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
        v-for="(subtodo, index) in todo.subtodos"
        :key="subtodo.id"
        :todo="subtodo"
        :subtodo-index="index"
        :subtodo-total="todo.subtodos?.length"
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
        @complete-early="(id) => emit('complete-early', id)"
        @toggle-disable="(id) => emit('toggle-disable', id)"
      />
    </div>
  </div>
</template>

<style scoped>
.todo-item-wrapper {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
  position: absolute;
  top: 4px;
  right: 4px;
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

.todo-item-checkbox {
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
}

.todo-item-checkbox input[type="checkbox"] {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
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
}

.todo-item-repeat {
  font-size: 12px;
  padding: 2px 8px;
  background: var(--bg-secondary);
  border-radius: 4px;
  color: var(--text-secondary);
}

.todo-item-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
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
.todo-item-expand,
.todo-item-add-sub,
.todo-item-complete-early,
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

.todo-item-move-up:hover,
.todo-item-move-down:hover {
  background: var(--bg-secondary);
  color: var(--primary-color);
}

.todo-item-expand {
  font-size: 16px;
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

.todo-item-expand:hover {
  background: var(--bg-secondary);
  color: var(--text-primary);
}

.todo-item-add-sub:hover {
  background: var(--bg-secondary);
  color: var(--primary-color);
}

.todo-item-complete-early {
  font-size: 14px;
}

.todo-item-complete-early:hover {
  background: var(--bg-secondary);
  color: #22c55e;
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
