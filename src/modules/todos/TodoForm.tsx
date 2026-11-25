import { useState } from 'react';
import type { Item } from '../../shared/types';
import { mutate } from '../../shared/snapshot-store';
import { Button, Dialog, report } from '../../shared/components/Controls';

export function TodoForm({ item, onClose }: { item: Item; onClose: () => void }) {
  const [title, setTitle] = useState(item.title);
  const [content, setContent] = useState(item.content ?? '');
  const [busy, setBusy] = useState(false);
  return <Dialog title="编辑待办" onClose={() => { if (!busy) onClose(); }}><form onSubmit={e => {
    e.preventDefault();
    if (busy || !title.trim()) return;
    setBusy(true);
    mutate('todos', { type: 'update', id: item.id, title, content, ...(item.version ? { expectedItemVersion: item.version } : {}) })
      .then(onClose).catch(report).finally(() => setBusy(false));
  }}>
    <label>标题<input autoFocus aria-label="待办标题" value={title} onChange={e => setTitle(e.target.value)} maxLength={200}/></label>
    <label>描述<textarea aria-label="待办描述" placeholder="补充说明（可选）" value={content} onChange={e => setContent(e.target.value)} rows={5}/></label>
    <footer><Button type="button" disabled={busy} onClick={onClose}>取消</Button><Button className="primary" type="submit" disabled={busy || !title.trim()}>{busy ? '保存中…' : '保存'}</Button></footer>
  </form></Dialog>;
}
