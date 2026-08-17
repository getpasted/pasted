import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { AlertTriangle, CheckCircle2, Info, X } from 'lucide-react';
import { translate } from '../localization/runtime';

export type ToastTone = 'success' | 'error' | 'info';

export interface ToastInput {
  message: string;
  tone?: ToastTone;
  durationMs?: number;
}

interface ToastItem extends Required<ToastInput> {
  id: number;
}

interface ToastApi {
  showToast: (toast: ToastInput) => number;
  dismissToast: (id: number) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const nextId = useRef(1);
  const timers = useRef(new Map<number, number>());
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  useEffect(() => () => {
    timers.current.forEach((timer) => window.clearTimeout(timer));
    timers.current.clear();
  }, []);

  const dismissToast = useCallback((id: number) => {
    const timer = timers.current.get(id);
    if (timer !== undefined) window.clearTimeout(timer);
    timers.current.delete(id);
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const showToast = useCallback((input: ToastInput) => {
    const id = nextId.current++;
    const toast: ToastItem = {
      id,
      message: input.message,
      tone: input.tone ?? 'info',
      durationMs: input.durationMs ?? 5000,
    };
    setToasts((current) => {
      const next = [...current.slice(-3), toast];
      const retainedIds = new Set(next.map((item) => item.id));
      current.forEach((item) => {
        if (retainedIds.has(item.id)) return;
        const timer = timers.current.get(item.id);
        if (timer !== undefined) window.clearTimeout(timer);
        timers.current.delete(item.id);
      });
      return next;
    });
    if (toast.durationMs > 0) {
      timers.current.set(id, window.setTimeout(() => dismissToast(id), toast.durationMs));
    }
    return id;
  }, [dismissToast]);

  const api = useMemo(() => ({ showToast, dismissToast }), [dismissToast, showToast]);

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="app-toast-region" aria-live="polite" aria-relevant="additions removals">
        {toasts.map((toast) => {
          const Icon = toast.tone === 'success' ? CheckCircle2 : toast.tone === 'error' ? AlertTriangle : Info;
          return (
            <div key={toast.id} className={`app-toast theme-status-${toast.tone === 'error' ? 'danger' : toast.tone}`} role={toast.tone === 'error' ? 'alert' : 'status'}>
              <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span className="min-w-0 flex-1 text-xs font-semibold leading-relaxed">{toast.message}</span>
              <button type="button" onClick={() => dismissToast(toast.id)} className="app-toast-close" aria-label={translate('component.toastProvider.dismissNotification')}>
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const api = useContext(ToastContext);
  if (!api) throw new Error('useToast must be used within ToastProvider');
  return api;
}
