import { getCurrentWindow } from '@tauri-apps/api/window';
import { call, native } from '../../shared/bridge';
import { useSnapshot } from '../../shared/snapshot-store';
import { modules } from '../registry';
import { Icon } from '../../shared/icons';
import { run } from '../../shared/components/Controls';
import { WindowSwitches } from './WindowSwitches';

export function MiniWindow() {
  const snapshot = useSnapshot();
  const number = Number(modules[0].getOrbSummary?.(snapshot) ?? 0);
  return <div className="mini-window"><header className="window-toolbar"
    onPointerDown={e => { if (!(e.target as HTMLElement).closest('button') && native && e.button === 0) void run(() => getCurrentWindow().startDragging()); }}
    onContextMenu={e => { e.preventDefault(); void run(() => call('orb_menu')); }}>
    <span className="mini-summary" title="拖动移动 · 右键菜单"><Icon name="spark" size={16}/>
      {snapshot.preferences.showTodoCount ? <span aria-label={`${number} 项待办`}><span className="mini-number">{number > 99 ? '99+' : number}</span> 待办</span> : null}
    </span><WindowSwitches mode="orb"/>
  </header></div>;
}
