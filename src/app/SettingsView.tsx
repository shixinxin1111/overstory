import { useEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import { disable as disableAutostart, enable as enableAutostart, isEnabled as isAutostartEnabled } from '@tauri-apps/plugin-autostart';
import { useSnapshot, accept, currentSnapshot } from '../shared/snapshot-store';
import { bridge, call, native } from '../shared/bridge';
import { flushAll, restoreSession } from './session';
import { Button, Confirm, IconButton, report, run, toast } from '../shared/components/Controls';
import { Icon } from '../shared/icons';
import { resetDraft } from '../modules/notes/drafts';
import type { Snapshot } from '../shared/types';
import { readTheme, selectTheme, themes } from '../shared/theme';
import type { ThemeId } from '../shared/theme';
import { IntegrationSettings } from './IntegrationSettings';

export function SettingsView({ onClose, recoveryError }: { onClose: () => void; recoveryError?: string }) {
  const snapshot = useSnapshot();
  const [counts, setCounts] = useState<Record<string, number> | null>(null);
  const [busy, setBusy] = useState(false);
  const [theme, setTheme] = useState<ThemeId>(readTheme);
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [autoBusy, setAutoBusy] = useState(false);
  useEffect(() => {
    if (!native) return;
    void isAutostartEnabled().then(setAutostart).catch(report);
  }, []);
  const toggleAutostart = () => {
    if (!native || autostart === null || autoBusy) return;
    setAutoBusy(true);
    void run(async () => {
      if (autostart) { await disableAutostart(); setAutostart(false); toast('已关闭开机自启'); }
      else { await enableAutostart(); setAutostart(true); toast('已开启开机自启'); }
    }).finally(() => setAutoBusy(false));
  };
  return <div className="module-view settings-view">
    <div className="view-heading"><span>通用</span><IconButton icon="back" label="返回模块" onClick={onClose}/></div>
    {recoveryError ? <p className="recovery-message">{recoveryError}</p> : null}
    <div className="setting-row"><div><strong>开机自启</strong><p>登录 Mac 后自动启动 Overstory</p></div><button className={`toggle ${autostart ? 'on' : ''}`} role="switch" aria-checked={!!autostart} aria-label="开机自启" disabled={!native || autostart === null || autoBusy} onClick={toggleAutostart}><span/></button></div>
    <div className="setting-row"><div><strong>迷你悬浮窗显示待办数量</strong><p>显示所有未完成的待办数量，超过 99 显示 99+</p></div><button className={`toggle ${snapshot.preferences.showTodoCount ? 'on' : ''}`} role="switch" aria-checked={snapshot.preferences.showTodoCount} aria-label="迷你悬浮窗显示待办数量" disabled={!!recoveryError} onClick={() => void run(async () => { await flushAll(); const s = currentSnapshot(); accept(await bridge.preferences({ ...s.preferences, showTodoCount: !s.preferences.showTodoCount }, s.revision)); })}><span/></button></div>
    <div className="settings-section"><h2>外观</h2><p>主题效果会同步到主窗、悬浮窗和迷你悬浮窗。</p>
      <div className="theme-picker">
        {(['light', 'dark'] as const).map(mode => <div className="theme-row" key={mode}>
          <span className="theme-row-label">{mode === 'light' ? '浅色' : '深色'}</span>
          <div className="theme-swatches">{themes.filter(item => item.mode === mode).map(item =>
            <button key={item.id} type="button" className="theme-swatch" title={item.label} aria-label={item.label} aria-pressed={theme === item.id}
              style={{ '--swatch-bg': item.swatchBg, '--swatch-dot': item.swatchDot } as CSSProperties}
              onClick={() => { setTheme(item.id); void selectTheme(item.id); }}/>)}
          </div>
        </div>)}
      </div>
    </div>
    <div className="settings-section"><h2>数据与备份</h2><p>内容保存在这台 Mac。备份包含全部内容及排序。</p><div className="backup-actions"><Button disabled={busy || !!recoveryError} onClick={() => { setBusy(true); void run(async () => { await flushAll(); if (await call<boolean>('export_backup')) toast('备份已导出'); }).finally(() => setBusy(false)); }}><Icon name="download" size={16}/>导出备份</Button><Button disabled={busy} onClick={() => { setBusy(true); void run(async () => { await flushAll(); const result = await call<Record<string, number> | null>('import_backup', { confirm: false }); setCounts(result); }).finally(() => setBusy(false)); }}><Icon name="upload" size={16}/>恢复备份</Button></div></div>
    <IntegrationSettings kind="cli"/>
    <IntegrationSettings kind="skill"/>
    <div className="about-mark"><strong>Overstory</strong><span>版本 1.2.0</span></div>
    {counts ? <Confirm title="从备份覆盖恢复？" detail={`备份包含 ${counts.todos} 项待办、${counts.bookmarks} 个网页、${counts.notes} 篇便签。恢复会覆盖当前内容，当前数据将先自动备份。`} onClose={() => setCounts(null)} onConfirm={async () => { await flushAll(); const restored = await call<Snapshot>('import_backup', { confirm: true }); resetDraft(); accept(restored); restoreSession(restored.session); toast('备份已恢复'); }}/>: null}
  </div>;
}
