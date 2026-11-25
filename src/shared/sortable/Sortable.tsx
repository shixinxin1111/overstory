import { createContext, useContext, useEffect, useRef, useState } from 'react';
import type { ReactNode, PointerEvent as ReactPointerEvent, KeyboardEvent as ReactKeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { motion, useMotionValue } from 'motion/react';
import { springs } from '../motion';

export type SortKind = 'item' | 'group' | 'nav';
export type SortEntry = { id: string; kind: SortKind; label: string; groupId?: string };
export type SortTarget = { groupId?: string; beforeId: string | null };
type Drag = { entry: SortEntry; target: SortTarget | null; keyboard: boolean; width: number };
type SortApi = {
  drag: Drag | null;
  hoverGroup: string | null;
  start: (entry: SortEntry, event: ReactPointerEvent<HTMLElement>) => void;
  key: (entry: SortEntry, event: ReactKeyboardEvent<HTMLElement>) => void;
  suppressClick: (entry: SortEntry) => boolean;
  marker: (kind: SortKind, id: string, groupId?: string) => boolean;
};
const Context = createContext<SortApi | null>(null);
export function SortableScope({ children, onMove, groups = [], orientation = 'vertical', onHoverGroup }: {
  children: ReactNode; onMove: (entry: SortEntry, target: SortTarget) => Promise<void>;
  groups?: { id: string; name: string }[]; orientation?: 'vertical' | 'horizontal';
  onHoverGroup?: (id: string | null) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [hoverGroup, setHoverGroup] = useState<string | null>(null);
  const current = useRef<Drag | null>(null);
  const x = useMotionValue(0); const y = useMotionValue(0);
  const clean = useRef<(() => void) | undefined>(undefined);
  const suppressed = useRef<{ id: string; until: number } | null>(null);
  const callbacks = useRef({ onMove, onHoverGroup }); callbacks.current = { onMove, onHoverGroup };
  const commit = (d: Drag | null) => { if (d?.target) void callbacks.current.onMove(d.entry, d.target); };
  const update = (next: Drag | null) => { current.current = next; setDrag(next); };
  const cancel = () => { clean.current?.(); update(null); setHoverGroup(null); callbacks.current.onHoverGroup?.(null); };
  useEffect(() => {
    const abort = () => { clean.current?.(); current.current = null; setDrag(null); setHoverGroup(null); callbacks.current.onHoverGroup?.(null); };
    const key = (e: KeyboardEvent) => { if (current.current && e.key === 'Escape' && !e.isComposing) { e.preventDefault(); e.stopImmediatePropagation(); abort(); } };
    window.addEventListener('blur', abort); document.addEventListener('keydown', key, true);
    document.addEventListener('overstory:cancel-drag', abort);
    return () => { abort(); window.removeEventListener('blur', abort); document.removeEventListener('keydown', key, true); document.removeEventListener('overstory:cancel-drag', abort); };
  }, []);
  const start = (entry: SortEntry, e: ReactPointerEvent<HTMLElement>) => {
    if (e.button !== 0) return;
    suppressed.current = null;
    const target = e.target as HTMLElement;
    if (target.closest('input,textarea,select,[contenteditable="true"],[data-no-drag],[role="checkbox"],[role="switch"]')) return;
    const button = target.closest('button');
    if (button && !button.hasAttribute('data-row-drag')) return;
    e.stopPropagation(); cancel();
    const handle = e.currentTarget;
    const row = handle.closest<HTMLElement>('[data-sort-id]')!;
    const rect = row.getBoundingClientRect();
    const origin = { x: e.clientX, y: e.clientY };
    let point = { ...origin }; let active = false; let frame = 0; let hoverTimer: ReturnType<typeof setTimeout> | undefined; let hovered: string | null = null;
    const scroll = row.closest<HTMLElement>('[data-scroll]');
    type Hit = { el: HTMLElement; rect: DOMRect };
    let hits: Hit[] = [];
    const measure = () => { hits = [...root.current!.querySelectorAll<HTMLElement>('[data-sort-id],[data-sort-drop],[data-sort-selector],[data-sort-end]')].map(el => ({ el, rect: el.getBoundingClientRect() })); };
    const hitTest = () => {
      let target: SortTarget | null = null;
      let overGroup: string | null = null;
      for (const { el, rect: r } of hits) {
        if (point.x < r.left || point.x > r.right || point.y < r.top || point.y > r.bottom) continue;
        const container = el.closest<HTMLElement>('[data-scroll]');
        if (container) {
          const clip = container.getBoundingClientRect();
          if (point.y < clip.top || point.y > clip.bottom) continue;
        }
        if (el.hasAttribute('data-sort-selector') && entry.kind === 'item') { overGroup = '__chooser__'; continue; }
        const drop = el.dataset.sortDrop;
        if (drop && entry.kind === 'item') { target = { groupId: drop, beforeId: null }; overGroup = drop; continue; }
        if (el.dataset.sortEnd === entry.kind && (entry.kind !== 'item' || el.dataset.sortGroup === entry.groupId || el.dataset.sortGroup)) {
          target = { groupId: el.dataset.sortGroup, beforeId: null }; continue;
        }
        if (el.dataset.sortKind !== entry.kind || el.dataset.sortId === entry.id) continue;
        const after = orientation === 'horizontal' ? point.x > r.left + r.width / 2 : point.y > r.top + r.height / 2;
        const siblings = hits.filter(h => h.el.dataset.sortKind === entry.kind && h.el.dataset.sortGroup === el.dataset.sortGroup && h.el.dataset.sortId !== entry.id);
        const index = siblings.findIndex(h => h.el === el);
        target = { groupId: el.dataset.sortGroup, beforeId: after ? siblings[index + 1]?.el.dataset.sortId ?? null : el.dataset.sortId! };
      }
      if (hovered !== overGroup) {
        hovered = overGroup; clearTimeout(hoverTimer);
        if (overGroup) hoverTimer = setTimeout(() => { setHoverGroup(overGroup); callbacks.current.onHoverGroup?.(overGroup); requestAnimationFrame(measure); }, 350);
      }
      if (current.current && JSON.stringify(current.current.target) !== JSON.stringify(target)) update({ ...current.current, target });
    };
    const tick = () => {
      if (scroll) {
        const r = scroll.getBoundingClientRect();
        const distance = point.y - r.top < 40 ? -(40 - (point.y - r.top)) : r.bottom - point.y < 40 ? 40 - (r.bottom - point.y) : 0;
        if (distance && point.x >= r.left && point.x <= r.right) { scroll.scrollTop += Math.max(-14, Math.min(14, distance / 4)); measure(); hitTest(); }
      }
      frame = requestAnimationFrame(tick);
    };
    const move = (event: PointerEvent) => {
      point = { x: event.clientX, y: event.clientY };
      if (!active && Math.hypot(point.x - origin.x, point.y - origin.y) > 5) {
        active = true; handle.setPointerCapture(e.pointerId);
        suppressed.current = { id: `${entry.kind}:${entry.id}`, until: Infinity };
        update({ entry, target: null, keyboard: false, width: rect.width }); measure(); tick();
      }
      if (!active) return;
      event.preventDefault();
      x.set(rect.left + point.x - origin.x); y.set(rect.top + point.y - origin.y); hitTest();
    };
    const stop = () => {
      window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up); window.removeEventListener('pointercancel', abort);
      scroll?.removeEventListener('scroll', measure); cancelAnimationFrame(frame); clearTimeout(hoverTimer);
      if (handle.hasPointerCapture(e.pointerId)) handle.releasePointerCapture(e.pointerId);
      if (active) suppressed.current = { id: `${entry.kind}:${entry.id}`, until: performance.now() + 450 };
      clean.current = undefined;
    };
    const up = () => { const d = current.current; stop(); update(null); setHoverGroup(null); callbacks.current.onHoverGroup?.(null); if (active) commit(d); };
    const abort = () => { stop(); update(null); setHoverGroup(null); callbacks.current.onHoverGroup?.(null); };
    clean.current = stop;
    x.set(rect.left); y.set(rect.top);
    window.addEventListener('pointermove', move); window.addEventListener('pointerup', up); window.addEventListener('pointercancel', abort);
    scroll?.addEventListener('scroll', measure, { passive: true });
  };
  const key = (entry: SortEntry, e: ReactKeyboardEvent<HTMLElement>) => {
    if (![' ', 'Enter', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) return;
    const d = current.current;
    if (!d && ![' ', 'Enter'].includes(e.key)) return;
    e.preventDefault(); e.stopPropagation();
    if (!d) { update({ entry, target: { groupId: entry.groupId, beforeId: entry.id }, keyboard: true, width: 0 }); return; }
    if (!d.keyboard || d.entry.id !== entry.id) return;
    if (e.key === ' ' || e.key === 'Enter') { update(null); commit(d); return; }
    if (entry.kind === 'item' && groups.length && ['ArrowLeft', 'ArrowRight'].includes(e.key)) {
      const n = groups.findIndex(g => g.id === d.target?.groupId);
      const group = groups[Math.max(0, Math.min(groups.length - 1, n + (e.key === 'ArrowRight' ? 1 : -1)))];
      update({ ...d, target: { groupId: group.id, beforeId: null } }); return;
    }
    const rows = [...root.current!.querySelectorAll<HTMLElement>('[data-sort-id]')].filter(el => el.dataset.sortKind === entry.kind && (entry.kind !== 'item' || el.dataset.sortGroup === d.target?.groupId));
    const n = d.target?.beforeId === null ? rows.length : rows.findIndex(el => el.dataset.sortId === d.target?.beforeId);
    const next = Math.max(0, Math.min(rows.length, n + (['ArrowDown', 'ArrowRight'].includes(e.key) ? 1 : -1)));
    update({ ...d, target: { groupId: d.target?.groupId, beforeId: rows[next]?.dataset.sortId ?? null } });
  };
  const api: SortApi = { drag, hoverGroup, start, key,
    suppressClick: entry => suppressed.current?.id === `${entry.kind}:${entry.id}` && performance.now() < suppressed.current.until,
    marker: (kind, id, groupId) => !!drag && drag.entry.kind === kind && drag.target?.beforeId === id && (kind !== 'item' || drag.target?.groupId === groupId) };
  return <Context.Provider value={api}><div className={`sort-scope ${orientation}`} ref={root}>{children}</div>
    <span className="sr-only" aria-live="assertive">{drag ? `正在移动${drag.entry.label}。方向键换位，左右键切换分组，回车放置，Esc 取消。${drag.target?.groupId ? groups.find(g => g.id === drag.target?.groupId)?.name ?? '' : ''}` : ''}</span>
    {drag && !drag.keyboard ? createPortal(<motion.div className="drag-ghost" style={{ x, y, width: drag.width }} transition={springs.feedback}>{drag.entry.label}</motion.div>, document.body) : null}
  </Context.Provider>;
}
export function SortRow({ entry, children, className = '', disabled = false }: { entry: SortEntry; children: ReactNode; className?: string; disabled?: boolean }) {
  const api = useContext(Context)!;
  return <motion.div layout="position" transition={springs.content} data-sort-id={entry.id} data-sort-kind={entry.kind} data-sort-group={entry.groupId}
    tabIndex={disabled ? undefined : 0} aria-label={`排序：${entry.label}`} aria-keyshortcuts="Space ArrowUp ArrowDown ArrowLeft ArrowRight Enter Escape"
    onPointerDown={e => { if (!disabled) api.start(entry, e); }}
    onKeyDown={e => { if (!disabled && e.target === e.currentTarget) api.key(entry, e); }}
    onClickCapture={e => { if (e.detail > 0 && api.suppressClick(entry)) { e.preventDefault(); e.stopPropagation(); } }}
    className={`sort-row ${disabled ? 'sort-disabled' : ''} ${className} ${api.marker(entry.kind, entry.id, entry.groupId) ? 'insert-before' : ''} ${api.drag?.entry.id === entry.id && api.drag.entry.kind === entry.kind ? 'drag-source' : ''}`}>{children}</motion.div>;
}
export function GroupDrop({ id, children, className = '' }: { id: string; children: ReactNode; className?: string }) {
  const api = useContext(Context)!;
  return <div data-sort-drop={id} className={`${className} ${api.drag?.entry.kind === 'item' && api.drag.target?.groupId === id && api.drag.target.beforeId === null ? 'drop-active' : ''}`}>{children}</div>;
}
export const useDrag = () => useContext(Context)?.drag ?? null;
export const useSortHover = () => useContext(Context)?.hoverGroup ?? null;
export function EndDrop({ kind, groupId }: { kind: SortKind; groupId?: string }) {
  const api = useContext(Context)!;
  const active = api.drag?.entry.kind === kind && api.drag.target?.beforeId === null && (kind !== 'item' || api.drag.target.groupId === groupId);
  return <div data-sort-end={kind} data-sort-group={groupId} className={`sort-end ${active ? 'insert-before' : ''}`}/>;
}
