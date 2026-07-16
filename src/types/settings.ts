export interface ShortcutConfig {
  showHideWindow: string;
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
  autoLaunch: boolean;
  priorityColors?: {
    1?: string;
    2?: string;
    3?: string;
  };
  backgroundWallpaper?: string | null;
}

export type ShortcutAction = 'showHideWindow';
