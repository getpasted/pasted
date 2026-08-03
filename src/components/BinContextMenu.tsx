import { Edit3, Trash2 } from 'lucide-react';
import type { Bin } from '../types';

interface BinContextMenuProps {
  menu: { x: number; y: number; bin: Bin };
  onEdit: (bin: Bin) => void;
  onDelete: (bin: Bin) => void;
}

export function BinContextMenu({ menu, onEdit, onDelete }: BinContextMenuProps) {
  return (
    <div
      style={{
        top: Math.min(menu.y, window.innerHeight - 100),
        left: Math.min(menu.x, window.innerWidth - 180),
      }}
      className="bin-context-menu theme-menu fixed min-w-[170px] rounded-xl border p-1.5 text-xs font-medium space-y-0.5 animate-in fade-in zoom-in-95 duration-100"
      onMouseDown={(event) => event.stopPropagation()}
      onClick={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        onClick={() => onEdit(menu.bin)}
        className="w-full flex items-center space-x-2 px-2.5 py-1.5 rounded-md hover:bg-blue-600 hover:text-white transition-colors cursor-pointer"
      >
        <Edit3 className="w-3.5 h-3.5" />
        <span>Edit Bin...</span>
      </button>
      <div className="border-t border-white/10 my-1" />
      <button
        type="button"
        onClick={() => onDelete(menu.bin)}
        className="w-full flex items-center space-x-2 px-2.5 py-1.5 rounded-md text-red-400 hover:bg-red-600 hover:text-white transition-colors cursor-pointer"
      >
        <Trash2 className="w-3.5 h-3.5" />
        <span>Delete Bin</span>
      </button>
    </div>
  );
}
