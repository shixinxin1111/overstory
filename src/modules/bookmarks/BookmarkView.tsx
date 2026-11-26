import { useState } from 'react';
import { useSnapshot, mutate } from '../../shared/snapshot-store';
import { useSession, setSession, currentSession } from '../../app/session';
import { commitSort } from '../../shared/sortable/actions';
import { Button, Confirm, Dialog, EmptyState, SearchField, report, run, toast } from '../../shared/components/Controls';
import { GroupedList, GroupChooser, GroupActions } from '../../shared/components/GroupedList';
import { SortableScope, SortRow } from '../../shared/sortable/Sortable';
import { Icon } from '../../shared/icons';
import { call } from '../../shared/bridge';
import type { Item, Group, Session } from '../../shared/types';
import { FavoriteButton } from '../../shared/components/FavoriteButton';
import { orderedGroups } from '../../shared/groups';
import { RowMenu } from '../../shared/components/RowMenu';
import { BookmarkIcon } from './BookmarkIcon';

export function bookmarkDraft(item?: Item, groupId = 'ungrouped'): NonNullable<Session['bookmarkForm']> {
  return {
    id: item?.id ?? null, title: item?.title ?? '', url: item?.url ?? '',
    icon: item?.icon ?? '', description: item?.description ?? '',
    groupId: item?.groupId ?? groupId,
    ...(item ? { expectedItemVersion: item.version ?? 0 } : {}),
  };
}

