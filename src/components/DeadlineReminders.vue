<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useDeadlineReminders } from '../composables/useDeadlineReminders';

const {
  reminders,
  loading,
  error,
  loadReminders,
  addReminder,
  updateReminder,
  deleteReminder,
} = useDeadlineReminders();

const showEditor = ref(false);
const editingId = ref<string | null>(null);
const formTitle = ref('');
const formDueDate = ref('');
const formError = ref('');
const saving = ref(false);
const showDeleteConfirm = ref(false);
const deletingId = ref<string | null>(null);
const deleting = ref(false);
const clockTick = ref(Date.now());
let clockTimer: ReturnType<typeof setInterval> | null = null;

const isoDateFromDate = (date: Date) => {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
};

const todayIso = computed(() => isoDateFromDate(new Date(clockTick.value)));

const dateToDayNumber = (isoDate: string) => {
  const [year, month, day] = isoDate.split('-').map(Number);
  return Date.UTC(year, month - 1, day) / 86400000;
};

const daysUntil = (dueDate: string) => {
  return dateToDayNumber(dueDate) - dateToDayNumber(todayIso.value);
};

const statusFor = (dueDate: string) => {
  const days = daysUntil(dueDate);
  if (days < 0) return 'overdue';
  if (days === 0) return 'today';
  if (days <= 4) return 'urgent';
  return 'normal';
};

const statusTextFor = (dueDate: string) => {
  const days = daysUntil(dueDate);
  if (days < 0) return `已逾期 ${Math.abs(days)} 天`;
  if (days === 0) return '今天到期';
  if (days === 1) return '明天到期';
  return `剩余 ${days} 天`;
};

const formatDueDate = (isoDate: string) => {
  const [year, month, day] = isoDate.split('-');
  return `${year}年${Number(month)}月${Number(day)}日`;
};

const sortedReminders = computed(() => [...reminders.value].sort((a, b) => {
  const dateOrder = a.dueDate.localeCompare(b.dueDate);
  if (dateOrder !== 0) return dateOrder;
  return a.createdAt - b.createdAt;
}));

const urgentCount = computed(() => sortedReminders.value.filter((reminder) => {
  const days = daysUntil(reminder.dueDate);
  return days <= 4;
}).length);

const openCreate = () => {
  editingId.value = null;
  formTitle.value = '';
  formDueDate.value = todayIso.value;
  formError.value = '';
  showEditor.value = true;
};

const openEdit = (id: string) => {
  const reminder = reminders.value.find((item) => item.id === id);
  if (!reminder) return;
  editingId.value = id;
  formTitle.value = reminder.title;
  formDueDate.value = reminder.dueDate;
  formError.value = '';
  showEditor.value = true;
};

const closeEditor = () => {
  if (saving.value) return;
  showEditor.value = false;
  formError.value = '';
};

