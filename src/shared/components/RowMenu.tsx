import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { Icon } from '../icons';
import { IconButton } from './Controls';

export function RowMenu({ title, onEdit, onDelete }: { title: string; onEdit: () => void; onDelete: () => void }) {
  const [position, setPosition] = useState<{ left: number; top: number } | null>(null);
  const trigger = useRef<HTMLDivElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!position) return;
    menu.current?.querySelector<HTMLButtonElement>('button')?.focus();
    const close = () => setPosition(null);
    const outside = (e: PointerEvent) => {
      if (!trigger.current?.contains(e.target as Node) && !menu.current?.contains(e.target as Node)) close();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault(); e.stopImmediatePropagation(); close();
        trigger.current?.querySelector<HTMLButtonElement>('button')?.focus();
      } else if (e.key === 'Tab') {
        close(); trigger.current?.querySelector<HTMLButtonElement>('button')?.focus();
      } else if (['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(e.key)) {
        e.preventDefault();
        const buttons = [...menu.current!.querySelectorAll<HTMLButtonElement>('button')];
        const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
        buttons[e.key === 'Home' ? 0 : e.key === 'End' ? buttons.length - 1 : (current + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length]?.focus();
      }
    };
    document.addEventListener('pointerdown', outside);
    document.addEventListener('keydown', key, true);
    window.addEventListener('resize', close);
    document.addEventListener('scroll', close, true);
    return () => {
      document.removeEventListener('pointerdown', outside);
      document.removeEventListener('keydown', key, true);
      window.removeEventListener('resize', close);
      document.removeEventListener('scroll', close, true);
    };
  }, [position]);
  return <div ref={trigger} className="row-menu-trigger" data-no-drag>
    <IconButton icon="more" label={`更多操作：${title}`} aria-haspopup="menu" aria-expanded={!!position} className={position ? '' : 'row-action'} onClick={e => {
      const rect = e.currentTarget.getBoundingClientRect();
      setPosition(position ? null : { left: Math.max(8, rect.right - 132), top: Math.min(rect.bottom + 4, window.innerHeight - 84) });
    }}/>
    {position ? createPortal(<div ref={menu} className="row-menu" role="menu" aria-label={`${title}的操作`} data-modal data-no-drag style={position}>
      <button role="menuitem" onClick={() => { setPosition(null); onEdit(); }}><Icon name="edit"/>编辑</button>
      <button role="menuitem" className="danger" onClick={() => { setPosition(null); onDelete(); }}><Icon name="trash"/>删除</button>
    </div>, document.body) : null}
  </div>;
}
