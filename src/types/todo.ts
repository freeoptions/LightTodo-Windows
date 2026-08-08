/**
 * Todo item interface matching the Rust backend
 */
export interface TodoItem {
  id: string;
  content: string;
  repeatMode: 'daily' | 'weekly' | 'weekly_this_week' | 'monthly_this_month' | 'none' | 'specific_dates';
  weekdays?: string; // For weekly mode: "1,3,5" (周一=1, 周日=7)
  activeWeekdays?: string; // For subtodos: "1,5,6" (周一=1, 周日=7)
  specificDates?: string; // For specific_dates mode: "2025-03-10,2025-03-11"
  parentId?: string; // 父待办 ID，undefined 表示顶级待办
  expanded: boolean; // 是否展开子待办
  completed: boolean;
  disabled?: boolean; // 是否禁用，禁用后任务会变成完成状态且无法操作
  inactiveByWeekday?: boolean; // 运行时字段：当前日期是否因未命中启用星期而禁用
  createdAt: number;
  completedAt?: number;
  lastResetDate?: string;
  expiryDate?: string; // YYYY-MM-DD 格式，仅用于子待办
  cycleStartDate?: string; // YYYY-MM-DD，仅用于子待办周期启用
  cycleActiveDays?: number; // 启用天数，包含开始日
  cycleIntervalWeeks?: number; // 间隔周数，例如 4 表示每 4 周启用一轮
  cycleCompletionMode?: CycleCompletionMode; // 周期内完成规则，旧数据缺失时按每天重置处理
  priority?: 1 | 2 | 3; // 优先级，只对父待办有效
  order?: number; // 排序字段，用于手动调整待办顺序
  weekStart?: string; // YYYY-MM-DD 格式，记录周待办所属周的周一，仅用于 weekly_this_week 模式
  monthStart?: string; // YYYY-MM-DD 格式，记录月待办所属月的第一天，仅用于 monthly_this_month 模式
  subtodos?: TodoItem[]; // 子待办列表（运行时构建，不存储在后端）
}

export type CycleCompletionMode = 'daily' | 'once';

/**
 * Repeat mode options for the UI
 */
export const REPEAT_MODES = [
  { value: 'daily', label: '每天重复' },
  { value: 'weekly', label: '每周' },
  { value: 'weekly_this_week', label: '本周内' },
  { value: 'monthly_this_month', label: '本月内' },
  { value: 'specific_dates', label: '指定日期' },
  { value: 'none', label: '不重复' },
] as const;

export type RepeatMode = typeof REPEAT_MODES[number]['value'];

/**
 * Long term todo interface - separate from daily todos
 * These are persistent todos without time limits
 */
export interface LongTermTodo {
  id: string;
  content: string;
  completed: boolean;
  createdAt: number;
  completedAt?: number;
}

/** One-off reminders with a single deadline, independent from recurring todos. */
export interface DeadlineReminder {
  id: string;
  title: string;
  dueDate: string;
  createdAt: number;
  updatedAt: number;
}
