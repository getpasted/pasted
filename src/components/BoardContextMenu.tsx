import { Edit3, Trash2 } from 'lucide-react';
import type { Board } from '../types';

interface BoardContextMenuProps {
  menu: { x: number; y: number; board: Board };
  onEdit: (board: Board) => void;
  onDelete: (board: Board) => void;
}

export function BoardContextMenu({ menu, onEdit, onDelete }: BoardContextMenuProps) {
  return (
    <div
      style={{
        top: Math.min(menu.y, window.innerHeight - 100),
        left: Math.min(menu.x, window.innerWidth - 180),
      }}
      className="board-context-menu fixed z-[9999] min-w-[170px] glass-hud rounded-xl p-1.5 shadow-2xl text-xs font-medium space-y-0.5 animate-in fade-in zoom-in-95 duration-100"
      onMouseDown={(event) => event.stopPropagation()}
      onClick={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        onClick={() => onEdit(menu.board)}
        className="w-full flex items-center space-x-2 px-2.5 py-1.5 rounded-md hover:bg-blue-600 hover:text-white transition-colors cursor-pointer"
      >
        <Edit3 className="w-3.5 h-3.5" />
        <span>Edit Bin...</span>
      </button>
      <div className="border-t border-white/10 my-1" />
      <button
        type="button"
        onClick={() => onDelete(menu.board)}
        className="w-full flex items-center space-x-2 px-2.5 py-1.5 rounded-md text-red-400 hover:bg-red-600 hover:text-white transition-colors cursor-pointer"
      >
        <Trash2 className="w-3.5 h-3.5" />
        <span>Delete Bin</span>
      </button>
    </div>
  );
}
