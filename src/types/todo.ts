/**
 * Todo item interface matching the Rust backend
 */
export interface TodoItem {
  id: string;
  content: string;
  repeatMode: 'daily' | 'weekly' | 'none' | 'specific_dates';
  weekdays?: string; // For weekly mode: "1,3,5" (周一=1, 周日=7)
  specificDates?: string; // For specific_dates mode: "2025-03-10,2025-03-11"
  parentId?: string; // 父待办ID，undefined表示是顶级待办
  expanded: boolean; // 是否展开子待办
  completed: boolean;
  disabled?: boolean; // 是否禁用，禁用后任务会变成完成状态且无法操作
  createdAt: number;
  completedAt?: number;
  lastResetDate?: string;
  subtodos?: TodoItem[]; // 子待办列表（运行时构建，不存储在后端）
}

/**
 * Repeat mode options for the UI
 */
export const REPEAT_MODES = [
  { value: 'daily', label: '每天重复' },
  { value: 'weekly', label: '每周' },
  { value: 'specific_dates', label: '指定日期' },
  { value: 'none', label: '不重复' },
] as const;

export type RepeatMode = typeof REPEAT_MODES[number]['value'];
