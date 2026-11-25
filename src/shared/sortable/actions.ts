import { flushAll } from '../../app/session';
import { currentSnapshot, mutate, accept } from '../snapshot-store';
import { bridge } from '../bridge';
import type { ModuleId } from '../types';
import type { SortEntry, SortTarget } from './Sortable';
import { orderedGroups } from '../groups';

export async function commitSort(module: ModuleId, entry: SortEntry, target: SortTarget) {
  await flushAll();
  const snapshot = currentSnapshot();
  const data = snapshot.modules[module].data;
  const rows = entry.kind === 'group' ? orderedGroups(data.groups).filter(g => g.id !== 'ungrouped') : data.items.filter(i => i.groupId === target.groupId);
  const beforeId = entry.kind === 'group' && target.beforeId === 'ungrouped' ? null : target.beforeId;
  if (entry.kind === 'group' && entry.id === 'ungrouped') return false;
  const index = rows.findIndex(i => i.id === entry.id);
  if (beforeId === entry.id || index >= 0 && (rows[index + 1]?.id ?? null) === beforeId) return false;
  try {
    await mutate(module, entry.kind === 'group'
      ? { type: 'move_group', groupId: entry.id, beforeGroupId: beforeId, expectedRevision: snapshot.revision }
      : { type: 'move_item', itemId: entry.id, toGroupId: target.groupId, beforeItemId: beforeId, expectedRevision: snapshot.revision });
    return true;
  } catch (e) {
    if (typeof e === 'object' && e && 'code' in e && e.code === 'STALE_REVISION') accept(await bridge.snapshot());
    throw e;
  }
}

export async function commitFavoriteSort(module: ModuleId, entry: SortEntry, target: SortTarget) {
  await flushAll();
  const snapshot = currentSnapshot();
  const rows = snapshot.modules[module].data.items.filter(i => i.favorited).sort((a, b) => (a.favoriteIndex ?? 0) - (b.favoriteIndex ?? 0));
  const index = rows.findIndex(i => i.id === entry.id);
  if (target.beforeId === entry.id || index >= 0 && (rows[index + 1]?.id ?? null) === target.beforeId) return false;
  try {
    await mutate(module, { type: 'move_favorite', itemId: entry.id, beforeItemId: target.beforeId, expectedRevision: snapshot.revision });
    return true;
  } catch (e) {
    if (typeof e === 'object' && e && 'code' in e && e.code === 'STALE_REVISION') accept(await bridge.snapshot());
    throw e;
  }
}