export function BookmarkView() {
  const snapshot = useSnapshot(); const session = useSession();
  const data = snapshot.modules.bookmarks.data;
  const query = session.search.bookmarks ?? '';
  const selected = session.selectedGroupIds.bookmarks;
  const edit = session.bookmarkForm;
  const setEdit = (item: Item | 'new' | null) => setSession({ bookmarkForm: item === null ? null : bookmarkDraft(item === 'new' ? undefined : item, selected === 'all' ? 'ungrouped' : selected) });
  const [deleting, setDeleting] = useState<Item | null>(null);
  const [hoverGroup, setHoverGroup] = useState<string | null>(null);
  const visible = data.items.filter(i => `${i.title} ${i.url ?? ''} ${i.description ?? ''}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  return <div className="module-view bookmark-view">
    <SortableScope groups={orderedGroups(data.groups)} onHoverGroup={setHoverGroup} onMove={async (entry, target) => { await run(async () => { const moved = await commitSort('bookmarks', entry, target); if (moved && entry.kind === 'item' && selected !== 'all' && selected !== target.groupId) toast(`已移到${data.groups.find(g => g.id === target.groupId)?.name}`); }); }}>
      <div className="view-heading content-toolbar" data-tauri-drag-region>
        <SearchField value={query} placeholder="搜索网页" onChange={s => setSession({ search: { ...currentSession().search, bookmarks: s } })}/>
        <div className="toolbar-filters"><GroupChooser module="bookmarks" groups={data.groups}/><GroupActions module="bookmarks"/></div>
        <Button title="清空搜索，显示全部分组" onClick={() => setSession({ search: { ...currentSession().search, bookmarks: '' }, selectedGroupIds: { ...currentSession().selectedGroupIds, bookmarks: 'all' } })}><Icon name="retry"/>重置</Button>
        <Button className="primary" aria-label="添加网页" onClick={() => setEdit('new')}><Icon name="plus"/>添加</Button>
      </div>
      {query ? <p className="sort-hint">清空搜索后可排序</p> : null}
      <div className="list-scroll" data-scroll>
        <GroupedList module="bookmarks" groups={data.groups} items={visible} search={query} temporaryGroup={hoverGroup} renderItem={item => {
          const entry = { id: item.id, kind: 'item' as const, label: item.title, groupId: item.groupId };
          return <SortRow key={item.id} entry={entry} disabled={!!query} className="bookmark-row" onActivate={() => void run(async () => { await call('open_bookmark', { id: item.id }); toast(`已打开${item.title}`); })}><div className="row-title"><button data-row-drag className="bookmark-select"><BookmarkIcon icon={item.icon}/><span>{item.title}</span></button><FavoriteButton module="bookmarks" item={item} className={item.favorited ? '' : 'row-action'}/></div><RowMenu title={item.title} onEdit={() => setEdit(item)} onDelete={() => setDeleting(item)}/></SortRow>;
        }}/>
        {!visible.length ? <EmptyState icon="bookmarks" title={query ? '没有匹配的网页' : '暂无网页'} detail={query ? '试试网页标题或域名。' : '添加网址后，点击标题打开。'}/> : null}
      </div>
    </SortableScope>
    <div className="view-footnote"><span>{visible.length} 个网页</span><span>拖动整行或分组排序</span></div>
    {edit ? <BookmarkForm draft={edit} groups={orderedGroups(data.groups)} onClose={() => setEdit(null)}/> : null}
    {deleting ? <Confirm title="删除这个网页？" detail={deleting.title} onClose={() => setDeleting(null)} onConfirm={() => mutate('bookmarks', { type: 'delete', id: deleting.id })}/> : null}
  </div>;
}
export function BookmarkForm({ draft, groups, onClose, showGroup = true }: { draft: NonNullable<Session['bookmarkForm']>; groups: Group[]; onClose: () => void; showGroup?: boolean }) {
  const { title, url, groupId: group } = draft;
  const icon = draft.icon ?? ''; const description = draft.description ?? '';
  const setTitle = (title: string) => setSession({ bookmarkForm: { ...draft, title } });
  const setUrl = (url: string) => setSession({ bookmarkForm: { ...draft, url } });
  const setGroup = (groupId: string) => setSession({ bookmarkForm: { ...draft, groupId } });
  const setIcon = (icon: string) => setSession({ bookmarkForm: { ...draft, icon } });
  const setDescription = (description: string) => setSession({ bookmarkForm: { ...draft, description } });
  const [busy, setBusy] = useState(false);
  return <Dialog title={draft.id ? '编辑网页' : '添加网页'} onClose={() => { if (!busy) onClose(); }}><form onSubmit={e => { e.preventDefault(); if (busy || !title.trim() || !url.trim()) return; setBusy(true); mutate('bookmarks', {
    type: draft.id ? 'update' : 'create', ...(draft.id ? { id: draft.id } : {}),
    ...(draft.id && draft.expectedItemVersion !== undefined ? { expectedItemVersion: draft.expectedItemVersion } : {}),
    title, url, groupId: group,
    ...(draft.icon !== undefined ? { icon } : {}), ...(draft.description !== undefined ? { description } : {}),
  }).then(onClose).catch(report).finally(() => setBusy(false)); }}>
    <label>标题<input autoFocus placeholder="例如：设计灵感" value={title} onChange={e => setTitle(e.target.value)} maxLength={200}/></label><label>网页地址<input placeholder="example.com" value={url} onChange={e => setUrl(e.target.value)} spellCheck={false}/></label><p className="field-hint">未填写协议时自动补全 https://</p>
    <label>图标地址<input placeholder="可选，例如 https://example.com/favicon.ico" value={icon} onChange={e => setIcon(e.target.value)} spellCheck={false}/></label>
    <label>描述<textarea rows={3} placeholder="可选的网站描述" value={description} onChange={e => setDescription(e.target.value)}/></label>
    {showGroup ? <label>分组<select value={group} onChange={e => setGroup(e.target.value)}>{groups.map(g => <option key={g.id} value={g.id}>{g.name}</option>)}</select></label> : null}<footer><Button type="button" disabled={busy} onClick={onClose}>取消</Button><Button className="primary" type="submit" disabled={busy || !title.trim() || !url.trim()}>{busy ? '保存中…' : '保存'}</Button></footer>
  </form></Dialog>;
}
