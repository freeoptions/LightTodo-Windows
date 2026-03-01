<script setup lang="ts">
import { ref, computed } from 'vue';
import type { RepeatMode } from '../types/todo';
import { REPEAT_MODES } from '../types/todo';

const emit = defineEmits<{
  (e: 'add', content: string, repeatMode: RepeatMode): void;
}>();

const content = ref('');
const repeatMode = ref<RepeatMode>('daily');

const canSubmit = computed(() => content.value.trim().length > 0);

const handleSubmit = () => {
  if (!canSubmit.value) return;
  emit('add', content.value.trim(), repeatMode.value);
  content.value = '';
  repeatMode.value = 'daily';
};
</script>

<template>
  <form class="todo-input" @submit.prevent="handleSubmit">
    <input
      v-model="content"
      type="text"
      placeholder="添加新待办..."
      class="todo-input-field"
      @keydown.enter="handleSubmit"
    />
    <select v-model="repeatMode" class="todo-input-select">
      <option v-for="mode in REPEAT_MODES" :key="mode.value" :value="mode.value">
        {{ mode.label }}
      </option>
    </select>
    <button type="submit" class="todo-input-button" :disabled="!canSubmit">
      添加
    </button>
  </form>
</template>

<style scoped>
.todo-input {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
  padding: 16px;
  background: var(--bg-secondary);
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.todo-input-field {
  flex: 1;
  padding: 12px 16px;
  border: 2px solid var(--border-color);
  border-radius: 8px;
  font-size: 15px;
  background: var(--bg-primary);
  color: var(--text-primary);
  transition: border-color 0.2s;
}

.todo-input-field:focus {
  outline: none;
  border-color: var(--primary-color);
}

.todo-input-field::placeholder {
  color: var(--text-secondary);
}

.todo-input-select {
  padding: 12px 16px;
  border: 2px solid var(--border-color);
  border-radius: 8px;
  font-size: 15px;
  background: var(--bg-primary);
  color: var(--text-primary);
  cursor: pointer;
  transition: border-color 0.2s;
}

.todo-input-select:focus {
  outline: none;
  border-color: var(--primary-color);
}

.todo-input-button {
  padding: 12px 24px;
  border: none;
  border-radius: 8px;
  font-size: 15px;
  font-weight: 600;
  background: var(--primary-color);
  color: white;
  cursor: pointer;
  transition: opacity 0.2s, transform 0.1s;
}

.todo-input-button:hover:not(:disabled) {
  opacity: 0.9;
}

.todo-input-button:active:not(:disabled) {
  transform: scale(0.98);
}

.todo-input-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --text-primary: #1a1a1a;
  --text-secondary: #6c757d;
  --border-color: #dee2e6;
  --primary-color: #4f46e5;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #2a2a2a;
    --text-primary: #e9ecef;
    --text-secondary: #adb5bd;
    --border-color: #495057;
    --primary-color: #6366f1;
  }
}
</style>
