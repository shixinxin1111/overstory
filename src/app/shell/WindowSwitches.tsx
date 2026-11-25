import { bridge } from '../../shared/bridge';
import { Icon } from '../../shared/icons';
import { run } from '../../shared/components/Controls';
import type { Presentation } from '../../shared/types';

const targets: Record<Presentation, readonly Presentation[]> = {
  orb: ['main', 'panel'],
  panel: ['main', 'orb'],
  main: ['panel', 'orb'],
};
const controls = {
  main: { icon: 'expand', label: '返回主窗' },
  panel: { icon: 'panel', label: '返回悬浮窗' },
  orb: { icon: 'collapse', label: '返回迷你悬浮窗' },
} as const;

export function WindowSwitches({ mode }: { mode: Presentation }) {
  return <div className="toolbar-actions" aria-label="切换窗口">
    {targets[mode].map(target => <button key={target} type="button" className="button icon-button" data-no-drag
      aria-label={controls[target].label} title={controls[target].label}
      onClick={() => {
        document.dispatchEvent(new Event('overstory:cancel-drag'));
        void run(() => bridge.presentation(target));
      }}><Icon name={controls[target].icon}/></button>)}
  </div>;
}
