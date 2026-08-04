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
      className="bin-context-menu theme-menu fixed min-w-[170px] rounded-xl border p-1.5 text-xs font-medium select-none animate-in fade-in zoom-in-95 duration-100"
      onMouseDown={(event) => event.stopPropagation()}
      onClick={(event) => event.stopPropagation()}
      role="menu"
    >
      <button
        type="button"
        onClick={() => onEdit(menu.bin)}
        className="theme-menu-item flex w-full items-center space-x-2 rounded-md px-2.5 py-1.5"
        role="menuitem"
      >
        <Edit3 className="w-3.5 h-3.5" />
        <span>Edit Bin...</span>
      </button>
      <div className="theme-menu-divider my-1 border-t" />
      <button
        type="button"
        onClick={() => onDelete(menu.bin)}
        className="theme-menu-item theme-danger-text flex w-full items-center space-x-2 rounded-md px-2.5 py-1.5"
        role="menuitem"
      >
        <Trash2 className="w-3.5 h-3.5" />
        <span>Delete Bin</span>
      </button>
    </div>
  );
}
