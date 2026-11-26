import { useRef, useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { useSnapshot, mutate } from '../../shared/snapshot-store';
import { useSession, setSession, currentSession, flushAll } from '../../app/session';
import { commitSort } from '../../shared/sortable/actions';
import { Button, Confirm, EmptyState, IconButton, SearchField, report, run } from '../../shared/components/Controls';
import { SortableScope, SortRow, EndDrop } from '../../shared/sortable/Sortable';
import { Icon } from '../../shared/icons';
import { reveal } from '../../shared/motion';
import type { Item } from '../../shared/types';
import { RowMenu } from '../../shared/components/RowMenu';
import { TodoForm } from './TodoForm';

export function TodoView() {
  const snapshot = useSnapshot(); const session = useSession();
  const items = snapshot.modules.todos.data.items;
  const query = session.search.todos ?? '';
  const title = session.todoInput ?? '';
  const setTitle = (todoInput: string) => setSession({ todoInput });
  const [busy, setBusy] = useState(false);
  const adding = session.todoAdding ?? false;
  const pending = useRef(false);
  const [editing, setEditing] = useState<Item | null>(null);
  const [deleting, setDeleting] = useState<Item | null>(null);
  const visible = items.filter(i => (session.todoFilter === 'all' || i.completed === (session.todoFilter === 'completed')) && `${i.title} ${i.content ?? ''}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
  const add = async () => {
    if (pending.current || !title.trim()) return; pending.current = true; setBusy(true);
    try { await flushAll(); await mutate('todos', { type: 'create', title }); setTitle(''); setSession({ todoAdding: false, todoFilter: 'active', search: { ...currentSession().search, todos: '' } }); } catch (e) { report(e); } finally { pending.current = false; setBusy(false); }
  };
  return <div className="module-view todo-view">
    <div className="view-heading content-toolbar" data-tauri-drag-region>
      <SearchField value={query} placeholder="搜索待办" onChange={s => setSession({ search: { ...currentSession().search, todos: s } })}/>
      <div className="toolbar-filters"><div className="segmented" role="group" aria-label="待办筛选">{([['all','全部'],['active','未完成'],['completed','已完成']] as const).map(([id,label]) => <button key={id} aria-pressed={session.todoFilter === id} onClick={() => setSession({ todoFilter: id })}>{session.todoFilter === id ? <motion.span className="segment-indicator" layoutId="todo-filter"/> : null}<span>{label}</span></button>)}</div></div>
      <Button title="清空搜索，显示全部待办" onClick={() => setSession({ todoFilter: 'all', search: { ...currentSession().search, todos: '' } })}><Icon name="retry"/>重置</Button>
      <Button className="primary" aria-label="新建待办" aria-expanded={adding} disabled={busy} onClick={() => setSession({ todoAdding: !adding })}><Icon name="plus"/>添加</Button>
    </div>
    {query ? <p className="sort-hint">清空搜索后可排序</p> : null}
    <SortableScope onMove={async (entry, target) => { await run(() => commitSort('todos', entry, target)); }}>
      <div className="list-scroll" data-scroll>{adding ? <form className="quick-add" onSubmit={e => { e.preventDefault(); void add(); }}><Icon name="todos" size={18}/><input autoFocus aria-label="新增待办" placeholder="输入待办标题，回车添加" value={title} disabled={busy} onChange={e => setTitle(e.target.value)} onKeyDown={e => { if (e.nativeEvent.isComposing) { if (e.key === 'Enter') e.preventDefault(); return; } if (e.key === 'Escape') { e.stopPropagation(); if (!busy) setSession({ todoAdding: false }); } }} maxLength={200}/><IconButton icon="close" label="取消添加" disabled={busy} onClick={() => setSession({ todoAdding: false })}/></form> : null}<AnimatePresence initial={false}>{visible.map(item => {
        const entry = { id: item.id, kind: 'item' as const, label: item.title };
        return <motion.div key={item.id} {...reveal}><SortRow entry={entry} disabled={!!query || editing?.id === item.id} className={`todo-row ${item.completed ? 'completed' : ''}`} onActivate={() => setEditing(item)}>
          <button className={`checkbox ${item.completed ? 'checked' : ''}`} role="checkbox" aria-checked={!!item.completed} aria-label={`${item.completed ? '恢复' : '完成'}：${item.title}`} onClick={() => void run(() => mutate('todos', { type: 'update', id: item.id, completed: !item.completed }))}><svg viewBox="0 0 20 20" aria-hidden="true"><motion.path d="m5 10 3.5 3.5L15 6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" animate={{ pathLength: item.completed ? 1 : 0 }} transition={{ duration: .18 }}/></svg></button>
          <div className="row-title">
          <button data-row-drag className="todo-title">{item.title}</button></div>
          <RowMenu title={item.title} onEdit={() => setEditing(item)} onDelete={() => setDeleting(item)}/>
          </SortRow></motion.div>;
      })}</AnimatePresence><EndDrop kind="item"/>{!visible.length && !adding ? <EmptyState icon={session.todoFilter === 'completed' ? 'check' : 'todos'} title={query ? '没有匹配的待办' : session.todoFilter === 'completed' ? '暂无已完成待办' : items.length ? '已全部完成' : '暂无待办'} detail={query ? '试试其他关键词。' : items.length && session.todoFilter === 'active' ? '新的待办可随时添加。' : '点击搜索旁的加号，按回车添加。'}/> : null}</div>
    </SortableScope>
    <div className="view-footnote"><span>{visible.length} 项</span><span>拖动整行排序</span></div>
    {editing ? <TodoForm key={editing.id} item={editing} onClose={() => setEditing(null)}/> : null}
    {deleting ? <Confirm title="删除这项待办？" detail={deleting.title} onClose={() => setDeleting(null)} onConfirm={() => mutate('todos', { type: 'delete', id: deleting.id })}/> : null}
  </div>;
}
