<script setup lang="ts">
import { ref, watch, computed } from 'vue';

const props = defineProps<{
  modelValue: string;
  action: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
}>();

// Parse the current shortcut
const parseShortcut = (shortcut: string) => {
  const parts = shortcut.toLowerCase().split('+');
  const key = parts.find(p =>
    !['ctrl', 'alt', 'shift', 'super', 'commandorcontrol'].includes(p)
  ) || '1';
  // Convert single digit to digit format for Tauri (e.g., "1" -> "digit1")
  const normalizedKey = /^\d$/.test(key) ? `digit${key}` : key;
  return {
    ctrl: parts.includes('ctrl') || parts.includes('commandorcontrol'),
    alt: parts.includes('alt'),
    shift: parts.includes('shift'),
    key: normalizedKey
  };
};

const ctrl = ref(false);
const alt = ref(false);
const shift = ref(false);
const selectedKey = ref('1');

// Initialize from current value
watch(() => props.modelValue, (newValue) => {
  const parsed = parseShortcut(newValue);
  ctrl.value = parsed.ctrl;
  alt.value = parsed.alt;
  shift.value = parsed.shift;
  selectedKey.value = parsed.key;
}, { immediate: true });

// Build shortcut string
const shortcutString = computed(() => {
  const parts: string[] = [];
  if (ctrl.value) parts.push('CommandOrControl');
  if (shift.value) parts.push('Shift');
  if (alt.value) parts.push('Alt');
  if (selectedKey.value) parts.push(selectedKey.value);
  return parts.join('+');
});

// Emit changes when any modifier or key changes
watch([ctrl, alt, shift, selectedKey], () => {
  emit('update:modelValue', shortcutString.value);
});
</script>

<template>
  <div class="shortcut-editor">
    <div class="modifiers">
      <button
        type="button"
        :class="{ active: ctrl }"
        @click="ctrl = !ctrl"
        class="mod-btn"
      >
        Ctrl
      </button>
      <button
        type="button"
        :class="{ active: shift }"
        @click="shift = !shift"
        class="mod-btn"
      >
        Shift
      </button>
      <button
        type="button"
        :class="{ active: alt }"
        @click="alt = !alt"
        class="mod-btn"
      >
        Alt
      </button>
    </div>

    <select v-model="selectedKey" class="key-select">
      <optgroup label="数字">
        <option v-for="i in 10" :key="i - 1" :value="`digit${i - 1}`">
          {{ i - 1 }}
        </option>
      </optgroup>
      <optgroup label="字母">
        <option v-for="letter in 26" :key="letter" :value="String.fromCharCode(97 + letter - 1)">
          {{ String.fromCharCode(65 + letter - 1) }}
        </option>
      </optgroup>
      <optgroup label="功能键">
        <option v-for="i in 12" :key="`f${i}`" :value="`f${i}`">
          F{{ i }}
        </option>
      </optgroup>
      <optgroup label="特殊键">
        <option value="space">空格</option>
        <option value="enter">回车</option>
        <option value="tab">Tab</option>
        <option value="escape">Esc</option>
        <option value="backspace">Backspace</option>
        <option value="delete">Delete</option>
        <option value="home">Home</option>
        <option value="end">End</option>
        <option value="pageup">Page Up</option>
        <option value="pagedown">Page Down</option>
        <option value="up">↑</option>
        <option value="down">↓</option>
        <option value="left">←</option>
        <option value="right">→</option>
      </optgroup>
    </select>
  </div>
</template>

<style scoped>
.shortcut-editor {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.modifiers {
  display: flex;
  gap: 6px;
}

.mod-btn {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 6px;
  background: var(--bg-secondary, #f8f9fa);
  color: var(--text-primary, #1a1a1a);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
}

.mod-btn:hover {
  border-color: var(--primary-color, #4f46e5);
  background: #eef2ff;
}

.mod-btn.active {
  background: var(--primary-color, #4f46e5);
  color: white;
  border-color: var(--primary-color, #4f46e5);
}

.key-select {
  padding: 8px 12px;
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 6px;
  background: var(--bg-secondary, #f8f9fa);
  color: var(--text-primary, #1a1a1a);
  font-size: 13px;
  cursor: pointer;
}

.key-select:focus {
  outline: none;
  border-color: var(--primary-color, #4f46e5);
}

@media (prefers-color-scheme: dark) {
  .mod-btn,
  .key-select {
    background: #2a2a2a;
    border-color: #495057;
  }

  .mod-btn:hover {
    background: #3a3a3a;
  }

  .mod-btn.active {
    background: #4f46e5;
    border-color: #4f46e5;
  }
}
</style>
