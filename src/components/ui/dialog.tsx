import { useEffect, useRef, type ReactNode } from 'react';
import { X } from 'lucide-react';
import { Button } from './button';
export function Dialog({ open, title, children, onClose }: { open: boolean; title: string; children: ReactNode; onClose: () => void }) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => { const dialog = ref.current; if (open && dialog && !dialog.open) dialog.showModal(); if (!open && dialog?.open) dialog.close(); }, [open]);
  return <dialog ref={ref} aria-labelledby="dialog-title" onCancel={(event) => { event.preventDefault(); onClose(); }} onClick={(event) => { if (event.target === event.currentTarget) onClose(); }}><div className="dialog-heading"><h2 id="dialog-title">{title}</h2><Button variant="ghost" size="icon" aria-label="Close dialog" onClick={onClose}><X size={18}/></Button></div>{children}</dialog>;
}
