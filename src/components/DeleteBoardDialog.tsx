import { Trash2 } from 'lucide-react';
import type { Board } from '../types';

interface DeleteBoardDialogProps {
  board: Board;
  onCancel: () => void;
  onConfirm: (board: Board) => void | Promise<void>;
}

export function DeleteBoardDialog({ board, onCancel, onConfirm }: DeleteBoardDialogProps) {
  return (
    <div className="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-md flex items-center justify-center p-4 animate-in fade-in duration-150">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="delete-board-title"
        className="app-dialog-panel bg-[#212121] border border-gray-700/80 rounded-2xl p-5 max-w-sm w-full shadow-2xl space-y-4"
      >
        <div className="flex items-center space-x-3">
          <div className="p-2.5 rounded-xl bg-red-500/10 border border-red-500/20 text-red-400 shrink-0">
            <Trash2 className="w-5 h-5" />
          </div>
          <div>
            <h3 id="delete-board-title" className="text-sm font-bold text-gray-100">Delete Bin &quot;{board.name}&quot;?</h3>
            <p className="text-xs text-gray-400 mt-0.5">Clips in this bin will be unassigned and preserved.</p>
          </div>
        </div>

        <div className="flex justify-end space-x-2 pt-2">
          <button
            type="button"
            onClick={onCancel}
            autoFocus
            className="app-dialog-cancel px-4 py-1.5 rounded-xl bg-[#343744] hover:bg-[#3d4150] text-gray-200 text-xs font-semibold transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => onConfirm(board)}
            className="px-4 py-1.5 rounded-xl bg-red-600 hover:bg-red-500 text-white text-xs font-semibold transition-colors shadow-md cursor-pointer"
          >
            Delete Bin
          </button>
        </div>
      </div>
    </div>
  );
}
