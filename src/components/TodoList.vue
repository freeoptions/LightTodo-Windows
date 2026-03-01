<script setup lang="ts">
import { computed } from 'vue';
import type { TodoItem as TodoItemType } from '../types/todo';
import TodoItem from './TodoItem.vue';

const props = defineProps<{
  todos: TodoItemType[];
  loading?: boolean;
  isToday?: boolean;
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

const incompleteTodos = computed(() => props.todos.filter((t) => !t.completed));
const completedTodos = computed(() => props.todos.filter((t) => t.completed));
</script>

<template>
  <div class="todo-list">
    <div v-if="loading" class="todo-list-loading">
      <span class="loading-spinner"></span>
      <span>加载中...</span>
    </div>

    <template v-else>
      <!-- Empty state (only when truly no todos at all) -->
      <div v-if="todos.length === 0" class="todo-list-empty">
        <div class="empty-icon">📝</div>
        <p>还没有待办事项</p>
        <p class="empty-hint">添加你的第一个待办吧！</p>
      </div>

      <!-- Incomplete todos section (always shown when there are any todos) -->
      <div v-if="todos.length > 0" class="todo-list-section">
        <h3 class="todo-list-section-title">未完成</h3>
        <div v-if="incompleteTodos.length === 0" class="todo-list-section-empty">
          暂无未完成的待办
        </div>
        <div v-else class="todo-list-items">
          <TodoItem
            v-for="(todo, index) in incompleteTodos"
            :key="todo.id"
            :todo="todo"
            :parent-index="index"
            :parent-total="incompleteTodos.length"
            :is-today="isToday"
            @toggle="emit('toggle', $event)"
            @delete="emit('delete', $event)"
            @edit="emit('edit', $event)"
            @toggle-expand="emit('toggle-expand', $event)"
            @add-subtodo="emit('add-subtodo', $event)"
            @toggle-parent="emit('toggle-parent', $event)"
            @move-up="emit('move-up', $event)"
            @move-down="emit('move-down', $event)"
            @complete-early="emit('complete-early', $event)"
            @toggle-disable="emit('toggle-disable', $event)"
          />
        </div>
      </div>

      <!-- Completed todos section (always shown when there are any todos) -->
      <div v-if="todos.length > 0" class="todo-list-section completed">
        <h3 class="todo-list-section-title">已完成</h3>
        <div v-if="completedTodos.length === 0" class="todo-list-section-empty">
          暂无已完成的待办
        </div>
        <div v-else class="todo-list-items">
          <TodoItem
            v-for="(todo, index) in completedTodos"
            :key="todo.id"
            :todo="todo"
            :parent-index="incompleteTodos.length + index"
            :parent-total="todos.length"
            :is-today="isToday"
            @toggle="emit('toggle', $event)"
            @delete="emit('delete', $event)"
            @edit="emit('edit', $event)"
            @toggle-expand="emit('toggle-expand', $event)"
            @add-subtodo="emit('add-subtodo', $event)"
            @toggle-parent="emit('toggle-parent', $event)"
            @move-up="emit('move-up', $event)"
            @move-down="emit('move-down', $event)"
            @complete-early="emit('complete-early', $event)"
            @toggle-disable="emit('toggle-disable', $event)"
          />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.todo-list {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.todo-list-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 48px;
  color: var(--text-secondary);
}

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-color);
  border-top-color: var(--primary-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.todo-list-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 64px 32px;
  text-align: center;
  color: var(--text-secondary);
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
  opacity: 0.8;
}

.todo-list-empty p {
  margin: 4px 0;
  font-size: 16px;
}

.empty-hint {
  font-size: 14px;
  opacity: 0.8;
}

.todo-list-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.todo-list-section.completed {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid var(--border-color);
}

.todo-list-section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
  padding-left: 4px;
}

.todo-list-items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.todo-list-section-empty {
  padding: 24px 16px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 14px;
  opacity: 0.7;
}

:root {
  --text-secondary: #6c757d;
  --border-color: #dee2e6;
  --primary-color: #4f46e5;
}

@media (prefers-color-scheme: dark) {
  :root {
    --text-secondary: #adb5bd;
    --border-color: #495057;
    --primary-color: #6366f1;
  }
}
</style>
