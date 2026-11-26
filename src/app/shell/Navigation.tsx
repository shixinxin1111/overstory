import { motion } from 'motion/react';
import { orderedModules } from '../registry';
import { currentSnapshot, useSnapshot, accept } from '../../shared/snapshot-store';
import { useSession, navigate, flushAll } from '../session';
import { bridge } from '../../shared/bridge';
import { SortableScope, SortRow, EndDrop } from '../../shared/sortable/Sortable';
import { previewOrder, useSortPreview } from '../../shared/sortable/preview';
import { run } from '../../shared/components/Controls';
import { Icon } from '../../shared/icons';
import type { ModuleId } from '../../shared/types';

export function Navigation({ horizontal, onSelect, settings }: { horizontal?: boolean; onSelect: () => void; settings: boolean }) {
  const snapshot = useSnapshot(); const session = useSession();
  const preview = useSortPreview();
  const ordered = previewOrder(orderedModules(snapshot.preferences.moduleOrder), preview, 'nav');
  return <nav className={`module-navigation ${horizontal ? 'horizontal-nav' : ''}`} aria-label="功能模块">
    <SortableScope orientation={horizontal ? 'horizontal' : 'vertical'} onMove={async (entry, target) => { await run(async () => {
      await flushAll(); const s = currentSnapshot();
      const order = [...s.preferences.moduleOrder, ...orderedModules(s.preferences.moduleOrder).filter(m => !s.preferences.moduleOrder.includes(m.id)).map(m => m.id)].filter(id => id !== entry.id);
      const at = target.beforeId ? order.indexOf(target.beforeId) : order.length;
      order.splice(at < 0 ? order.length : at, 0, entry.id);
      if (order.join('|') === s.preferences.moduleOrder.join('|')) return;
      accept(await bridge.preferences({ ...s.preferences, moduleOrder: order }, s.revision));
    }); }}>
      {ordered.map(module => {
        const entry = { id: module.id, kind: 'nav' as const, label: module.title };
        const active = !settings && session.activeModuleId === module.id;
        const count = snapshot.modules[module.id].data.items.filter(i => module.id === 'todos' ? !i.completed : !horizontal || i.favorited).length;
        return <SortRow key={module.id} entry={entry} className={`nav-row nav-${module.id} ${active ? 'active' : ''}`}><button data-row-drag aria-current={active ? 'page' : undefined} onClick={() => void run(async () => { document.dispatchEvent(new Event('overstory:cancel-drag')); await navigate({ activeModuleId: module.id as ModuleId }); onSelect(); })}>{active ? <motion.span className="nav-indicator" layoutId={horizontal ? 'panel-nav' : 'main-nav'}/> : null}<Icon name={module.icon} size={18}/><span>{module.title}</span><small className="nav-count">{count}</small></button></SortRow>;
      })}<EndDrop kind="nav"/>
    </SortableScope>
  </nav>;
}
