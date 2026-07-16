<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useSettings } from '../composables/useSettings';
import ShortcutEditor from './ShortcutEditor.vue';

const emit = defineEmits<{
  (e: 'shortcuts-changed', shortcuts: { showHideWindow: string }): void;
  (e: 'wallpaper-changed', wallpaper: string | null): void;
}>();

const {
  settings,
  loading,
  error,
  loadSettings,
  updateShortcut,
  updatePriorityColor,
  updateAutoLaunch,
  updateBackgroundWallpaper,
} = useSettings();

const savingShortcut = ref<string | null>(null);
const savingAutoLaunch = ref(false);
const savingWallpaper = ref(false);
const wallpaperInputRef = ref<HTMLInputElement | null>(null);

const MAX_WALLPAPER_BYTES = 8 * 1024 * 1024;

onMounted(() => {
  loadSettings();
});

const handleShortcutChange = async (action: string, value: string) => {
  savingShortcut.value = action;
  try {
    await updateShortcut(action, value);
    if (settings.value) {
      emit('shortcuts-changed', settings.value.shortcuts);
    }
  } catch (e) {
    await loadSettings();
    alert('快捷键保存失败：' + (e as Error).message);
  } finally {
    savingShortcut.value = null;
  }
};

const handlePriorityColorChange = async (priority: 1 | 2 | 3, color: string) => {
  try {
    await updatePriorityColor(priority, color);
  } catch (e) {
    await loadSettings();
    alert('颜色保存失败：' + (e as Error).message);
  }
};

const handleAutoLaunchChange = async (event: Event) => {
  const enabled = (event.target as HTMLInputElement).checked;
  savingAutoLaunch.value = true;
  try {
    await updateAutoLaunch(enabled);
  } catch (e) {
    await loadSettings();
    alert('开机自启设置保存失败：' + (e as Error).message);
  } finally {
    savingAutoLaunch.value = false;
  }
};

const handleWallpaperSelect = async (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;

  if (!file.type.startsWith('image/')) {
    alert('请选择图片文件');
    input.value = '';
    return;
  }

  if (file.size > MAX_WALLPAPER_BYTES) {
    alert('壁纸图片不能超过 8MB');
    input.value = '';
    return;
  }

  savingWallpaper.value = true;
  try {
    const wallpaper = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ''));
      reader.onerror = () => reject(reader.error || new Error('读取图片失败'));
      reader.readAsDataURL(file);
    });

    await updateBackgroundWallpaper(wallpaper);
    emit('wallpaper-changed', wallpaper);
  } catch (e) {
    await loadSettings();
    alert('壁纸保存失败：' + (e as Error).message);
  } finally {
    savingWallpaper.value = false;
    input.value = '';
  }
};

const clearWallpaper = async () => {
  savingWallpaper.value = true;
  try {
    await updateBackgroundWallpaper(null);
    emit('wallpaper-changed', null);
  } catch (e) {
    await loadSettings();
    alert('壁纸移除失败：' + (e as Error).message);
  } finally {
    savingWallpaper.value = false;
  }
};

const formatShortcut = (shortcut: string): string => {
  if (!shortcut.trim()) return '未设置';

  return shortcut
    .split('+')
    .map(part => {
      const partUpper = part.toUpperCase();
      const displayNames: Record<string, string> = {
        COMMANDORCONTROL: 'Ctrl',
        CTRL: 'Ctrl',
        ALT: 'Alt',
        SHIFT: 'Shift',
        SUPER: 'Win',
        SPACE: 'Space',
        DIGIT0: '0',
        DIGIT1: '1',
        DIGIT2: '2',
        DIGIT3: '3',
        DIGIT4: '4',
        DIGIT5: '5',
        DIGIT6: '6',
        DIGIT7: '7',
        DIGIT8: '8',
        DIGIT9: '9',
      };
      return displayNames[partUpper] || partUpper;
    })
    .join(' + ');
};
</script>

