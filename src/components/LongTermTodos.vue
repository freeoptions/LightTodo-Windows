<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue';
import { useLongTermTodos } from '../composables/useLongTermTodos';
import ArrowIcon from './ArrowIcon.vue';

const emit = defineEmits<{
  (e: 'close'): void;
}>();

const {
  longTermTodos,
  loading,
  error,
  loadLongTermTodos,
  addLongTermTodo,
  toggleLongTermTodo,
  updateLongTermTodo,
  deleteLongTermTodo,
  moveLongTermTodoUp,
  moveLongTermTodoDown,
} = useLongTermTodos();

// New todo input
const newTodoContent = ref('');
const addingTodo = ref(false);

// Edit todo
const editingTodoId = ref<string | null>(null);
const editingTodoContent = ref('');
const showEditModal = ref(false);
const savingEdit = ref(false);

// Delete confirmation
const showDeleteConfirm = ref(false);
const deletingTodoId = ref<string | null>(null);
const deletingTodoContent = ref('');

// Store body scroll position to prevent jump when modal opens
let bodyScrollPosition = 0;

const lockBodyScroll = () => {
  bodyScrollPosition = window.pageYOffset || document.documentElement.scrollTop;
  document.body.style.position = 'fixed';
  document.body.style.top = `-${bodyScrollPosition}px`;
  document.body.style.width = '100%';
  document.body.style.overflow = 'hidden';
};

const unlockBodyScroll = () => {
  document.body.style.position = '';
  document.body.style.top = '';
  document.body.style.width = '';
  document.body.style.overflow = '';
  window.scrollTo(0, bodyScrollPosition);
};

onMounted(() => {
  loadLongTermTodos();
  lockBodyScroll();
});

onUnmounted(() => {
  unlockBodyScroll();
});

const handleAddTodo = async () => {
  const content = newTodoContent.value.trim();
  if (!content) return;

  addingTodo.value = true;
  try {
    await addLongTermTodo(content);
    newTodoContent.value = '';
  } catch (e) {
    console.error('Failed to add long term todo:', e);
  } finally {
    addingTodo.value = false;
  }
};

const handleKeyPress = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleAddTodo();
  }
};

const confirmDelete = (id: string, content: string) => {
  deletingTodoId.value = id;
  deletingTodoContent.value = content;
  showDeleteConfirm.value = true;
};

const handleDeleteConfirm = async () => {
  if (deletingTodoId.value) {
    await deleteLongTermTodo(deletingTodoId.value);
    closeDeleteConfirm();
  }
};

const closeDeleteConfirm = () => {
  showDeleteConfirm.value = false;
  deletingTodoId.value = null;
  deletingTodoContent.value = '';
};

// Edit todo functions
const openEditModal = (id: string, content: string) => {
  editingTodoId.value = id;
  editingTodoContent.value = content;
  showEditModal.value = true;
};

const closeEditModal = () => {
  showEditModal.value = false;
  editingTodoId.value = null;
  editingTodoContent.value = '';
};

const handleEditSave = async () => {
  if (!editingTodoId.value || !editingTodoContent.value.trim()) return;

  savingEdit.value = true;
  try {
    await updateLongTermTodo(editingTodoId.value, editingTodoContent.value.trim());
    closeEditModal();
  } catch (e) {
    console.error('Failed to update long term todo:', e);
  } finally {
    savingEdit.value = false;
  }
};

const handleEditKeyPress = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleEditSave();
  }
};

// Separate completed and incomplete todos
const incompleteTodos = computed(() => longTermTodos.value.filter(t => !t.completed));
const completedTodos = computed(() => longTermTodos.value.filter(t => t.completed));

// Get display index in the full list
const getDisplayIndex = (id: string): number => {
  return longTermTodos.value.findIndex(t => t.id === id);
};

const canMoveUp = (id: string): boolean => {
  const index = getDisplayIndex(id);
  return index > 0;
};

const canMoveDown = (id: string): boolean => {
  const index = getDisplayIndex(id);
  return index < longTermTodos.value.length - 1;
};

