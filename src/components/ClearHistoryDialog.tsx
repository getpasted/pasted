import { useEffect, useRef } from 'react';
import { AlertTriangle } from 'lucide-react';

export type ClearHistoryMode = 'trash' | 'purge';

interface ClearHistoryDialogProps {
  mode: ClearHistoryMode;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}

export function ClearHistoryDialog({ mode, onCancel, onConfirm }: ClearHistoryDialogProps) {
  const modalRef = useRef<HTMLDivElement>(null);
  const onCancelRef = useRef(onCancel);
  onCancelRef.current = onCancel;

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onCancelRef.current();
        return;
      }

      if (event.key !== 'Tab' || !modalRef.current) return;
      const focusables = modalRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      );
      if (focusables.length === 0) return;

      const firstElement = focusables[0];
      const lastElement = focusables[focusables.length - 1];
      if (event.shiftKey && document.activeElement === firstElement) {
        event.preventDefault();
        lastElement.focus();
      } else if (!event.shiftKey && document.activeElement === lastElement) {
        event.preventDefault();
        firstElement.focus();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div ref={modalRef} className="fixed inset-0 bg-black/75 backdrop-blur-md z-50 flex items-center justify-center p-4 select-none">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="clear-history-title"
        className="app-dialog-panel bg-[#212121] w-full max-w-md rounded-2xl p-6 space-y-4 border border-red-500/40 shadow-2xl text-gray-100 font-sans"
      >
        <div className="flex items-center space-x-3 text-red-400">
          <div className="p-2.5 rounded-xl bg-red-500/20 border border-red-500/30">
            <AlertTriangle className="w-6 h-6" />
          </div>
          <div>
            <h3 id="clear-history-title" className="text-base font-bold text-gray-100">
              {mode === 'purge' ? 'Delete Clipboard History?' : 'Trash Clipboard History?'}
            </h3>
            <p className="text-xs text-gray-400">
              {mode === 'purge' ? 'This action cannot be undone.' : 'Items can be restored from Trash.'}
            </p>
          </div>
        </div>

        <p className="app-dialog-message text-xs text-gray-300 leading-relaxed bg-[#191b22] p-3 rounded-xl border border-gray-700/70">
          {mode === 'purge'
            ? 'Permanently delete all unpinned and unprotected clipboard history? Pinned clips, protected clips, and Bin definitions will be preserved.'
            : 'Move all unpinned and unprotected clipboard history into Trash? Pinned clips, protected clips, and Bin definitions will be preserved.'}
        </p>

        <div className="flex justify-end space-x-3 pt-2">
          <button
            type="button"
            onClick={onCancel}
            autoFocus
            className="app-dialog-cancel px-4 py-2 rounded-xl bg-[#343744] hover:bg-[#3d4150] text-gray-200 text-xs font-semibold transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className="px-4 py-2 rounded-xl bg-red-600 hover:bg-red-500 text-white text-xs font-semibold shadow-lg shadow-red-600/30 transition-all hover:scale-105 active:scale-95"
          >
            {mode === 'purge' ? 'Delete History' : 'Move to Trash'}
          </button>
        </div>
      </div>
    </div>
  );
}
