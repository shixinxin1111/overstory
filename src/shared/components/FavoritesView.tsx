import { useState } from 'react';
import { currentSession, flushAll, navigate, setSession, useSession } from '../../app/session';
import { NoteEditor } from '../../modules/notes/NoteEditor';
import { TodoForm } from '../../modules/todos/TodoForm';
import { BookmarkForm, bookmarkDraft } from '../../modules/bookmarks/BookmarkView';
import { BookmarkIcon } from '../../modules/bookmarks/BookmarkIcon';
import { discardDeletedDraft, useDraft } from '../../modules/notes/drafts';
import { call } from '../bridge';
import { Icon } from '../icons';
import { mutate, useSnapshot } from '../snapshot-store';
import { commitFavoriteSort, commitSort } from '../sortable/actions';
import { previewOrder, useSortPreview } from '../sortable/preview';
import { EndDrop, SortableScope, SortRow } from '../sortable/Sortable';
import type { Item, ModuleId } from '../types';
import { Confirm, EmptyState, run } from './Controls';
import { FavoriteButton } from './FavoriteButton';
import { RowMenu } from './RowMenu';

const labels = { todos: '待办', bookmarks: '网页', notes: '便签' };

export function FavoritesView({ module }: { module: ModuleId }) {
  const snapshot = useSnapshot(); const session = useSession(); const draft = useDraft();
  const data = snapshot.modules[module].data;
  const [deleting, setDeleting] = useState<Item | null>(null);
  const [editing, setEditing] = useState<Item | null>(null);
  const visible = module === 'todos'
    ? data.items.filter(i => !i.completed).sort((a, b) => a.sortIndex - b.sortIndex)
    : data.items.filter(i => i.favorited).sort((a, b) => (a.favoriteIndex ?? 0) - (b.favoriteIndex ?? 0));
  const selected = module === 'notes' ? visible.find(i => i.id === session.selectedNoteId) : undefined;
  const preview = useSortPreview();
  const ordered = previewOrder(visible, preview, 'item');
  const showEditor = !!selected && session.noteEditor;
  if (showEditor && selected) return <div className="module-view favorites-view favorite-editor"><NoteEditor item={selected} panel/></div>;
  return <div className="module-view favorites-view">
    <SortableScope onMove={async (entry, target) => { await run(() => module === 'todos' ? commitSort(module, entry, target) : commitFavoriteSort(module, entry, target)); }}>
      <div className="list-scroll favorites-list" data-scroll>
        {ordered.map(item => {
          const entry = { id: item.id, kind: 'item' as const, label: item.title };
          return <SortRow key={item.id} entry={entry} disabled={editing?.id === item.id}
            className={`favorite-row ${module === 'todos' ? `todo-row ${item.completed ? 'completed' : ''}` : module === 'bookmarks' ? 'bookmark-row' : 'note-row'}`}
            onActivate={module === 'todos' ? () => setEditing(item) : module === 'bookmarks' ? () => void run(() => call('open_bookmark', { id: item.id })) : () => void run(() => navigate({ selectedNoteId: item.id, noteEditor: true }))}>
            {module === 'todos' ? <>
              <button className={`checkbox ${item.completed ? 'checked' : ''}`} role="checkbox" aria-checked={!!item.completed} aria-label={`${item.completed ? '恢复' : '完成'}：${item.title}`} onClick={() => void run(() => mutate('todos', { type: 'update', id: item.id, completed: !item.completed }))}>{item.completed ? <Icon name="check" size={12}/> : null}</button>
              <div className="row-title">
              <button data-row-drag className="todo-title">{item.title}</button></div>
            </> : <div className="row-title">
              {module === 'bookmarks' ? <button data-row-drag className="bookmark-select"><BookmarkIcon icon={item.icon}/><span>{item.title}</span></button> : <button data-row-drag className="note-select">{item.id === draft?.id ? draft.title || '未命名便签' : item.title}</button>}
              <FavoriteButton module={module} item={item} className="row-action"/>
            </div>}
            <RowMenu title={item.title} onEdit={() => {
              if (module === 'todos') setEditing(item);
              else if (module === 'bookmarks') setSession({ bookmarkForm: bookmarkDraft(item) });
              else void run(() => navigate({ selectedNoteId: item.id, noteEditor: true }));
            }} onDelete={() => setDeleting(item)}/>
          </SortRow>;
        })}
        <EndDrop kind="item"/>
        {!visible.length ? <EmptyState icon={module === 'todos' ? 'todos' : 'favorite'} title={module === 'todos' ? '暂无未完成待办' : `暂无收藏${labels[module]}`} detail={module === 'todos' ? '在主窗添加待办后会自动显示。' : '在主窗点击星标收藏后会显示在这里。'}/> : null}
      </div>
    </SortableScope>
    {editing ? <TodoForm key={editing.id} item={editing} onClose={() => setEditing(null)}/> : null}
    {module === 'bookmarks' && session.bookmarkForm?.id ? <BookmarkForm draft={session.bookmarkForm} groups={data.groups} showGroup={false} onClose={() => setSession({ bookmarkForm: null })}/> : null}
    {deleting ? <Confirm title={`删除这${module === 'notes' ? '篇便签' : module === 'bookmarks' ? '个网页' : '项待办'}？`} detail={module === 'todos' ? '内容也会从主窗删除。完成待办可将它移出悬浮窗。' : '内容也会从主窗删除。仅移出悬浮窗请取消收藏。'} onClose={() => setDeleting(null)} onConfirm={async () => {
      await flushAll(); await mutate(module, { type: 'delete', id: deleting.id });
      if (module === 'notes') {
        discardDeletedDraft(deleting.id);
        if (currentSession().selectedNoteId === deleting.id) setSession({ selectedNoteId: null, noteEditor: false });
      }
      if (editing?.id === deleting.id) setEditing(null);
    }}/> : null}
  </div>;
}
