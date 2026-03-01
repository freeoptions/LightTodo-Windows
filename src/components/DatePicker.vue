<script setup lang="ts">
import { ref, computed, watch } from 'vue';

const props = defineProps<{
  modelValue: string[];  // 已选日期数组 (YYYY-MM-DD 格式)
  minDate?: string;      // 最小可选日期
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string[]): void;
}>();

// 当前显示的年月
const currentYear = ref(new Date().getFullYear());
const currentMonth = ref(new Date().getMonth()); // 0-11

// 本地状态，从 props 初始化
const selectedDates = ref<string[]>([...props.modelValue]);

// 只监听 props 变化，更新本地状态
watch(() => props.modelValue, (newVal) => {
  selectedDates.value = [...newVal];
}, { deep: true });

// 月份名称
const monthNames = ['一月', '二月', '三月', '四月', '五月', '六月', '七月', '八月', '九月', '十月', '十一月', '十二月'];

// 星期名称
const weekDays = ['日', '一', '二', '三', '四', '五', '六'];

// 切换到上个月
const prevMonth = () => {
  if (currentMonth.value === 0) {
    currentMonth.value = 11;
    currentYear.value--;
  } else {
    currentMonth.value--;
  }
};

// 切换到下个月
const nextMonth = () => {
  if (currentMonth.value === 11) {
    currentMonth.value = 0;
    currentYear.value++;
  } else {
    currentMonth.value++;
  }
};

// 返回今天
const goToToday = () => {
  const now = new Date();
  currentYear.value = now.getFullYear();
  currentMonth.value = now.getMonth();
};

// 获取当前月份的所有日期
const calendarDays = computed(() => {
  const firstDay = new Date(currentYear.value, currentMonth.value, 1);
  const lastDay = new Date(currentYear.value, currentMonth.value + 1, 0);
  const daysInMonth = lastDay.getDate();
  const startDayOfWeek = firstDay.getDay(); // 0-6

  const days: Array<{
    date: string;
    day: number;
    isCurrentMonth: boolean;
    isDisabled: boolean;
  }> = [];

  // 填充上个月的日期
  const prevMonthLastDay = new Date(currentYear.value, currentMonth.value, 0).getDate();
  for (let i = startDayOfWeek - 1; i >= 0; i--) {
    const day = prevMonthLastDay - i;
    const date = new Date(currentYear.value, currentMonth.value - 1, day);
    days.push({
      date: formatDateToYYYYMMDD(date),
      day,
      isCurrentMonth: false,
      isDisabled: true,
    });
  }

  // 填充当月的日期
  for (let i = 1; i <= daysInMonth; i++) {
    const date = new Date(currentYear.value, currentMonth.value, i);
    const dateStr = formatDateToYYYYMMDD(date);
    const isDisabled = props.minDate ? dateStr < props.minDate : false;
    days.push({
      date: dateStr,
      day: i,
      isCurrentMonth: true,
      isDisabled,
    });
  }

  // 填充下个月的日期（补齐到42个，6行）
  const remainingDays = 42 - days.length;
  for (let i = 1; i <= remainingDays; i++) {
    const date = new Date(currentYear.value, currentMonth.value + 1, i);
    days.push({
      date: formatDateToYYYYMMDD(date),
      day: i,
      isCurrentMonth: false,
      isDisabled: true,
    });
  }

  return days;
});

// 格式化日期为 YYYY-MM-DD
const formatDateToYYYYMMDD = (date: Date): string => {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
};

// 切换日期选中状态
const toggleDate = (dateStr: string) => {
  const index = selectedDates.value.indexOf(dateStr);
  if (index > -1) {
    selectedDates.value.splice(index, 1);
  } else {
    selectedDates.value.push(dateStr);
  }
  // 手动触发更新
  emit('update:modelValue', [...selectedDates.value]);
};

// 检查日期是否被选中
const isSelected = (dateStr: string) => {
  return selectedDates.value.includes(dateStr);
};

// 检查是否是今天
const isToday = (dateStr: string) => {
  const today = formatDateToYYYYMMDD(new Date());
  return dateStr === today;
};

// 当前显示的年月文本
const currentMonthText = computed(() => {
  return `${currentYear.value}年 ${monthNames[currentMonth.value]}`;
});
</script>

<template>
  <div class="date-picker">
    <!-- 头部：年月显示和切换 -->
    <div class="date-picker-header">
      <button @click="prevMonth" class="month-nav" title="上个月">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M11 3L7 7L11 11"/>
        </svg>
      </button>
      <span class="current-month">{{ currentMonthText }}</span>
      <button @click="nextMonth" class="month-nav" title="下个月">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
          <path d="M5 3L9 7L5 11"/>
        </svg>
      </button>
    </div>

    <!-- 星期标题 -->
    <div class="weekdays-header">
      <span v-for="day in weekDays" :key="day" class="weekday">{{ day }}</span>
    </div>

    <!-- 日期网格 -->
    <div class="calendar-grid">
      <div
        v-for="day in calendarDays"
        :key="day.date"
        :class="{
          'calendar-day': true,
          'current-month': day.isCurrentMonth,
          'other-month': !day.isCurrentMonth,
          'disabled': day.isDisabled,
          'selected': isSelected(day.date),
          'today': isToday(day.date)
        }"
        @click="!day.isDisabled && toggleDate(day.date)"
      >
        {{ day.day }}
      </div>
    </div>

    <!-- 底部：回到今天 -->
    <div class="date-picker-footer">
      <button @click="goToToday" class="btn-today">回到今天</button>
      <span class="selected-count">已选 {{ selectedDates.length }} 个日期</span>
    </div>
  </div>
</template>

<style scoped>
.date-picker {
  background: var(--bg-primary);
  border-radius: 8px;
  padding: 12px;
}

.date-picker-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.month-nav {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  cursor: pointer;
  transition: all 0.2s;
}

.month-nav:hover {
  background: var(--border-color);
}

.current-month {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.weekdays-header {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
  margin-bottom: 8px;
}

.weekday {
  text-align: center;
  font-size: 12px;
  color: var(--text-secondary);
  padding: 8px 0;
}

.calendar-grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
}

.calendar-day {
  aspect-ratio: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
  color: var(--text-primary);
}

.calendar-day.other-month {
  color: var(--text-secondary);
  opacity: 0.4;
}

.calendar-day.disabled {
  cursor: not-allowed;
  opacity: 0.3;
}

.calendar-day.current-month:not(.disabled):hover {
  background: var(--bg-secondary);
}

.calendar-day.selected {
  background: var(--primary-color);
  color: white;
  font-weight: 500;
}

.calendar-day.today {
  border: 2px solid var(--primary-color);
}

.calendar-day.today.selected {
  border-color: transparent;
}

.date-picker-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border-color);
}

.btn-today {
  padding: 6px 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-today:hover {
  background: var(--bg-secondary);
}

.selected-count {
  font-size: 12px;
  color: var(--text-secondary);
}

:root {
  --bg-primary: #ffffff;
  --bg-secondary: #f1f3f5;
  --text-primary: #1a1a1a;
  --text-secondary: #6c757d;
  --border-color: #dee2e6;
  --primary-color: #228be6;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #2a2a2a;
    --bg-secondary: #3a3a3a;
    --text-primary: #e9ecef;
    --text-secondary: #adb5bd;
    --border-color: #495057;
    --primary-color: #339af0;
  }
}
</style>
