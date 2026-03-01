export interface ShortcutConfig {
  showHideWindow: string;
  toggleFloating: string;
}

export interface WindowState {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
}

export interface Settings {
  shortcuts: ShortcutConfig;
  window?: WindowState;
  memo: string;
  memoHeight: number;
}

export type ShortcutAction = 'showHideWindow' | 'toggleFloating';
