import { Edit3, Trash2 } from 'lucide-react';
import type { Bin } from '../types';
import { AnchoredMenu, MenuDivider, MenuItem } from './AnchoredMenu';
import { translate } from '../localization/runtime';

interface BinContextMenuProps {
  menu: { x: number; y: number; bin: Bin };
  onClose: () => void;
  onEdit: (bin: Bin) => void;
  onDelete: (bin: Bin) => void;
}

export function BinContextMenu({ menu, onClose, onEdit, onDelete }: BinContextMenuProps) {
  return (
    <AnchoredMenu
      anchor={{ kind: 'point', x: menu.x, y: menu.y }}
      ariaLabel={translate('component.binContextMenu.nameActions', { name: menu.bin.name })}
      className="bin-context-menu min-w-[170px]"
      onClose={onClose}
    >
      <MenuItem
        onClick={() => onEdit(menu.bin)}
        className="gap-2 px-2.5 py-1.5"
      >
        <Edit3 className="theme-status-info-text w-3.5 h-3.5" />
        <span>{translate('component.binContextMenu.editBin')}</span>
      </MenuItem>
      <MenuDivider />
      <MenuItem
        danger
        onClick={() => onDelete(menu.bin)}
        className="gap-2 px-2.5 py-1.5"
      >
        <Trash2 className="theme-danger-text w-3.5 h-3.5" />
        <span>{translate('component.binContextMenu.deleteBin')}</span>
      </MenuItem>
    </AnchoredMenu>
  );
}
