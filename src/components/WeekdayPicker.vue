<script setup lang="ts">
import { computed } from 'vue';

const props = withDefaults(defineProps<{
  modelValue?: number[];
  compact?: boolean;
}>(), {
  modelValue: () => [],
  compact: false,
});

const emit = defineEmits<{
  (event: 'update:modelValue', value: number[]): void;
}>();

const weekdayOptions = [
  { value: 1, label: '周一' },
  { value: 2, label: '周二' },
  { value: 3, label: '周三' },
  { value: 4, label: '周四' },
  { value: 5, label: '周五' },
  { value: 6, label: '周六' },
  { value: 7, label: '周日' },
];

const selectedWeekdays = computed(() => new Set(props.modelValue));

const selectedLabels = computed(() => weekdayOptions
  .filter((day) => selectedWeekdays.value.has(day.value))
  .map((day) => day.label)
  .join('、'));

const toggleWeekday = (weekday: number) => {
  const next = new Set(selectedWeekdays.value);
  if (next.has(weekday)) {
    next.delete(weekday);
  } else {
    next.add(weekday);
  }
  emit('update:modelValue', [...next].sort((a, b) => a - b));
};

const showEveryday = () => {
  emit('update:modelValue', []);
};
</script>

<template>
  <div class="weekday-picker" :class="{ compact }">
    <div class="weekday-picker-header">
      <span class="weekday-picker-label">展示星期</span>
      <button
        type="button"
        class="weekday-picker-reset"
        :class="{ active: modelValue.length === 0 }"
        @click="showEveryday"
      >
        每天
      </button>
    </div>
    <div class="weekday-picker-options" role="group" aria-label="子待办展示星期">
      <button
        v-for="day in weekdayOptions"
        :key="day.value"
        type="button"
        class="weekday-picker-option"
        :class="{ selected: selectedWeekdays.has(day.value) }"
        :aria-pressed="selectedWeekdays.has(day.value)"
        @click="toggleWeekday(day.value)"
      >
        {{ day.label }}
      </button>
    </div>
    <span class="weekday-picker-hint">
      {{ selectedLabels ? `仅${selectedLabels}展示` : '未选择时每天展示' }}
    </span>
  </div>
</template>

<style scoped>
.weekday-picker {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid rgba(79, 70, 229, 0.16);
  border-radius: 8px;
  background: var(--bg-secondary);
}

.weekday-picker.compact {
  gap: 6px;
  padding: 9px 10px;
  border-color: var(--border-color);
  background: rgba(248, 250, 252, 0.82);
}

.weekday-picker-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.weekday-picker-label {
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
}

.weekday-picker-reset {
  padding: 2px 7px;
  border: 1px solid var(--border-color);
  border-radius: 5px;
  background: var(--bg-primary);
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  transition: border-color 0.16s ease, color 0.16s ease, background 0.16s ease;
}

.weekday-picker-reset:hover,
.weekday-picker-reset.active {
  border-color: var(--primary-color);
  background: rgba(239, 246, 255, 0.92);
  color: var(--primary-color);
}

.weekday-picker-options {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 6px;
}

.weekday-picker-option {
  min-width: 0;
  min-height: 32px;
  padding: 0 4px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  transition: border-color 0.16s ease, background 0.16s ease, color 0.16s ease, transform 0.16s ease;
}

.weekday-picker-option:hover {
  border-color: var(--primary-color);
  color: var(--primary-color);
  transform: translateY(-1px);
}

.weekday-picker-option.selected {
  border-color: var(--primary-color);
  background: var(--primary-color);
  color: #ffffff;
  font-weight: 600;
  box-shadow: 0 4px 10px rgba(37, 99, 235, 0.16);
}

.weekday-picker-hint {
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1.35;
}

@media (max-width: 420px) {
  .weekday-picker-options {
    gap: 4px;
  }

  .weekday-picker-option {
    font-size: 11px;
  }
}
</style>