const submitReminder = async () => {
  const title = formTitle.value.trim();
  if (!title) {
    formError.value = '请填写提醒内容';
    return;
  }
  if (!formDueDate.value) {
    formError.value = '请选择截止日期';
    return;
  }

  saving.value = true;
  formError.value = '';
  try {
    if (editingId.value) {
      await updateReminder(editingId.value, title, formDueDate.value);
    } else {
      await addReminder(title, formDueDate.value);
    }
    showEditor.value = false;
  } catch (e) {
    formError.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
};

const deletingTitle = computed(() => {
  if (!deletingId.value) return '';
  return reminders.value.find((item) => item.id === deletingId.value)?.title ?? '';
});

const requestRemoveReminder = (id: string) => {
  const reminder = reminders.value.find((item) => item.id === id);
  if (!reminder) return;
  deletingId.value = id;
  showDeleteConfirm.value = true;
};

const closeDeleteConfirm = () => {
  if (deleting.value) return;
  showDeleteConfirm.value = false;
  deletingId.value = null;
};

const confirmDeleteReminder = async () => {
  if (!deletingId.value) return;
  deleting.value = true;
  try {
    await deleteReminder(deletingId.value);
    showDeleteConfirm.value = false;
    deletingId.value = null;
  } finally {
    deleting.value = false;
  }
};

onMounted(() => {
  loadReminders();
  clockTimer = setInterval(() => {
    clockTick.value = Date.now();
  }, 60_000);
});

onUnmounted(() => {
  if (clockTimer) clearInterval(clockTimer);
});
</script>

<template>
  <section class="deadline-panel" :class="{ 'has-urgent': urgentCount > 0 }">
    <div class="deadline-panel-header">
      <h2 class="deadline-section-title">截止提醒</h2>
      <button type="button" class="deadline-add-button" @click="openCreate">
        <span aria-hidden="true">＋</span>
        添加
      </button>
    </div>

    <div v-if="error" class="deadline-error">{{ error }}</div>
    <div v-if="loading && sortedReminders.length === 0" class="deadline-empty">正在读取提醒…</div>
    <div v-else-if="sortedReminders.length === 0" class="deadline-empty deadline-empty-action" @click="openCreate">
      <span class="deadline-empty-icon">＋</span>
      <span>添加第一条截止提醒</span>
    </div>
    <div v-else class="deadline-list">
      <article
        v-for="reminder in sortedReminders"
        :key="reminder.id"
        class="deadline-item"
        :class="`deadline-item-${statusFor(reminder.dueDate)}`"
      >
        <div class="deadline-date-badge">
          <span>{{ reminder.dueDate.slice(5, 7) }}</span>
          <strong>{{ reminder.dueDate.slice(8, 10) }}</strong>
        </div>
        <div class="deadline-item-main">
          <div class="deadline-item-title">{{ reminder.title }}</div>
          <div class="deadline-item-meta">
            <span>{{ formatDueDate(reminder.dueDate) }}</span>
            <span class="deadline-dot">·</span>
            <strong>{{ statusTextFor(reminder.dueDate) }}</strong>
          </div>
        </div>
        <div class="deadline-item-actions">
          <button type="button" title="编辑提醒" aria-label="编辑提醒" @click="openEdit(reminder.id)">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4 12.5-12.5Z" /></svg>
          </button>
          <button type="button" title="删除提醒" aria-label="删除提醒" @click="requestRemoveReminder(reminder.id)">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6M10 11v5M14 11v5" /></svg>
          </button>
        </div>
      </article>
    </div>
  </section>

  <div v-if="showEditor" class="deadline-editor-overlay" @click.self="closeEditor">
    <div class="deadline-editor" role="dialog" aria-modal="true" aria-labelledby="deadline-editor-title">
      <div class="deadline-editor-header">
        <div>
          <h2 id="deadline-editor-title">{{ editingId ? '编辑提醒' : '添加提醒' }}</h2>
        </div>
        <button type="button" class="deadline-editor-close" aria-label="关闭" @click="closeEditor">×</button>
      </div>
      <label class="deadline-field">
        <span>提醒内容</span>
        <input v-model="formTitle" type="text" maxlength="120" placeholder="例如：续签证件、提交报名材料" @keydown.enter.prevent="submitReminder" />
      </label>
      <label class="deadline-field">
        <span>截止日期</span>
        <input v-model="formDueDate" type="date" />
      </label>
      <div v-if="formError" class="deadline-form-error">{{ formError }}</div>
      <div class="deadline-editor-footer">
        <button type="button" class="deadline-cancel" :disabled="saving" @click="closeEditor">取消</button>
        <button type="button" class="deadline-submit" :disabled="saving" @click="submitReminder">
          {{ saving ? '保存中…' : '保存提醒' }}
        </button>
      </div>
    </div>
  </div>

  <div v-if="showDeleteConfirm" class="deadline-confirm-overlay" @click.self="closeDeleteConfirm">
    <div
      class="deadline-confirm"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="deadline-confirm-title"
      aria-describedby="deadline-confirm-message"
    >
      <div class="deadline-confirm-content">
        <div class="deadline-confirm-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M3 6h18M8 6V4h8v2M19 6l-1 14H6L5 6M10 11v5M14 11v5" /></svg>
        </div>
        <div class="deadline-confirm-copy">
          <h2 id="deadline-confirm-title">确认删除提醒？</h2>
          <p id="deadline-confirm-message">
            将删除“<strong>{{ deletingTitle }}</strong>”，删除后无法恢复。
          </p>
        </div>
      </div>
      <div class="deadline-confirm-actions">
        <button type="button" class="deadline-confirm-cancel" :disabled="deleting" @click="closeDeleteConfirm">取消</button>
        <button type="button" class="deadline-confirm-delete" :disabled="deleting" @click="confirmDeleteReminder">
          {{ deleting ? '删除中…' : '确认删除' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.deadline-panel {
  position: relative;
  overflow: hidden;
  padding: 16px;
  border: 1px solid rgba(129, 140, 248, 0.28);
  border-radius: 14px;
  background: linear-gradient(135deg, #ffffff 0%, #fbfbff 64%, #f5f3ff 100%);
  box-shadow: 0 12px 28px rgba(79, 70, 229, 0.07);
}

.deadline-panel::after {
  position: absolute;
  top: -58px;
  right: -42px;
  width: 142px;
  height: 142px;
  border-radius: 50%;
  background: rgba(99, 102, 241, 0.09);
  content: '';
  pointer-events: none;
}

.deadline-panel.has-urgent {
  border-color: rgba(129, 140, 248, 0.44);
  box-shadow: 0 14px 30px rgba(79, 70, 229, 0.11);
}

.deadline-panel-header,
.deadline-item,
.deadline-item-meta,
.deadline-editor-header,
.deadline-editor-footer {
  display: flex;
  align-items: center;
}

.deadline-panel-header {
  position: relative;
  z-index: 1;
  justify-content: space-between;
  gap: 12px;
}

.deadline-section-title {
  position: relative;
  margin: 0;
  padding-left: 11px;
  color: #27346b;
  font-size: 16px;
  font-weight: 850;
  letter-spacing: 0.04em;
  line-height: 1.2;
}

.deadline-section-title::before {
  position: absolute;
  top: 2px;
  bottom: 2px;
  left: 0;
  width: 3px;
  border-radius: 999px;
  background: linear-gradient(180deg, #4f46e5, #7c3aed);
  content: '';
}

.deadline-item-actions svg {
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.8;
}

.deadline-editor h2 {
  margin: 2px 0 0;
  color: var(--text-primary);
  font-size: 16px;
  font-weight: 800;
}

.deadline-add-button,
.deadline-submit,
.deadline-cancel {
  border: 0;
  border-radius: 8px;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 800;
  transition: transform 0.18s ease, box-shadow 0.18s ease, background 0.18s ease;
}

.deadline-add-button {
  position: relative;
  z-index: 1;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 8px 10px;
  color: #4338ca;
  background: #eef2ff;
  white-space: nowrap;
}

.deadline-add-button:hover,
.deadline-submit:hover {
  transform: translateY(-1px);
  box-shadow: 0 7px 16px rgba(79, 70, 229, 0.2);
}

.deadline-list {
  position: relative;
  z-index: 1;
  display: grid;
  gap: 7px;
  margin-top: 14px;
}

.deadline-item {
  gap: 10px;
  min-width: 0;
  padding: 9px 10px 9px 9px;
  border: 1px solid rgba(203, 213, 225, 0.76);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.78);
  transition: border-color 0.18s ease, transform 0.18s ease, box-shadow 0.18s ease;
}

.deadline-item:hover {
  transform: translateY(-1px);
  border-color: rgba(148, 163, 184, 0.9);
  box-shadow: 0 7px 15px rgba(23, 33, 43, 0.06);
}

.deadline-item-urgent {
  border-color: rgba(124, 58, 237, 0.38);
  background: #faf7ff;
}

.deadline-item-today {
  border-color: rgba(239, 68, 68, 0.72);
  background: #fff1f2;
  box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.09), 0 8px 18px rgba(239, 68, 68, 0.1);
}

.deadline-item-overdue {
  border-color: rgba(220, 38, 38, 0.5);
  background: #fff1f2;
}

.deadline-date-badge {
  display: flex;
  width: 40px;
  height: 43px;
  flex: 0 0 40px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  color: #475569;
  background: #f1f5f9;
  font-size: 10px;
  line-height: 1.05;
}

.deadline-date-badge strong {
  margin-top: 2px;
  color: #1e293b;
  font-size: 17px;
  line-height: 1;
}

.deadline-item-urgent .deadline-date-badge,
.deadline-item-urgent .deadline-date-badge {
  color: #6d28d9;
  background: #ddd6fe;
}

.deadline-item-today .deadline-date-badge {
  color: #b91c1c;
  background: #fecdd3;
}

.deadline-item-overdue .deadline-date-badge {
  color: #b91c1c;
  background: #fecdd3;
}

.deadline-item-main {
  min-width: 0;
  flex: 1;
}

.deadline-item-title {
  overflow: hidden;
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 800;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.deadline-item-meta {
  gap: 5px;
  margin-top: 3px;
  color: var(--text-secondary);
  font-size: 11px;
}

.deadline-item-meta strong {
  color: #64748b;
  font-weight: 800;
}

.deadline-item-urgent .deadline-item-meta strong {
  color: #6d28d9;
}

.deadline-item-today .deadline-item-meta strong {
  color: #dc2626;
}

.deadline-item-overdue .deadline-item-meta strong {
  color: #dc2626;
}

.deadline-dot {
  color: #cbd5e1;
}

.deadline-item-actions {
  display: flex;
  gap: 2px;
  flex: 0 0 auto;
}

.deadline-item-actions button,
.deadline-editor-close {
  display: grid;
  width: 28px;
  height: 28px;
  padding: 0;
  place-items: center;
  border: 0;
  border-radius: 7px;
  color: #64748b;
  background: transparent;
  cursor: pointer;
}

.deadline-item-actions button:hover {
  color: #4f46e5;
  background: #eef2ff;
}

.deadline-item-actions button:last-child:hover {
  color: #dc2626;
  background: #fee2e2;
}

.deadline-empty,
.deadline-error {
  position: relative;
  z-index: 1;
  margin-top: 12px;
  color: var(--text-secondary);
  font-size: 12px;
}

.deadline-empty-action {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
  padding: 18px 12px 16px;
  border: 1px dashed rgba(99, 102, 241, 0.42);
  border-radius: 10px;
  color: #4f46e5;
  background: rgba(238, 242, 255, 0.64);
  cursor: pointer;
}

.deadline-empty-action:hover {
  background: #eef2ff;
}

.deadline-empty-icon {
  font-size: 21px;
  font-weight: 300;
}

.deadline-error,
.deadline-form-error {
  color: #dc2626;
}

.deadline-editor-overlay {
  position: fixed;
  z-index: 1100;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 18px;
  background: rgba(15, 23, 42, 0.38);
  backdrop-filter: blur(5px);
}

.deadline-confirm-overlay {
  position: fixed;
  z-index: 1110;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 18px;
  background: rgba(15, 23, 42, 0.42);
  backdrop-filter: blur(6px);
}

.deadline-confirm {
  width: min(100%, 390px);
  padding: 22px;
  border: 1px solid rgba(226, 232, 240, 0.95);
  border-radius: 18px;
  background: linear-gradient(145deg, #ffffff 0%, #fbfbff 100%);
  box-shadow: 0 26px 80px rgba(15, 23, 42, 0.24);
}

.deadline-confirm-content {
  display: flex;
  align-items: flex-start;
  gap: 13px;
}

.deadline-confirm-icon {
  display: grid;
  width: 42px;
  height: 42px;
  flex: 0 0 42px;
  place-items: center;
  border-radius: 12px;
  color: #dc2626;
  background: #fee2e2;
}

.deadline-confirm-icon svg {
  width: 20px;
  height: 20px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.8;
}

.deadline-confirm-copy {
  min-width: 0;
}

.deadline-confirm-copy h2 {
  margin: 1px 0 0;
  color: #27346b;
  font-size: 16px;
  font-weight: 850;
}

.deadline-confirm-copy p {
  margin: 7px 0 0;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.65;
  overflow-wrap: anywhere;
}

.deadline-confirm-copy strong {
  color: var(--text-primary);
  font-weight: 800;
}

.deadline-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 22px;
}

.deadline-confirm-cancel,
.deadline-confirm-delete {
  min-height: 36px;
  padding: 0 14px;
  border: 0;
  border-radius: 9px;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 800;
  transition: transform 0.18s ease, box-shadow 0.18s ease, background 0.18s ease;
}

.deadline-confirm-cancel {
  color: var(--text-secondary);
  background: #f1f5f9;
}

.deadline-confirm-cancel:hover {
  background: #e2e8f0;
}

.deadline-confirm-delete {
  color: #fff;
  background: #dc2626;
}

.deadline-confirm-delete:hover {
  transform: translateY(-1px);
  background: #b91c1c;
  box-shadow: 0 7px 16px rgba(220, 38, 38, 0.2);
}

.deadline-confirm-cancel:disabled,
.deadline-confirm-delete:disabled {
  cursor: wait;
  opacity: 0.6;
}

.deadline-editor {
  width: min(100%, 410px);
  padding: 20px;
  border: 1px solid rgba(226, 232, 240, 0.92);
  border-radius: 16px;
  background: var(--bg-primary);
  box-shadow: 0 24px 70px rgba(15, 23, 42, 0.2);
}

.deadline-editor-header {
  justify-content: space-between;
  margin-bottom: 18px;
}

.deadline-editor-close {
  font-size: 24px;
  font-weight: 300;
}

.deadline-editor-close:hover {
  color: #4f46e5;
  background: #eef2ff;
}

.deadline-field {
  display: grid;
  gap: 7px;
  margin-top: 14px;
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 800;
}

.deadline-field input {
  width: 100%;
  min-height: 40px;
  padding: 0 11px;
  border: 1px solid var(--border-color);
  border-radius: 9px;
  color: var(--text-primary);
  background: var(--bg-secondary);
  font: inherit;
  font-size: 13px;
}

.deadline-field input:focus {
  outline: none;
  border-color: #6366f1;
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.16);
}

.deadline-form-error {
  margin-top: 10px;
  font-size: 12px;
}

.deadline-editor-footer {
  justify-content: flex-end;
  gap: 8px;
  margin-top: 22px;
}

.deadline-cancel,
.deadline-submit {
  min-height: 36px;
  padding: 0 13px;
}

.deadline-cancel {
  color: var(--text-secondary);
  background: var(--bg-secondary);
}

.deadline-cancel:hover {
  background: #e2e8f0;
}

.deadline-submit {
  color: #fff;
  background: #4f46e5;
}

.deadline-cancel:disabled,
.deadline-submit:disabled {
  cursor: wait;
  opacity: 0.6;
}

@media (max-width: 520px) {
  .deadline-panel {
    padding: 14px;
  }

  .deadline-panel-header {
    align-items: flex-start;
  }

}
</style>
