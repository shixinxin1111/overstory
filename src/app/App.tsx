import { useEffect, useState } from 'react';
import { MotionConfig } from 'motion/react';
import { flushSync } from 'react-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { bridge, call, native, on } from '../shared/bridge';
import { accept, initialize, useSnapshot, currentSnapshot } from '../shared/snapshot-store';
import { currentSession, flushAll, restoreSession, useSession } from './session';
import { modules } from './registry';
import { Navigation } from './shell/Navigation';
import { MiniWindow } from './shell/MiniWindow';
import { WindowSwitches } from './shell/WindowSwitches';
import { SettingsView } from './SettingsView';
import { ToastHost, report, run, toast } from '../shared/components/Controls';
import { Icon } from '../shared/icons';
import { FavoritesView } from '../shared/components/FavoritesView';
import { springs } from '../shared/motion';
import { applyTheme, readTheme } from '../shared/theme';
import type { AppError, Presentation, Session, Snapshot } from '../shared/types';
import type { ThemeId } from '../shared/theme';
import '../modules/notes/drafts';

const windowLabel = native ? getCurrentWindow().label : 'preview';
export function App() {
  const snapshot = useSnapshot(); const session = useSession();
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState(false);
  const [recoveryError, setRecoveryError] = useState<string>();
  const mode = windowLabel === 'main' ? 'main' : snapshot.windowState.mode;
  const presentation = mode === 'main' ? 'main' : 'panel';
  const selectedModule = modules.find(m => m.id === session.activeModuleId) ?? modules[0];
  const View = selectedModule.View;
  const showSettings = settings && presentation === 'main';
  useEffect(() => {
    applyTheme(readTheme());
    let disposed = false; let off: (() => void) | undefined;
    void on<ThemeId>('overstory:theme', applyTheme).then(fn => { if (disposed) fn(); else off = fn; });
    return () => { disposed = true; off?.(); };
  }, []);
  useEffect(() => {
    const unlisteners: (() => void)[] = [];
    let disposed = false;
    const subscribe = async () => {
      const push = (unlisten: () => void) => { if (disposed) unlisten(); else unlisteners.push(unlisten); };
      push(await on<Session>('app:session', restoreSession));
      push(await on<AppError>('app:error', report));
      push(await on<Snapshot>('app:present-window', next => {
        flushSync(() => { accept(next); restoreSession(next.session); });
        void run(() => call('renderer_ready', { presentation: next.windowState.mode }));
      }));
      push(await on<{ token: string; target: Presentation | null }>('app:prepare-transition', async ({ token }) => {
        document.dispatchEvent(new Event('overstory:cancel-drag'));
        try {
          await flushAll();
          await bridge.acknowledge(token, currentSession(), true);
        } catch (e) { report(e); await bridge.acknowledge(token, currentSession(), false).catch(report); }
      }));
      let needsRecovery = false;
      try { await initialize(); restoreSession(currentSnapshot().session); }
      catch (e) { needsRecovery = true; const message = (e as AppError).message ?? String(e); setRecoveryError(message); setSettings(true); }
      if (!disposed) {
        setReady(true); await call('renderer_ready');
        if (needsRecovery) await bridge.presentation('main');
      }
    };
    void subscribe().catch(report);
    return () => { disposed = true; unlisteners.forEach(fn => fn()); };
  }, []);
  useEffect(() => {
    if (snapshot.recoveryNotice && ready) toast(snapshot.recoveryNotice, true);
  }, [snapshot.recoveryNotice, ready]);
  useEffect(() => {
    if (recoveryError && snapshot.revision > 0) setRecoveryError(undefined);
  }, [snapshot.revision]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing || e.key !== 'Escape' || document.querySelector('[data-modal]') || mode !== 'panel') return;
      e.preventDefault(); void run(() => bridge.presentation('orb'));
    };
    document.addEventListener('keydown', key); return () => document.removeEventListener('keydown', key);
  }, [mode]);
  useEffect(() => {
    const root = document.querySelector('.module-content');
    if (!root) return;
    const scrolls = [...root.querySelectorAll<HTMLElement>('[data-scroll]')];
    scrolls.forEach((el, i) => { el.scrollTop = currentSession().scroll[`${presentation}:${session.activeModuleId}:${i}`] ?? 0; });
    const scroll = (e: Event) => {
      const index = scrolls.indexOf(e.target as HTMLElement);
      if (index >= 0) currentSession().scroll[`${presentation}:${session.activeModuleId}:${index}`] = (e.target as HTMLElement).scrollTop;
    };
    root.addEventListener('scroll', scroll, true); return () => root.removeEventListener('scroll', scroll, true);
  }, [session.activeModuleId, mode, ready, settings, session.noteEditor]);
  const openSettings = () => void run(async () => { await flushAll(); setSettings(true); });
  if (!ready) return <div className="loading-surface" aria-label="正在加载"/>;
  return <MotionConfig reducedMotion="user" transition={springs.feedback}>
    {mode === 'orb' && !recoveryError ? <MiniWindow/> : <div className={`app-surface ${presentation}`}>
      {presentation === 'main' ? <aside className="sidebar"><div className="sidebar-titlebar" onPointerDown={e => { if (native && e.button === 0) void run(() => getCurrentWindow().startDragging()); }}/><Navigation settings={settings} onSelect={() => setSettings(false)}/><div className="sidebar-bottom"><button onClick={openSettings} className={settings ? 'active' : ''}><Icon name="settings" size={18}/>设置</button></div></aside> : null}
      <div className="workspace"><header className="window-toolbar" onPointerDown={e => { if (!(e.target as HTMLElement).closest('button') && native && e.button === 0) void run(() => getCurrentWindow().startDragging()); }}>
        <h1 className="window-title">{showSettings ? '设置' : selectedModule.title}</h1><WindowSwitches mode={presentation}/>
      </header>
        {presentation === 'panel' ? <Navigation horizontal settings={false} onSelect={() => setSettings(false)}/> : null}
        <main className="module-content"><div className="module-stage" key={showSettings ? 'settings' : `${presentation}:${selectedModule.id}`}>{showSettings ? <SettingsView recoveryError={recoveryError} onClose={() => setSettings(false)}/> : presentation === 'panel' ? <FavoritesView module={session.activeModuleId}/> : <View presentation={presentation}/>}</div></main>
      </div>
    </div>}<ToastHost/>{!native ? <div className="preview-badge">开发预览 · 隔离示例数据</div> : null}
  </MotionConfig>;
}
