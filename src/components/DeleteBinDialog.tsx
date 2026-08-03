import { Trash2 } from 'lucide-react';
import type { Bin } from '../types';

interface DeleteBinDialogProps {
  bin: Bin;
  onCancel: () => void;
  onConfirm: (bin: Bin) => void | Promise<void>;
}

export function DeleteBinDialog({ bin, onCancel, onConfirm }: DeleteBinDialogProps) {
  return (
    <div className="app-dialog-overlay fixed inset-0 flex items-center justify-center p-4 animate-in fade-in duration-150">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="delete-bin-title"
        className="app-dialog-panel app-dialog-danger theme-panel border rounded-2xl p-5 max-w-sm w-full space-y-4"
      >
        <div className="flex items-center space-x-3">
          <div className="p-2.5 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 shrink-0">
            <Trash2 className="w-5 h-5" />
          </div>
          <div>
            <h3 id="delete-bin-title" className="theme-title text-sm font-bold">Delete Bin &quot;{bin.name}&quot;?</h3>
            <p className="theme-text-muted text-xs mt-0.5">Clips in this bin will be unassigned and preserved.</p>
          </div>
        </div>

        <div className="flex justify-end space-x-2 pt-2">
          <button
            type="button"
            onClick={onCancel}
            autoFocus
            className="app-dialog-cancel theme-secondary-button px-4 py-1.5 rounded-xl border text-xs font-semibold transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => onConfirm(bin)}
            className="px-4 py-1.5 rounded-xl bg-red-600 hover:bg-red-500 text-white text-xs font-semibold transition-colors shadow-md cursor-pointer"
          >
            Delete Bin
          </button>
        </div>
      </div>
    </div>
  );
}
