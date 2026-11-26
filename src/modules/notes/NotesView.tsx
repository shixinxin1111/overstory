import { useRef, useState } from 'react';
import { useSnapshot, mutate, currentSnapshot } from '../../shared/snapshot-store';
import { useSession, currentSession, setSession, flushAll, navigate } from '../../app/session';
import { Button, Confirm, EmptyState, SearchField, run, toast } from '../../shared/components/Controls';
import { FavoriteButton } from '../../shared/components/FavoriteButton';
import { GroupedList, GroupChooser, GroupActions } from '../../shared/components/GroupedList';
import { SortableScope, SortRow } from '../../shared/sortable/Sortable';
import { Icon } from '../../shared/icons';
import { discardDeletedDraft, useDraft } from './drafts';
import { NoteEditor } from './NoteEditor';
import { commitSort } from '../../shared/sortable/actions';
import { orderedGroups } from '../../shared/groups';
import { RowMenu } from '../../shared/components/RowMenu';
import type { Item } from '../../shared/types';

export function NotesView() {
  const snapshot = useSnapshot(); const session = useSession(); const draft = useDraft();
  const data = snapshot.modules.notes.data;
  const query = session.search.notes ?? '';
  const selectedGroup = session.selectedGroupIds.notes;
  const selected = data.items.find(i => i.id === session.selectedNoteId);
  const [hoverGroup, setHoverGroup] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<Item | null>(null);
  const [adding, setAdding] = useState(false);
  const pending = useRef(false);
  const visible = data.items.filter(i => `${i.title} ${i.content}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const add = () => run(async () => {
    if (pending.current) return;
    pending.current = true; setAdding(true);
    try {
      await flushAll();
      const ids = new Set(currentSnapshot().modules.notes.data.items.map(i => i.id));
      const groupId = selectedGroup === 'all' ? 'ungrouped' : selectedGroup;
      const result = await mutate('notes', { type: 'create', title: '未命名便签', content: '', groupId });
      const added = result.modules.notes.data.items.find(i => !ids.has(i.id));
      if (added) setSession({ selectedNoteId: added.id, noteEditor: true, search: { ...currentSession().search, notes: '' }, collapsed: { ...currentSession().collapsed, [`notes:${groupId}`]: false } });
    } finally { pending.current = false; setAdding(false); }
  });
  return <div className="module-view notes-view editor-open">
    <SortableScope groups={orderedGroups(data.groups)} onHoverGroup={setHoverGroup} onMove={async (entry, target) => { await run(async () => { const moved = await commitSort('notes', entry, target); if (moved && entry.kind === 'item' && selectedGroup !== 'all' && target.groupId !== selectedGroup) toast(`已移到${data.groups.find(g => g.id === target.groupId)?.name}`); }); }}>
      <div className="view-heading content-toolbar" data-tauri-drag-region>
        <SearchField value={query} placeholder="搜索便签" onChange={s => setSession({ search: { ...currentSession().search, notes: s } })}/>
        <div className="toolbar-filters"><GroupChooser module="notes" groups={data.groups}/><GroupActions module="notes"/></div>
        <Button title="清空搜索，显示全部分组" onClick={() => setSession({ search: { ...currentSession().search, notes: '' }, selectedGroupIds: { ...currentSession().selectedGroupIds, notes: 'all' } })}><Icon name="retry"/>重置</Button>
        <Button className="primary" aria-label="新建便签" disabled={adding} onClick={() => void add()}><Icon name="plus"/>添加</Button>
      </div>
    <div className="notes-workspace">
      <div className="notes-browser">
          {query ? <p className="sort-hint">清空搜索后可排序</p> : null}
          <div className="list-scroll" data-scroll><GroupedList module="notes" groups={data.groups} items={visible} search={query} temporaryGroup={hoverGroup} renderItem={item => {
            const entry = { id: item.id, kind: 'item' as const, label: item.title, groupId: item.groupId };
            return <SortRow entry={entry} disabled={!!query} key={item.id} className={`note-row ${selected?.id === item.id ? 'selected' : ''}`} onActivate={() => void run(() => navigate({ selectedNoteId: item.id, noteEditor: true }))}><div className="row-title"><button data-row-drag className="note-select">{item.id === draft?.id ? draft.title || '未命名便签' : item.title}</button><FavoriteButton module="notes" item={item} className={item.favorited ? '' : 'row-action'}/></div><RowMenu title={item.title} onEdit={() => void run(() => navigate({ selectedNoteId: item.id, noteEditor: true }))} onDelete={() => setDeleting(item)}/></SortRow>;
          }}/></div>
      </div>
      {selected ? <NoteEditor item={selected} location={data.groups.find(g => g.id === selected.groupId)?.name}/> : <div className="note-editor"><EmptyState icon="notes" title={data.items.length ? '选择一篇便签' : '暂无便签'} detail={data.items.length ? '从左侧选择，继续编辑。' : '记录想法，内容自动保存在这台 Mac。'} action={<Button disabled={adding} onClick={() => void add()}><Icon name="plus" size={18}/>新建便签</Button>}/></div>}
    </div>
    </SortableScope>
    {deleting ? <Confirm title="删除这篇便签？" detail="删除后正文无法撤销，可先导出备份。" onClose={() => setDeleting(null)} onConfirm={async () => {
      await flushAll(); await mutate('notes', { type: 'delete', id: deleting.id }); discardDeletedDraft(deleting.id);
      if (currentSession().selectedNoteId === deleting.id) setSession({ selectedNoteId: null, noteEditor: false });
    }}/> : null}
  </div>;
}
