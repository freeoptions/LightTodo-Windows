<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useSettings } from '../composables/useSettings';
import ShortcutEditor from './ShortcutEditor.vue';

const emit = defineEmits<{
  (e: 'shortcuts-changed', shortcuts: { showHideWindow: string }): void;
}>();

const { settings, loading, error, loadSettings, updateShortcut, updatePriorityColor, updateAutoLaunch } = useSettings();

const savingShortcut = ref<string | null>(null);
const savingAutoLaunch = ref(false);

onMounted(() => {
  loadSettings();
});

const handleShortcutChange = async (action: string, value: string) => {
  savingShortcut.value = action;
  try {
    await updateShortcut(action, value);
    // Emit event to notify parent to re-register shortcuts
    if (settings.value) {
      emit('shortcuts-changed', settings.value.shortcuts);
    }
  } catch (e) {
    // Revert on error
    await loadSettings();
    alert('快捷键注册失败: ' + (e as Error).message);
  } finally {
    savingShortcut.value = null;
  }
};

const handlePriorityColorChange = async (priority: 1 | 2 | 3, color: string) => {
  try {
    await updatePriorityColor(priority, color);
  } catch (e) {
    // Revert on error
    await loadSettings();
    alert('颜色保存失败: ' + (e as Error).message);
  }
};

const handleAutoLaunchChange = async (event: Event) => {
  const enabled = (event.target as HTMLInputElement).checked;
  savingAutoLaunch.value = true;
  try {
    await updateAutoLaunch(enabled);
  } catch (e) {
    await loadSettings();
    alert('开机自启设置保存失败: ' + (e as Error).message);
  } finally {
    savingAutoLaunch.value = false;
  }
};

const formatShortcut = (shortcut: string): string => {
  if (!shortcut.trim()) return '未设置';

  return shortcut
    .split('+')
    .map(part => {
      const partUpper = part.toUpperCase();
      const displayNames: Record<string, string> = {
        'COMMANDORCONTROL': 'Ctrl',
        'CTRL': 'Ctrl',
        'ALT': 'Alt',
        'SHIFT': 'Shift',
        'SUPER': 'Win',
        'SPACE': 'Space',
        'DIGIT0': '0',
        'DIGIT1': '1',
        'DIGIT2': '2',
        'DIGIT3': '3',
        'DIGIT4': '4',
        'DIGIT5': '5',
        'DIGIT6': '6',
        'DIGIT7': '7',
        'DIGIT8': '8',
        'DIGIT9': '9',
      };
      return displayNames[partUpper] || partUpper;
    })
    .join(' + ');
};
</script>

