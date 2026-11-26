import { useSyncExternalStore } from 'react';
import type { SortEntry, SortKind, SortTarget } from './Sortable';

export type SortPreview = { entry: SortEntry; target: SortTarget | null };
let preview: SortPreview | null = null;
const subscribers = new Set<() => void>();
export function setSortPreview(next: SortPreview | null) {
  if (preview === next) return;
  preview = next; subscribers.forEach(fn => fn());
}
export const currentSortPreview = () => preview;
export const useSortPreview = () => useSyncExternalStore(fn => { subscribers.add(fn); return () => subscribers.delete(fn); }, () => preview);

export function previewOrder<T extends { id: string }>(rows: T[], current: SortPreview | null, kind: SortKind): T[] {
  if (!current?.target || current.entry.kind !== kind) return rows;
  const index = rows.findIndex(row => row.id === current.entry.id);
  if (index < 0) return rows;
  const rest = rows.filter(row => row.id !== current.entry.id);
  const at = current.target.beforeId ? rest.findIndex(row => row.id === current.target!.beforeId) : rest.length;
  if (at < 0) return rows;
  rest.splice(at, 0, rows[index]);
  return rest;
}

export function previewGroupedItems<T extends { id: string; groupId?: string }>(items: T[], current: SortPreview | null): T[] {
  if (!current?.target || current.entry.kind !== 'item') return items;
  const index = items.findIndex(item => item.id === current.entry.id);
  if (index < 0) return items;
  const target = current.target;
  const moved = items[index];
  if (target.groupId && moved.groupId && target.groupId !== moved.groupId) return items;
  const rest = items.filter(item => item.id !== moved.id);
  let at: number;
  if (target.beforeId) {
    at = rest.findIndex(item => item.id === target.beforeId);
    if (at < 0) return items;
  } else if (target.groupId) {
    let last = -1;
    rest.forEach((item, i) => { if (item.groupId === target.groupId) last = i; });
    at = last + 1;
  } else {
    at = rest.length;
  }
  rest.splice(at, 0, moved);
  return rest;
}