<template>
  <div class="settings-container">
    <h2 class="settings-title">设置</h2>

    <div v-if="loading" class="loading">加载中...</div>

    <div v-else-if="error" class="error">加载失败：{{ error }}</div>

    <div v-else-if="settings" class="settings-content">
      <section class="settings-section">
        <h3 class="section-title">启动设置</h3>
        <div class="setting-item setting-item-inline">
          <div class="toggle-content">
            <label class="setting-label" for="auto-launch">开机自启</label>
            <p class="setting-hint">开启后会在 Windows 登录时自动启动 LightTodo。</p>
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
      </section>

      <section class="settings-section">
        <h3 class="section-title">背景壁纸</h3>
        <div class="setting-item wallpaper-setting">
          <div class="wallpaper-preview" :class="{ empty: !settings.backgroundWallpaper }">
            <img v-if="settings.backgroundWallpaper" :src="settings.backgroundWallpaper" alt="当前壁纸预览" />
            <span v-else>未设置壁纸</span>
          </div>
          <div class="wallpaper-actions">
            <input
              ref="wallpaperInputRef"
              class="hidden-file-input"
              type="file"
              accept="image/*"
              @change="handleWallpaperSelect"
            />
            <button class="btn-secondary" type="button" :disabled="savingWallpaper" @click="wallpaperInputRef?.click()">
              选择图片
            </button>
            <button
              class="btn-secondary danger"
              type="button"
              :disabled="savingWallpaper || !settings.backgroundWallpaper"
              @click="clearWallpaper"
            >
              移除壁纸
            </button>
            <span v-if="savingWallpaper" class="saving-hint">保存中...</span>
          </div>
          <p class="setting-hint">支持 JPG、PNG、WebP 等图片，最大 8MB。图片会写入便携配置文件，移动 exe 后仍可显示。</p>
        </div>
      </section>

      <section class="settings-section">
        <h3 class="section-title">全局快捷键</h3>
        <div class="setting-item">
          <label class="setting-label">显示 / 隐藏窗口</label>
          <div class="setting-control">
            <ShortcutEditor
              :model-value="settings.shortcuts.showHideWindow"
              action="showHideWindow"
              @update:model-value="handleShortcutChange('showHideWindow', $event)"
            />
            <span v-if="savingShortcut === 'showHideWindow'" class="saving-hint">保存中...</span>
          </div>
          <p class="setting-hint">当前：<kbd>{{ formatShortcut(settings.shortcuts.showHideWindow) }}</kbd></p>
        </div>
      </section>

      <section class="settings-section">
        <h3 class="section-title">快捷键说明</h3>
        <ul class="shortcut-help">
          <li><kbd>Ctrl</kbd> 控制键</li>
          <li><kbd>Alt</kbd> 替代键</li>
          <li><kbd>Shift</kbd> 上档键</li>
          <li><kbd>Win</kbd> Windows 键</li>
          <li>使用 <code>+</code> 组合按键，例如 <code>ctrl+shift+alt+1</code></li>
        </ul>
      </section>

      <section class="settings-section">
        <h3 class="section-title">优先级颜色</h3>
        <div class="priority-colors">
          <div class="priority-color-item">
            <label class="priority-label">优先级 1</label>
            <input
              type="color"
              :value="settings?.priorityColors?.['1'] || '#ef4444'"
              class="color-picker"
              @input="(e) => handlePriorityColorChange(1, (e.target as HTMLInputElement).value)"
            />
          </div>
          <div class="priority-color-item">
            <label class="priority-label">优先级 2</label>
            <input
              type="color"
              :value="settings?.priorityColors?.['2'] || '#f59e0b'"
              class="color-picker"
              @input="(e) => handlePriorityColorChange(2, (e.target as HTMLInputElement).value)"
            />
          </div>
          <div class="priority-color-item">
            <label class="priority-label">优先级 3</label>
            <input
              type="color"
              :value="settings?.priorityColors?.['3'] || '#22c55e'"
              class="color-picker"
              @input="(e) => handlePriorityColorChange(3, (e.target as HTMLInputElement).value)"
            />
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.settings-container {
  display: flex;
  flex-direction: column;
  gap: 22px;
}

.settings-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary, #17212b);
  margin: 0;
}

.loading,
.error {
  padding: 20px;
  text-align: center;
  color: var(--text-secondary, #65717f);
}

.error {
  color: var(--danger-color, #ef4444);
}

.settings-content,
.settings-section {
  display: flex;
  flex-direction: column;
}

.settings-content {
  gap: 22px;
}

.settings-section {
  gap: 12px;
}

.section-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary, #17212b);
  margin: 0;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-secondary, #f4f6f8);
  border: 1px solid rgba(216, 224, 231, 0.75);
  border-radius: 8px;
}

.setting-item-inline {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.setting-label,
.priority-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #17212b);
}

.setting-hint {
  font-size: 12px;
  color: var(--text-secondary, #65717f);
  margin: 0;
  line-height: 1.45;
}

.setting-control,
.toggle-control,
.wallpaper-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.toggle-content {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 6px;
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

.wallpaper-setting {
  gap: 12px;
}

.wallpaper-preview {
  width: 100%;
  height: 118px;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--border-color, #d8e0e7);
  background: var(--bg-primary, #ffffff);
}

.wallpaper-preview.empty {
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary, #65717f);
  font-size: 13px;
}

.wallpaper-preview img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.hidden-file-input {
  display: none;
}

.btn-secondary {
  height: 32px;
  padding: 0 12px;
  border: 1px solid var(--border-color, #d8e0e7);
  border-radius: 7px;
  background: var(--bg-primary, #ffffff);
  color: var(--text-primary, #17212b);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.btn-secondary:hover:not(:disabled) {
  border-color: var(--primary-color, #2563eb);
  color: var(--primary-color, #2563eb);
}

.btn-secondary.danger:hover:not(:disabled) {
  border-color: var(--danger-color, #ef4444);
  color: var(--danger-color, #ef4444);
}

.btn-secondary:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

kbd,
code {
  display: inline-block;
  padding: 2px 6px;
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  background: var(--bg-primary, #ffffff);
  border: 1px solid var(--border-color, #d8e0e7);
  border-radius: 4px;
}

.saving-hint {
  font-size: 12px;
  color: var(--primary-color, #2563eb);
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
  color: var(--text-secondary, #65717f);
}

.priority-colors {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.priority-color-item {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  background: var(--bg-secondary, #f4f6f8);
  border-radius: 8px;
  min-width: 100px;
}

.color-picker {
  width: 60px;
  height: 40px;
  border: 1px solid var(--border-color, #d8e0e7);
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

@media (max-width: 640px) {
  .setting-item-inline {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
