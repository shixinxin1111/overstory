import { useEffect, useId, useRef, useState } from 'react';
import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { AnimatePresence, motion } from 'motion/react';
import { Icon } from '../icons';
import type { IconName } from '../icons';
import { springs } from '../motion';

export function Button({ children, className = '', ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <motion.span className="button-motion" whileTap={{ scale: .97 }} transition={springs.feedback}><button className={`button ${className}`} {...props}>{children}</button></motion.span>;
}
export function IconButton({ icon, label, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { icon: IconName; label: string }) {
  return <Button data-no-drag aria-label={label} title={label} {...props} className={`icon-button ${props.className ?? ''}`}><Icon name={icon}/></Button>;
}
export function EmptyState({ icon, title, detail, action }: { icon: IconName; title: string; detail: string; action?: ReactNode }) {
  return <motion.div className="empty-state" initial={{ opacity: 0 }} animate={{ opacity: 1 }}><div className="empty-glyph"><Icon name={icon} size={28}/></div><strong>{title}</strong><p>{detail}</p>{action}</motion.div>;
}
export function SearchField({ value, onChange, placeholder }: { value: string; onChange: (s: string) => void; placeholder: string }) {
  return <div className="search-field"><Icon name="search" size={18}/><input aria-label={placeholder} placeholder={placeholder} value={value} onChange={e => onChange(e.target.value)}/>{value ? <IconButton icon="close" label="清空搜索" onClick={() => onChange('')}/> : null}</div>;
}
export function Dialog({ title, children, onClose }: { title: string; children: ReactNode; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const id = useId();
  const close = useRef(onClose); close.current = onClose;
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const root = ref.current!;
    const focusable = () => [...root.querySelectorAll<HTMLElement>('button:not(:disabled),input,textarea,select,[tabindex="0"]')];
    (root.querySelector<HTMLElement>('[autofocus]') ?? focusable()[1] ?? focusable()[0])?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.isComposing) return;
      if (e.key === 'Escape') { e.preventDefault(); e.stopImmediatePropagation(); close.current(); }
      if (e.key === 'Tab') {
        const list = focusable(); const first = list[0]; const last = list.at(-1);
        if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus(); }
        else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus(); }
      }
    };
    document.addEventListener('keydown', key, true);
    return () => { document.removeEventListener('keydown', key, true); previous?.focus(); };
  }, []);
  return createPortal(<motion.div className="dialog-backdrop" data-modal initial={{ opacity: 0 }} animate={{ opacity: 1 }} onPointerDown={e => { if (e.target === e.currentTarget) onClose(); }}>
    <motion.div className="dialog" ref={ref} role="dialog" aria-modal="true" aria-labelledby={id} onKeyDownCapture={e => { if (e.key === 'Enter' && e.nativeEvent.isComposing) e.preventDefault(); }} initial={{ scale: .96, y: 8 }} animate={{ scale: 1, y: 0 }} transition={springs.content}>
      <header><h2 id={id}>{title}</h2><IconButton icon="close" label="关闭弹层" onClick={onClose}/></header>{children}
    </motion.div>
  </motion.div>, document.body);
}
type ToastMessage = { id: number; message: string; error: boolean };
const toastListeners = new Set<(t: ToastMessage) => void>();
export function toast(message: string, error = false) { const t = { id: Date.now(), message, error }; toastListeners.forEach(fn => fn(t)); }
export function report(error: unknown) {
  toast(typeof error === 'object' && error && 'message' in error ? String(error.message) : String(error), true);
}
export async function run<T>(operation: () => Promise<T>): Promise<T | undefined> {
  try { return await operation(); } catch (e) { report(e); return undefined; }
}
export function ToastHost() {
  const [item, setItem] = useState<ToastMessage | null>(null);
  useEffect(() => { toastListeners.add(setItem); return () => { toastListeners.delete(setItem); }; }, []);
  useEffect(() => { if (!item) return; const timer = setTimeout(() => setItem(null), item.error ? 6500 : 3000); return () => clearTimeout(timer); }, [item]);
  return createPortal(<div className="toast-host" role="status" aria-live="polite"><AnimatePresence>{item ? <motion.div key={item.id} className={`toast ${item.error ? 'error' : ''}`} initial={{ y: 10, opacity: 0 }} animate={{ y: 0, opacity: 1 }} exit={{ y: 5, opacity: 0 }}><span>{item.message}</span><IconButton icon="close" label="关闭提示" onClick={() => setItem(null)}/></motion.div> : null}</AnimatePresence></div>, document.body);
}
export function Confirm({ title, detail, onConfirm, onClose }: { title: string; detail: string; onConfirm: () => Promise<unknown>; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  return <Dialog title={title} onClose={() => { if (!busy) onClose(); }}><p className="dialog-detail">{detail}</p><footer><Button onClick={onClose} disabled={busy}>取消</Button><Button className="danger" disabled={busy} onClick={() => { setBusy(true); onConfirm().then(onClose).catch(report).finally(() => setBusy(false)); }}>{busy ? '处理中…' : '确认'}</Button></footer></Dialog>;
}