<template>
  <div class="settings-container">
    <h2 class="settings-title">快捷键设置</h2>

    <div v-if="loading" class="loading">
      加载中...
    </div>

    <div v-else-if="error" class="error">
      加载失败: {{ error }}
    </div>

    <div v-else-if="settings" class="settings-content">
      <div class="settings-section">
        <h3 class="section-title">启动设置</h3>
        <p class="section-description">
          控制 LightTodo 是否在 Windows 登录后自动启动，默认开启。
        </p>

        <div class="setting-item setting-item-inline">
          <div class="toggle-content">
            <label class="setting-label" for="auto-launch">开机自启</label>
            <p class="setting-hint">开启后会在系统登录时自动启动 LightTodo。</p>
          </div>

          <div class="toggle-control">
            <label class="switch">
              <input
                id="auto-launch"
                type="checkbox"
                :checked="settings.autoLaunch"
                :disabled="savingAutoLaunch"
                @change="handleAutoLaunchChange"
              />
              <span class="switch-slider"></span>
            </label>
            <span v-if="savingAutoLaunch" class="saving-hint">保存中...</span>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="section-title">全局快捷键</h3>
        <p class="section-description">
          这些快捷键在应用外也能使用，可以快速控制窗口
        </p>

        <div class="setting-item">
          <label class="setting-label">显示/隐藏窗口</label>
          <div class="setting-control">
            <ShortcutEditor
              :model-value="settings.shortcuts.showHideWindow"
              action="showHideWindow"
              @update:model-value="handleShortcutChange('showHideWindow', $event)"
            />
            <span v-if="savingShortcut === 'showHideWindow'" class="saving-hint">
              保存中...
            </span>
          </div>
          <p class="setting-hint">
            当前: <kbd>{{ formatShortcut(settings.shortcuts.showHideWindow) }}</kbd>
          </p>
        </div>
      </div>

      <div class="settings-section">
        <h3 class="section-title">快捷键说明</h3>
        <ul class="shortcut-help">
          <li><kbd>Ctrl</kbd> - 控制键</li>
          <li><kbd>Alt</kbd> - 替代键</li>
          <li><kbd>Shift</kbd> - 上档键</li>
          <li><kbd>Win</kbd> - Windows键</li>
          <li>使用 <code>+</code> 组合按键，例如: <code>ctrl+shift+alt+1</code></li>
        </ul>
      </div>

      <div class="settings-section">
        <h3 class="section-title">优先级颜色</h3>
        <p class="section-description">
          设置不同优先级的待办显示颜色
        </p>

        <div class="priority-colors">
          <div class="priority-color-item">
            <label class="priority-label">优先级 1</label>
            <input
              type="color"
              :value="settings?.priorityColors?.['1'] || '#ef4444'"
              @input="(e) => handlePriorityColorChange(1, (e.target as HTMLInputElement).value)"
              class="color-picker"
            />
          </div>
          <div class="priority-color-item">
            <label class="priority-label">优先级 2</label>
            <input
              type="color"
              :value="settings?.priorityColors?.['2'] || '#f59e0b'"
              @input="(e) => handlePriorityColorChange(2, (e.target as HTMLInputElement).value)"
              class="color-picker"
            />
          </div>
          <div class="priority-color-item">
            <label class="priority-label">优先级 3</label>
            <input
              type="color"
              :value="settings?.priorityColors?.['3'] || '#22c55e'"
              @input="(e) => handlePriorityColorChange(3, (e.target as HTMLInputElement).value)"
              class="color-picker"
            />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-container {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.settings-title {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
  margin: 0;
}

.loading, .error {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary, #6c757d);
}

.error {
  color: var(--danger-color, #ef4444);
}

.settings-content {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary, #1a1a1a);
  margin: 0;
}

.section-description {
  font-size: 13px;
  color: var(--text-secondary, #6c757d);
  margin: 0;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-secondary, #f8f9fa);
  border-radius: 8px;
}

.setting-item-inline {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary, #1a1a1a);
}

.setting-control {
  display: flex;
  align-items: center;
  gap: 12px;
}

.setting-hint {
  font-size: 12px;
  color: var(--text-secondary, #6c757d);
  margin: 0;
}

.toggle-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 6px;
}

.toggle-control {
  display: flex;
  align-items: center;
  gap: 10px;
}

.switch {
  position: relative;
  display: inline-flex;
  width: 48px;
  height: 28px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.switch-slider {
  position: absolute;
  inset: 0;
  cursor: pointer;
  background: #cbd5e1;
  border-radius: 999px;
  transition: background 0.2s ease;
}

.switch-slider::before {
  content: '';
  position: absolute;
  left: 3px;
  top: 3px;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #ffffff;
  box-shadow: 0 2px 6px rgba(15, 23, 42, 0.2);
  transition: transform 0.2s ease;
}

.switch input:checked + .switch-slider {
  background: #22c55e;
}

.switch input:checked + .switch-slider::before {
  transform: translateX(20px);
}

.switch input:disabled + .switch-slider {
  cursor: not-allowed;
  opacity: 0.7;
}

kbd {
  display: inline-block;
  padding: 2px 6px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  background: var(--bg-primary, #ffffff);
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 4px;
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
}

code {
  padding: 2px 6px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 12px;
  background: var(--bg-primary, #ffffff);
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 4px;
}

.saving-hint {
  font-size: 12px;
  color: var(--primary-color, #4f46e5);
}

.shortcut-help {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.shortcut-help li {
  font-size: 13px;
  color: var(--text-secondary, #6c757d);
  display: flex;
  align-items: center;
  gap: 8px;
}

.priority-colors {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.priority-color-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-secondary, #f8f9fa);
  border-radius: 8px;
  min-width: 100px;
}

.priority-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary, #1a1a1a);
}

.color-picker {
  width: 60px;
  height: 40px;
  border: 1px solid var(--border-color, #dee2e6);
  border-radius: 6px;
  cursor: pointer;
  background: transparent;
}

.color-picker::-webkit-color-swatch-wrapper {
  padding: 0;
}

.color-picker::-webkit-color-swatch {
  border: none;
  border-radius: 4px;
}

@media (prefers-color-scheme: dark) {
  .priority-color-item {
    background: #2a2a2a;
  }

  .color-picker {
    border-color: #495057;
  }
}

@media (prefers-color-scheme: dark) {
  .setting-item {
    background: #2a2a2a;
  }

  kbd, code {
    background: #1e1e1e;
    border-color: #495057;
  }

  .switch-slider {
    background: #475569;
  }
}

@media (max-width: 640px) {
  .setting-item-inline {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
