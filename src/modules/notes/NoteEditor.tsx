import { useState } from 'react';
import { flushAll, setSession } from '../../app/session';
import { mutate } from '../../shared/snapshot-store';
import { Button, Confirm, IconButton, run } from '../../shared/components/Controls';
import { FavoriteButton } from '../../shared/components/FavoriteButton';
import { Icon } from '../../shared/icons';
import type { Item } from '../../shared/types';
import { discardDeletedDraft, editDraft, flushDraft, useDraft, useLatestNote, overwriteDraft } from './drafts';
import { Markdown } from '../../shared/Markdown';

export function noteDate(value: string, full = false) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? '日期不可用' : date.toLocaleString('zh-CN', full
    ? { year: 'numeric', month: 'long', day: 'numeric', hour: '2-digit', minute: '2-digit' }
    : { month: 'numeric', day: 'numeric' });
}

export function NoteEditor({ item, location, panel = false }: { item: Item; location?: string; panel?: boolean }) {
  const draft = useDraft();
  const activeDraft = draft?.id === item.id ? draft : null;
  const [deleting, setDeleting] = useState(false);
  const [overwriting, setOverwriting] = useState(false);
  const title = activeDraft?.title ?? item.title;
  const content = activeDraft?.content ?? item.content ?? '';
  return <div className="note-editor">
    <div className="editor-toolbar" data-tauri-drag-region>
      {panel ? <Button onClick={() => void run(async () => { await flushAll(); setSession({ noteEditor: false }); })}><Icon name="back" size={16}/>收藏</Button> : <span className="editor-location">{location}</span>}
      {panel ? null : <span className={`save-status ${activeDraft?.status ?? 'saved'}`} aria-live="polite">{activeDraft?.status === 'failed' ? '保存失败' : activeDraft?.status === 'dirty' || activeDraft?.status === 'saving' ? '保存中' : '已保存'}</span>}
      <FavoriteButton module="notes" item={item} closeEditor={panel}/>
      <IconButton icon="trash" label="删除当前便签" onClick={() => setDeleting(true)}/>
    </div>
{activeDraft?.status === 'failed' ? <div className="save-error"><span>{activeDraft.error}</span>{activeDraft.conflict ? <><Button onClick={() => void run(useLatestNote)}>使用最新内容</Button><Button onClick={() => setOverwriting(true)}>保留草稿覆盖</Button></> : <Button onClick={() => void run(flushDraft)}><Icon name="retry" size={14}/>重试</Button>}</div> : null}
    {panel ? <div className="note-readonly" key={item.id}>
      <time className="note-date" dateTime={item.updatedAt}>{noteDate(item.updatedAt, true)}</time>
      <h1 className="note-readonly-title">{item.title || '未命名便签'}</h1>
      <Markdown content={item.content ?? ''}/>
    </div> : <>
      <div className="note-split">
        <div className="note-edit-pane">
          <input className="note-title-input" aria-label="便签标题" placeholder="未命名便签" value={title} onChange={e => editDraft(item.id, 'title', e.target.value)} maxLength={200}/>
          <textarea className="note-content-input" aria-label="便签正文" placeholder="支持 Markdown，例如 # 标题、**加粗**、- 列表" value={content} onChange={e => editDraft(item.id, 'content', e.target.value)} spellCheck={false}/>
        </div>
        <div className="note-preview" key={item.id}>
          <h1 className="note-preview-title">{title || '未命名便签'}</h1>
          <Markdown content={content}/>
        </div>
      </div>
      <div className="editor-footer">{content.length} 字 · Markdown</div>
    </>}
    {overwriting ? <Confirm title="用草稿覆盖最新内容？" detail="其他窗口或 CLI 修改的便签内容会被当前草稿替换。" onClose={() => setOverwriting(false)} onConfirm={overwriteDraft}/> : null}
    {deleting ? <Confirm title="删除这篇便签？" detail="删除后正文无法撤销，可先导出备份。" onClose={() => setDeleting(false)} onConfirm={async () => {
      await flushAll(); await mutate('notes', { type: 'delete', id: item.id });
      discardDeletedDraft(item.id); setSession({ selectedNoteId: null, noteEditor: false });
    }}/> : null}
  </div>;
}
