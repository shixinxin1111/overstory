import { useSyncExternalStore } from 'react';
import { bridge, on } from './bridge';
import { emptySnapshot } from './types';
import type { Snapshot } from './types';

let snapshot = emptySnapshot();
const subscribers = new Set<() => void>();
export function accept(next: Snapshot) {
  if (next.revision < snapshot.revision) return;
  snapshot = next; subscribers.forEach(fn => fn());
}
export const currentSnapshot = () => snapshot;
export const useSnapshot = () => useSyncExternalStore(fn => { subscribers.add(fn); return () => subscribers.delete(fn); }, currentSnapshot);
let initializing: Promise<void> | undefined;
export function initialize() {
  return initializing ??= (async () => {
    await on<Snapshot>('app:snapshot-changed', accept);
    accept(await bridge.snapshot());
  })();
}
let queue: Promise<unknown> = Promise.resolve();
export function mutate(moduleId: string, action: Record<string, unknown>) {
  const task = queue.catch(() => {}).then(async () => {
    const result = await bridge.action(moduleId, action);
    accept(result); return result;
  });
  queue = task; return task;
}
export async function settleMutations() { await queue.catch(() => {}); }
