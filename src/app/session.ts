import { useSyncExternalStore } from 'react';
import { defaultSession } from '../shared/types';
import type { Session } from '../shared/types';
import { settleMutations } from '../shared/snapshot-store';

let session = defaultSession();
const subscribers = new Set<() => void>();
export const currentSession = () => session;
export function setSession(update: Partial<Session>) {
  session = { ...session, ...update }; subscribers.forEach(fn => fn());
}
export function restoreSession(next: Session) { setSession({ ...defaultSession(), ...next }); }
export const useSession = () => useSyncExternalStore(fn => { subscribers.add(fn); return () => subscribers.delete(fn); }, currentSession);
const draftFlushers = new Set<() => Promise<void>>();
export function registerDraftFlush(flush: () => Promise<void>) { draftFlushers.add(flush); return () => { draftFlushers.delete(flush); }; }
export async function flushAll() {
  for (const flush of draftFlushers) await flush();
  await settleMutations();
}
export async function navigate(update: Partial<Session>) { await flushAll(); setSession(update); }
