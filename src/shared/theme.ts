import { emit } from '@tauri-apps/api/event';
import { native } from './bridge';

export type ThemeMode = 'light' | 'dark';
export type ThemeHue = 'gray' | 'yellow' | 'red' | 'green' | 'blue' | 'purple';
export type ThemeId = `${ThemeMode}-${ThemeHue}`;

const labels: Record<ThemeHue, string> = { gray: '灰', yellow: '黄', red: '红', green: '绿', blue: '蓝', purple: '紫' };
const dots: Record<ThemeHue, string> = { gray: '#8e8e93', yellow: '#ffd60a', red: '#ff453a', green: '#30d158', blue: '#0a84ff', purple: '#bf5af2' };
const surfaces: Record<ThemeMode, string> = { light: '#fff', dark: '#2c2c2e' };
const modes: ThemeMode[] = ['light', 'dark'];
const hues: ThemeHue[] = ['gray', 'yellow', 'red', 'green', 'blue', 'purple'];

export const themes = modes.flatMap(mode => hues.map(hue => ({
  id: `${mode}-${hue}` as ThemeId,
  mode,
  label: `${mode === 'light' ? '浅' : '深'}${labels[hue]}`,
  swatchBg: surfaces[mode],
  swatchDot: dots[hue],
})));

export const defaultTheme: ThemeId = 'dark-gray';
const storageKey = 'overstory.theme';

export function readTheme(): ThemeId {
  const value = localStorage.getItem(storageKey);
  return themes.some(theme => theme.id === value) ? value as ThemeId : defaultTheme;
}

export const applyTheme = (theme: ThemeId) => { document.documentElement.dataset.theme = theme; };

export async function selectTheme(theme: ThemeId) {
  applyTheme(theme);
  localStorage.setItem(storageKey, theme);
  if (native) await emit('overstory:theme', theme);
}
