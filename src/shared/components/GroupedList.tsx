import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { motion } from 'motion/react';
import { springs } from '../motion';
import type { Group, Item, ModuleId } from '../types';
import { currentSession, setSession, useSession } from '../../app/session';
import { GroupDrop, SortRow, EndDrop, useDrag, useSortHover } from '../sortable/Sortable';
import { Button, Dialog, IconButton, Confirm, report } from './Controls';
import { Icon } from '../icons';
import { mutate } from '../snapshot-store';
import { orderedGroups } from '../groups';
import { RowMenu } from './RowMenu';

export function GroupChooser({ module, groups }: { module: 'bookmarks' | 'notes'; groups: Group[] }) {
  const session = useSession();
  const selected = session.selectedGroupIds[module];
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const hover = useSortHover();
  const drag = useDrag();
  const expanded = open || hover === '__chooser__' && !!drag;
  useEffect(() => {
    if (!open) return;
    const outside = (e: PointerEvent) => { if (!root.current?.contains(e.target as Node)) setOpen(false); };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') { e.stopImmediatePropagation(); setOpen(false); root.current?.querySelector<HTMLButtonElement>('.group-select')?.focus(); }
      if (['ArrowUp', 'ArrowDown'].includes(e.key)) {
        e.preventDefault(); const buttons = [...root.current!.querySelectorAll<HTMLButtonElement>('.group-options button')];
        const n = buttons.indexOf(document.activeElement as HTMLButtonElement);
        buttons[Math.max(0, Math.min(buttons.length - 1, n + (e.key === 'ArrowDown' ? 1 : -1)))]?.focus();
      }
    };
    document.addEventListener('pointerdown', outside); document.addEventListener('keydown', key, true);
    return () => { document.removeEventListener('pointerdown', outside); document.removeEventListener('keydown', key, true); };
  }, [open]);
  return <div className="group-chooser" ref={root} data-sort-selector>
    <Button className="group-select" aria-expanded={expanded} onClick={() => setOpen(!open)}><Icon name="folder" size={18}/><span className="group-label">{groups.find(g => g.id === selected)?.name ?? '全部分组'}</span><Icon name="chevron" size={14} style={{ transform: 'rotate(90deg)' }}/></Button>
    {expanded ? <div className="group-options" data-modal={!drag || undefined}>{[{ id: 'all', name: '全部分组' }, ...orderedGroups(groups)].map(g => g.id === 'all' ? <button key={g.id} onClick={() => { setSession({ selectedGroupIds: { ...currentSession().selectedGroupIds, [module]: g.id } }); setOpen(false); }}>全部分组</button> : <GroupDrop id={g.id} key={g.id}><button onClick={() => { if (drag) return; setSession({ selectedGroupIds: { ...currentSession().selectedGroupIds, [module]: g.id } }); setOpen(false); }}>{g.name}<span>{selected === g.id ? '✓' : ''}</span></button></GroupDrop>)}</div> : null}
  </div>;
}
export function GroupedList({ module, groups, items, search, temporaryGroup, renderItem }: {
  module: 'bookmarks' | 'notes'; groups: Group[]; items: Item[]; search: string; temporaryGroup: string | null; renderItem: (item: Item) => ReactNode;
}) {
  const session = useSession();
  const selected = session.selectedGroupIds[module];
  const shown = orderedGroups(groups).filter(g => selected === 'all' || g.id === selected);
  return <>{shown.map(group => {
    const entry = { id: group.id, kind: 'group' as const, label: group.name };
    const key = `${module}:${group.id}`;
    const collapsed = session.collapsed[key] && temporaryGroup !== group.id && !search;
    const groupItems = items.filter(i => i.groupId === group.id);
    return <motion.section layout="position" transition={springs.content} className="group-section" key={group.id}>
      <SortRow entry={entry} disabled={!!search || group.id === 'ungrouped'} className="group-row"><GroupDrop id={group.id} className="group-title-drop"><button data-row-drag className="group-heading" aria-expanded={!collapsed} onClick={() => setSession({ collapsed: { ...currentSession().collapsed, [key]: !session.collapsed[key] } })}><Icon name="chevron" size={14} style={{ transform: `rotate(${collapsed ? 0 : 90}deg)` }}/><span>{group.name}</span><small>{groupItems.length}</small></button></GroupDrop><GroupActions module={module} group={group}/></SortRow>
      {!collapsed ? <div className="group-items">{groupItems.map(renderItem)}{!groupItems.length ? <GroupDrop id={group.id} className="group-empty">{search ? '无匹配项' : '空分组'}</GroupDrop> : null}<EndDrop kind="item" groupId={group.id}/></div> : null}
    </motion.section>;
  })}<EndDrop kind="group"/></>;
}
export function GroupActions({ module, group }: { module: ModuleId; group?: Group }) {
  const [mode, setMode] = useState<'name' | 'delete' | null>(null);
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  if (group?.id === 'ungrouped') return <span className="group-action-spacer"/>;
  return <>
    {group ? <RowMenu title={`分类：${group.name}`} onEdit={() => { setName(group.name); setMode('name'); }} onDelete={() => setMode('delete')}/> : <IconButton icon="plus" label="新建分组" className="group-create" onClick={() => { setName(''); setMode('name'); }}/>}
    {mode === 'name' ? <Dialog title={group ? '重命名分组' : '新建分组'} onClose={() => { if (!busy) setMode(null); }}><form onSubmit={e => { e.preventDefault(); if (busy || !name.trim()) return; setBusy(true); mutate(module, { type: group ? 'rename_group' : 'create_group', id: group?.id, name }).then(() => setMode(null)).catch(report).finally(() => setBusy(false)); }}><label>分组名称<input autoFocus value={name} onChange={e => setName(e.target.value)} maxLength={60}/></label><footer><Button onClick={() => setMode(null)} type="button" disabled={busy}>取消</Button><Button className="primary" type="submit" disabled={busy || !name.trim()}>{busy ? '保存中…' : '保存'}</Button></footer></form></Dialog> : null}
    {mode === 'delete' && group ? <Confirm title={`删除“${group.name}”？`} detail="组内内容会按原顺序移到未分组，内容不会被删除。" onClose={() => setMode(null)} onConfirm={async () => { await mutate(module, { type: 'delete_group', id: group.id }); const s = currentSession(); if (module !== 'todos' && s.selectedGroupIds[module] === group.id) setSession({ selectedGroupIds: { ...s.selectedGroupIds, [module]: 'all' } }); }}/> : null}
  </>;
}