// Keep native scrolling inside the modal.
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal-container long-term-modal">
      <!-- Modal Header -->
      <div class="modal-header">
        <h2 class="modal-title">🎯 长期待办</h2>
        <button @click="emit('close')" class="modal-close">✕</button>
      </div>

      <!-- Modal Content -->
      <div class="modal-content long-term-content">
        <!-- Error Message -->
        <div v-if="error" class="error-message">
          <span class="error-icon">⚠</span>
          <span>{{ error }}</span>
        </div>

        <!-- Add New Todo Section -->
        <div class="add-todo-section">
          <div class="input-wrapper">
            <textarea
              v-model="newTodoContent"
              @keydown="handleKeyPress"
              class="todo-input"
              placeholder="输入长期待办事项..."
              rows="2"
              :disabled="addingTodo"
            ></textarea>
          </div>
          <button
            @click="handleAddTodo"
            class="btn-add"
            :disabled="addingTodo || !newTodoContent.trim()"
          >
            {{ addingTodo ? '添加中...' : '添加' }}
          </button>
        </div>

        <!-- Loading State -->
        <div v-if="loading && longTermTodos.length === 0" class="loading-state">
          <span class="loading-spinner"></span>
          <span>加载中...</span>
        </div>

        <!-- Empty State -->
        <div v-else-if="longTermTodos.length === 0" class="empty-state">
          <div class="empty-icon">🎯</div>
          <p>还没有长期待办</p>
          <p class="empty-hint">添加你的第一个长期目标吧！</p>
        </div>

        <!-- Todo Lists -->
        <template v-else>
          <!-- Incomplete Section -->
          <div v-if="incompleteTodos.length > 0" class="todo-section">
            <h3 class="section-title incomplete">进行中</h3>
            <div class="todo-list">
              <div
                v-for="todo in incompleteTodos"
                :key="todo.id"
                class="long-term-todo-item"
                :class="{ completed: todo.completed }"
              >
                <label class="todo-checkbox">
                  <input
                    type="checkbox"
                    :checked="todo.completed"
                    @change="toggleLongTermTodo(todo.id)"
                  />
                  <span class="checkbox-icon">
                    <span class="icon-unchecked">⃞</span>
                    <span class="icon-checked">✅</span>
                  </span>
                </label>
                <span class="todo-content">{{ todo.content }}</span>
                <div class="todo-actions">
                  <button
                    @click="openEditModal(todo.id, todo.content)"
                    class="action-btn edit-btn"
                    title="编辑"
                  >✏️</button>
                  <button
                    v-if="canMoveUp(todo.id)"
                    @click="moveLongTermTodoUp(todo.id)"
                    class="action-btn move-btn"
                    title="上移"
                  >
                    <ArrowIcon direction="up" />
                  </button>
                  <button
                    v-if="canMoveDown(todo.id)"
                    @click="moveLongTermTodoDown(todo.id)"
                    class="action-btn move-btn"
                    title="下移"
                  >
                    <ArrowIcon direction="down" />
                  </button>
                  <button
                    @click="confirmDelete(todo.id, todo.content)"
                    class="action-btn delete-btn"
                    title="删除"
                  >❎</button>
                </div>
              </div>
            </div>
          </div>

          <!-- Completed Section -->
          <div v-if="completedTodos.length > 0" class="todo-section completed">
            <h3 class="section-title completed">已完成</h3>
            <div class="todo-list">
              <div
                v-for="todo in completedTodos"
                :key="todo.id"
                class="long-term-todo-item completed"
              >
                <label class="todo-checkbox">
                  <input
                    type="checkbox"
                    :checked="todo.completed"
                    @change="toggleLongTermTodo(todo.id)"
                  />
                  <span class="checkbox-icon">
                    <span class="icon-unchecked">⃞</span>
                    <span class="icon-checked">✅</span>
                  </span>
                </label>
                <span class="todo-content">{{ todo.content }}</span>
                <div class="todo-actions">
                  <button
                    @click="confirmDelete(todo.id, todo.content)"
                    class="action-btn delete-btn"
                    title="删除"
                  >❎</button>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>

      <!-- Delete Confirmation Modal -->
      <div v-if="showDeleteConfirm" class="delete-confirm-overlay" @click.self="closeDeleteConfirm">
        <div class="delete-confirm-modal">
          <div class="delete-confirm-header">
            <h3>确认删除</h3>
          </div>
          <div class="delete-confirm-body">
            <p>确定要删除这个长期待办吗？</p>
            <p class="delete-confirm-content">"{{ deletingTodoContent }}"</p>
          </div>
          <div class="delete-confirm-actions">
            <button @click="closeDeleteConfirm" class="btn-cancel">取消</button>
            <button @click="handleDeleteConfirm" class="btn-confirm btn-danger">确认删除</button>
          </div>
        </div>
      </div>

      <!-- Edit Modal -->
      <div v-if="showEditModal" class="delete-confirm-overlay" @click.self="closeEditModal">
        <div class="delete-confirm-modal">
          <div class="delete-confirm-header">
            <h3>编辑长期待办</h3>
          </div>
          <div class="delete-confirm-body">
            <textarea
              v-model="editingTodoContent"
              @keydown="handleEditKeyPress"
              class="edit-textarea"
              rows="4"
              :disabled="savingEdit"
              placeholder="输入待办内容..."
            ></textarea>
          </div>
          <div class="delete-confirm-actions">
            <button @click="closeEditModal" class="btn-cancel" :disabled="savingEdit">取消</button>
            <button @click="handleEditSave" class="btn-confirm" :disabled="savingEdit || !editingTodoContent.trim()">
              {{ savingEdit ? '保存中...' : '保存' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Modal Overlay */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

/* Modal Container */
.modal-container {
  background: var(--bg-primary, #ffffff);
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  max-width: 600px;
  width: 90%;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
}

.long-term-modal {
  height: 85vh;
  min-height: 500px;
}

/* Modal Header */
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid var(--border-color, #dee2e6);
}

.modal-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
  margin: 0;
}

.modal-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  font-size: 24px;
  color: var(--text-secondary, #6c757d);
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}

.modal-close:hover {
  background: var(--bg-secondary, #f8f9fa);
  color: var(--text-primary, #1a1a1a);
}

/* Modal Content */
.modal-content {
  padding: 24px;
  overflow-y: auto;
  flex: 1;
}

.long-term-content {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* Error Message */
.error-message {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: var(--danger-bg, #fee2e2);
  color: var(--danger-color, #ef4444);
  border-radius: 8px;
  font-size: 14px;
}

/* Add Todo Section */
.add-todo-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
  background: var(--bg-secondary, #f8f9fa);
  border-radius: 8px;
}

.input-wrapper {
  display: flex;
}

.todo-input {
  flex: 1;
  padding: 12px;
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 8px;
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  min-height: 60px;
  background: var(--bg-primary, #ffffff);
  color: var(--text-primary, #1a1a1a);
  transition: border-color 0.2s;
}

.todo-input:focus {
  outline: none;
  border-color: var(--primary-color, #4f46e5);
}

.todo-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.todo-input::placeholder {
  color: var(--text-secondary, #6c757d);
  white-space: pre-line;
}

.todo-input::-webkit-scrollbar {
  display: none;
}

.todo-input {
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.btn-add {
  padding: 10px 20px;
  background: var(--primary-color, #4f46e5);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
  align-self: flex-end;
}

.btn-add:hover:not(:disabled) {
  background: #4338ca;
}

.btn-add:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

/* Loading & Empty States */
.loading-state {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 48px;
  color: var(--text-secondary, #6c757d);
}

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-color, #dee2e6);
  border-top-color: var(--primary-color, #4f46e5);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px;
  text-align: center;
  color: var(--text-secondary, #6c757d);
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
  opacity: 0.8;
}

.empty-hint {
  font-size: 14px;
  opacity: 0.7;
  margin-top: 4px;
}

/* Todo Section */
.todo-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
  padding: 4px 8px;
  border-radius: 4px;
  display: inline-block;
}

.section-title.incomplete {
  color: #f59e0b;
}

.section-title.completed {
  color: #22c55e;
}

.todo-section.completed {
  border-top: 1px solid var(--border-color, #dee2e6);
  padding-top: 16px;
}

/* Todo List */
.todo-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* Long Term Todo Item */
.long-term-todo-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: var(--bg-primary, #ffffff);
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 8px;
  transition: all 0.2s;
}

.long-term-todo-item:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
}

.long-term-todo-item.completed {
  opacity: 0.7;
  background: var(--bg-secondary, #f8f9fa);
}

.long-term-todo-item.completed .todo-content {
  text-decoration: line-through;
  color: var(--text-secondary, #6c757d);
}

/* Checkbox */
.todo-checkbox {
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
}

.todo-checkbox input[type="checkbox"] {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
  pointer-events: none;
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
  color: var(--border-color, #dee2e6);
  font-size: 26px;
}

.icon-checked {
  display: none;
  color: #22c55e;
}

.todo-checkbox input:checked ~ .checkbox-icon .icon-unchecked {
  display: none;
}

.todo-checkbox input:checked ~ .checkbox-icon .icon-checked {
  display: block;
}

/* Todo Content */
.todo-content {
  flex: 1;
  font-size: 15px;
  color: var(--text-primary, #1a1a1a);
  word-break: break-word;
  white-space: pre-wrap;
  min-width: 0;
}

/* Todo Actions */
.todo-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.long-term-todo-item:hover .todo-actions {
  opacity: 1;
}

.action-btn {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary, #6c757d);
  font-size: 16px;
  cursor: pointer;
  transition: all 0.2s;
}

.move-btn {
  font-size: 17px;
}

.move-btn:hover {
  background: rgba(79, 70, 229, 0.1);
  color: var(--primary-color, #4f46e5);
}

.edit-btn:hover {
  background: var(--bg-secondary, #f8f9fa);
  color: var(--primary-color, #4f46e5);
}

.delete-btn:hover {
  background: var(--danger-bg, #fee2e2);
  color: var(--danger-color, #ef4444);
}

/* Delete Confirm Modal */
.delete-confirm-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1100;
}

.delete-confirm-modal {
  background: var(--bg-primary, #ffffff);
  border-radius: 12px;
  padding: 24px;
  max-width: 400px;
  width: 90%;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.delete-confirm-header h3 {
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
  margin: 0 0 16px 0;
}

.delete-confirm-body {
  margin-bottom: 20px;
}

.delete-confirm-body p {
  margin: 0 0 8px 0;
  font-size: 14px;
  color: var(--text-primary, #1a1a1a);
}

.delete-confirm-content {
  font-style: italic;
  color: var(--text-secondary, #6c757d) !important;
  padding: 8px 12px;
  background: var(--bg-secondary, #f8f9fa);
  border-radius: 6px;
  margin-top: 12px !important;
}

.delete-confirm-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}

.btn-cancel,
.btn-confirm {
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-cancel {
  background: var(--bg-secondary, #f8f9fa);
  color: var(--text-primary, #1a1a1a);
}

.btn-cancel:hover {
  background: #e9ecef;
}

.btn-confirm {
  background: var(--primary-color, #4f46e5);
  color: white;
}

.btn-confirm:hover {
  background: #4338ca;
}

.btn-confirm.btn-danger {
  background: var(--danger-color, #ef4444);
}

.btn-confirm.btn-danger:hover {
  background: #dc2626;
}

/* Edit Textarea */
.edit-textarea {
  width: 100%;
  padding: 12px;
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 8px;
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  min-height: 100px;
  background: var(--bg-primary, #ffffff);
  color: var(--text-primary, #1a1a1a);
  transition: border-color 0.2s;
}

.edit-textarea:focus {
  outline: none;
  border-color: var(--primary-color, #4f46e5);
}

.edit-textarea:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.edit-textarea::placeholder {
  color: var(--text-secondary, #6c757d);
}

/* Dark Mode */
@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #2a2a2a;
    --text-primary: #e9ecef;
    --text-secondary: #adb5bd;
    --border-color: #495057;
    --primary-color: #6366f1;
    --danger-color: #f87171;
    --danger-bg: #7f1d1d;
  }
}

</style>
