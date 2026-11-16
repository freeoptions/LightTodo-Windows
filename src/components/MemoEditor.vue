<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { decodeMemo, encodeMemo } from '../utils/memoFormat';

const props = defineProps<{ modelValue: string; height: number; saving: boolean }>();
const emit = defineEmits<{
  (event: 'update:modelValue', value: string): void;
  (event: 'resize', height: number): void;
}>();

const formats = [
  { command: 'bold', label: '加粗', key: 'B', symbol: 'B' },
  { command: 'italic', label: '斜体', key: 'I', symbol: 'I' },
  { command: 'underline', label: '下划线', key: 'U', symbol: 'U' },
  { command: 'strikeThrough', label: '删除线', key: '', symbol: 'S' },
] as const;
const editor = ref<HTMLDivElement>();
const activeFormats = ref<string[]>([]);
let savedRange: Range | null = null;
let lastValue = props.modelValue;
let composing = false;
let resizeObserver: ResizeObserver | undefined;

function loadValue(value: string) {
  if (!editor.value) return;
  const decoded = decodeMemo(value);
  if (decoded.html !== undefined) editor.value.innerHTML = decoded.html;
  else editor.value.textContent = decoded.text || '';
  savedRange = null;
  activeFormats.value = [];
  lastValue = value;
}

function rememberSelection() {
  const selection = window.getSelection();
  const root = editor.value;
  if (!root || !selection?.rangeCount) return;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.startContainer) || !root.contains(range.endContainer)) return;
  savedRange = range.cloneRange();
  activeFormats.value = formats
    .filter(({ command }) => document.queryCommandState(command))
    .map(({ command }) => command);
}

function restoreSelection() {
  const root = editor.value;
  if (!root) return;
  root.focus({ preventScroll: true });
  const selection = window.getSelection();
  if (!selection) return;
  const range = savedRange && root.contains(savedRange.commonAncestorContainer)
    ? savedRange : document.createRange();
  if (range !== savedRange) {
    range.selectNodeContents(root);
    range.collapse(false);
  }
  selection.removeAllRanges();
  selection.addRange(range);
}

function publish() {
  if (composing || !editor.value) return;
  lastValue = encodeMemo(editor.value.innerHTML);
  emit('update:modelValue', lastValue);
  rememberSelection();
}

function applyFormat(command: string) {
  if (composing) return;
  restoreSelection();
  // WebView2's native commands preserve the selection and native undo/redo history.
  document.execCommand('styleWithCSS', false, 'false');
  document.execCommand(command);
  publish();
}

function handleKeydown(event: KeyboardEvent) {
  if (event.isComposing || composing || event.altKey || event.shiftKey || !(event.ctrlKey || event.metaKey)) return;
  const format = formats.find(({ key }) => key && key.toLowerCase() === event.key.toLowerCase());
  if (!format) return;
  event.preventDefault();
  rememberSelection();
  applyFormat(format.command);
}

function handlePaste(event: ClipboardEvent) {
  event.preventDefault();
  const text = event.clipboardData?.getData('text/plain');
  if (!text) return;
  // Paste text only; external HTML, styles and resources never enter the editor.
  document.execCommand('insertText', false, text);
  publish();
}

function startComposition() {
  composing = true;
}

function endComposition() {
  composing = false;
  publish();
}

watch(() => props.modelValue, (value) => {
  // Do not replace the DOM on each keystroke: it would reset the caret and undo stack.
  if (value !== lastValue) loadValue(value);
});

onMounted(() => {
  loadValue(props.modelValue);
  document.addEventListener('selectionchange', rememberSelection);
  resizeObserver = new ResizeObserver(() => {
    if (!editor.value) return;
    const height = Math.round(editor.value.getBoundingClientRect().height);
    if (height !== Math.round(props.height)) emit('resize', height);
  });
  if (editor.value) resizeObserver.observe(editor.value);
});

onBeforeUnmount(() => {
  document.removeEventListener('selectionchange', rememberSelection);
  resizeObserver?.disconnect();
});
</script>

<template>
  <div class="memo-editor">
    <div class="memo-toolbar" role="group" aria-label="备忘文字样式">
      <button
        v-for="format in formats"
        :key="format.command"
        type="button"
        :class="['format-button', format.command, { active: activeFormats.includes(format.command) }]"
        :title="format.label + (format.key ? `（Ctrl+${format.key}）` : '')"
        :aria-label="format.label"
        :aria-keyshortcuts="format.key ? `Control+${format.key}` : undefined"
        :aria-pressed="activeFormats.includes(format.command)"
        @mousedown.prevent
        @click="applyFormat(format.command)"
      >{{ format.symbol }}</button>
      <span class="toolbar-divider" aria-hidden="true"></span>
      <button type="button" class="format-button clear-format" title="清除所选文字格式" aria-label="清除所选文字格式" @mousedown.prevent @click="applyFormat('removeFormat')">清除格式</button>
      <span v-if="saving" class="memo-saving" role="status">保存中...</span>
    </div>
    <div
      ref="editor"
      class="memo-content"
      contenteditable="true"
      role="textbox"
      aria-label="备忘内容"
      aria-multiline="true"
      data-placeholder="在这里记录你的备忘...（自动保存）"
      :style="{ height: height + 'px' }"
      @input="publish"
      @keydown="handleKeydown"
      @paste="handlePaste"
      @drop.prevent
      @compositionstart="startComposition"
      @compositionend="endComposition"
    ></div>
  </div>
</template>

<style scoped>
.memo-editor {
  margin-bottom: 16px;
  min-width: 0;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-primary);
  box-shadow: 0 8px 20px rgba(23, 33, 43, 0.04);
  transition: border-color 0.2s, box-shadow 0.2s;
  overflow: hidden;
}

.memo-editor:focus-within {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.12);
}

.memo-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  padding: 5px 10px;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-secondary);
}

.format-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  height: 28px;
  padding: 0 6px;
  border: 1px solid transparent;
  border-radius: 4px;
  color: var(--text-secondary);
  background: transparent;
  font-family: inherit;
  font-size: 14px;
  cursor: pointer;
}

.format-button:hover,
.format-button.active {
  color: var(--primary-color);
  background: rgba(37, 99, 235, 0.09);
}

.format-button.active { border-color: rgba(37, 99, 235, 0.2); }
.format-button:focus-visible { outline: 2px solid var(--primary-color); outline-offset: 1px; }
.bold { font-weight: 700; }
.italic { font-style: italic; }
.underline { text-decoration: underline; }
.strikeThrough { text-decoration: line-through; }
.clear-format { font-size: 12px; }
.toolbar-divider { width: 1px; height: 16px; margin: 0 4px; background: var(--border-color); }
.memo-saving { margin-left: auto; color: var(--text-secondary); font-size: 12px; }

.memo-content {
  box-sizing: border-box;
  width: 100%;
  min-height: 80px;
  max-height: 200px;
  padding: 14px 16px;
  overflow-y: auto;
  resize: vertical;
  outline: none;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 14px;
  line-height: 1.5;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  scrollbar-width: thin;
}

.memo-content:empty::before {
  content: attr(data-placeholder);
  color: var(--text-secondary);
  opacity: 0.7;
  pointer-events: none;
}

.memo-content :deep(p) { margin: 0; }
</style>
