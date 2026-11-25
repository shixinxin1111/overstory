import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Snapshot, Presentation, Preferences, Session, CliStatus, SkillStatus } from './types';

export const native = isTauri();
export async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (native) return invoke<T>(command, args);
  if (!import.meta.env.DEV) throw new Error('请在 Overstory 桌面应用中打开');
  const { previewInvoke } = await import('./preview');
  return previewInvoke<T>(command, args);
}
export async function on<T>(event: string, handler: (payload: T) => void) {
  if (native) return listen<T>(event, e => handler(e.payload));
  const listener = (e: Event) => handler((e as CustomEvent<T>).detail);
  window.addEventListener(event, listener);
  return () => window.removeEventListener(event, listener);
}
export const bridge = {
  cliStatus: () => call<CliStatus>('cli_status'),
  manageCli: (operation: 'install' | 'upgrade' | 'uninstall') => call<CliStatus>('manage_cli', { operation }),
  skillStatus: () => call<SkillStatus>('skill_status'),
  manageSkill: (operation: 'install' | 'upgrade' | 'uninstall') => call<SkillStatus>('manage_skill', { operation }),
  snapshot: () => call<Snapshot>('get_snapshot'),
  action: (moduleId: string, action: Record<string, unknown>) => call<Snapshot>('execute_module_action', { moduleId, action }),
  preferences: (preferences: Preferences, expectedRevision: number) => call<Snapshot>('update_preferences', { preferences, expectedRevision }),
  presentation: (presentation: Presentation) => call<void>('request_presentation', { presentation }),
  acknowledge: (token: string, session: Session, success: boolean) => call<void>('acknowledge_transition', { token, session, success }),
};
