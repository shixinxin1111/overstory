import { useSyncExternalStore } from 'react';
import { currentSnapshot, mutate, accept } from '../../shared/snapshot-store';
import { bridge } from '../../shared/bridge';
import { registerDraftFlush } from '../../app/session';

type Draft = { id: string; title: string; content: string; baseVersion: number; generation: number; status: 'dirty' | 'saving' | 'saved' | 'failed'; error?: string; conflict?: boolean };
let draft: Draft | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;
let saving: Promise<void> | null = null;
const listeners = new Set<() => void>();
const notify = () => listeners.forEach(fn => fn());
export const useDraft = () => useSyncExternalStore(fn => { listeners.add(fn); return () => listeners.delete(fn); }, () => draft?.status === 'saved' ? null : draft);
export function editDraft(id: string, field: 'title' | 'content', value: string) {
  const item = currentSnapshot().modules.notes.data.items.find(i => i.id === id);
  if (!item) return;
  draft = { ...(draft?.id === id && draft.status !== 'saved' ? draft : { id, title: item.title, content: item.content ?? '', baseVersion: item.version ?? 0, generation: 0, status: 'saved' }), [field]: value, generation: (draft?.generation ?? 0) + 1, status: 'dirty' };
  notify(); clearTimeout(timer);
  timer = setTimeout(() => { void flushDraft().catch(() => {}); }, 250);
}
export async function flushDraft(): Promise<void> {
  clearTimeout(timer);
  if (saving) { await saving; if (draft?.status === 'dirty') return flushDraft(); return; }
  if (!draft || draft.status === 'saved') return;
  const pending = { ...draft };
  draft = { ...draft, status: 'saving' }; notify();
  saving = (async () => {
    try {
      const result = await mutate('notes', { type: 'update', id: pending.id, title: pending.title, content: pending.content, expectedItemVersion: pending.baseVersion });
      const saved = result.modules.notes.data.items.find(i => i.id === pending.id);
      if (draft?.id === pending.id && saved) {
        draft = { ...draft, baseVersion: saved.version ?? 0, error: undefined, conflict: false,
          ...(draft.generation === pending.generation ? { title: saved.title, status: 'saved' as const } : {}) };
        notify();
      }
    } catch (e) {
      if (draft?.id === pending.id) { draft = { ...draft, status: 'failed', error: typeof e === 'object' && e && 'message' in e ? String(e.message) : String(e), conflict: typeof e === 'object' && e !== null && 'code' in e && e.code === 'ITEM_CONFLICT' }; notify(); }
      throw e;
    } finally { saving = null; }
  })();
  await saving;
  if (draft?.status === 'dirty') await flushDraft();
}
export function discardDeletedDraft(id: string) { if (draft?.id === id) { clearTimeout(timer); draft = null; notify(); } }
export function resetDraft() { clearTimeout(timer); draft = null; notify(); }
export async function overwriteDraft() {
  if (!draft) return;
  accept(await bridge.snapshot());
  const item = currentSnapshot().modules.notes.data.items.find(i => i.id === draft?.id);
  if (!item) throw new Error('便签已删除，草稿仍保留，请先复制正文。');
  draft = { ...draft, baseVersion: item.version ?? 0, status: 'dirty', conflict: false };
  notify(); await flushDraft();
}
export async function useLatestNote() { accept(await bridge.snapshot()); resetDraft(); }
registerDraftFlush(flushDraft);
