import { useState } from 'react';
import { useSnapshot, accept, currentSnapshot } from '../shared/snapshot-store';
import { bridge, call } from '../shared/bridge';
import { flushAll, restoreSession } from './session';
import { Button, Confirm, IconButton, run, toast } from '../shared/components/Controls';
import { Icon } from '../shared/icons';
import { resetDraft } from '../modules/notes/drafts';
import type { Snapshot } from '../shared/types';
import { IntegrationSettings } from './IntegrationSettings';

export function SettingsView({ onClose, recoveryError }: { onClose: () => void; recoveryError?: string }) {
  const snapshot = useSnapshot();
  const [counts, setCounts] = useState<Record<string, number> | null>(null);
  const [busy, setBusy] = useState(false);
  return <div className="module-view settings-view">
    <div className="view-heading"><span>通用</span><IconButton icon="back" label="返回模块" onClick={onClose}/></div>
    {recoveryError ? <p className="recovery-message">{recoveryError}</p> : null}
    <div className="setting-row"><div><strong>迷你悬浮窗显示待办数量</strong><p>显示所有未完成的待办数量，超过 99 显示 99+</p></div><button className={`toggle ${snapshot.preferences.showTodoCount ? 'on' : ''}`} role="switch" aria-checked={snapshot.preferences.showTodoCount} aria-label="迷你悬浮窗显示待办数量" disabled={!!recoveryError} onClick={() => void run(async () => { await flushAll(); const s = currentSnapshot(); accept(await bridge.preferences({ ...s.preferences, showTodoCount: !s.preferences.showTodoCount }, s.revision)); })}><span/></button></div>
    <div className="settings-section"><h2>数据与备份</h2><p>内容保存在这台 Mac。备份包含全部内容及排序。</p><div className="backup-actions"><Button disabled={busy || !!recoveryError} onClick={() => { setBusy(true); void run(async () => { await flushAll(); if (await call<boolean>('export_backup')) toast('备份已导出'); }).finally(() => setBusy(false)); }}><Icon name="download" size={16}/>导出备份</Button><Button disabled={busy} onClick={() => { setBusy(true); void run(async () => { await flushAll(); const result = await call<Record<string, number> | null>('import_backup', { confirm: false }); setCounts(result); }).finally(() => setBusy(false)); }}><Icon name="upload" size={16}/>恢复备份</Button></div></div>
    <IntegrationSettings kind="cli"/>
    <IntegrationSettings kind="skill"/>
    <div className="about-mark"><strong>Overstory</strong><span>版本 1.1.1</span></div>
    {counts ? <Confirm title="从备份覆盖恢复？" detail={`备份包含 ${counts.todos} 项待办、${counts.bookmarks} 个网页、${counts.notes} 篇便签。恢复会覆盖当前内容，当前数据将先自动备份。`} onClose={() => setCounts(null)} onConfirm={async () => { await flushAll(); const restored = await call<Snapshot>('import_backup', { confirm: true }); resetDraft(); accept(restored); restoreSession(restored.session); toast('备份已恢复'); }}/>: null}
  </div>;
}
