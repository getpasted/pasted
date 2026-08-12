import { useRef, useState } from 'react';
import { Check, ChevronDown, Folder, FolderX, Sparkles } from 'lucide-react';
import type { Bin } from '../types';
import { binTextColor } from '../utils/binColor';
import { formatEmojiIcon } from '../utils/emoji';
import { AnchoredMenu, MenuDivider, MenuItem } from './AnchoredMenu';
import { OverflowText } from './OverflowText';

interface ClipBinPickerProps {
  bins: Bin[];
  selectedBinIds: number[];
  viewedBinId?: number | null;
  onClear: () => void;
  onToggle: (binId: number, selected: boolean) => void;
}

export function ClipBinPicker({ bins, selectedBinIds, viewedBinId, onClear, onToggle }: ClipBinPickerProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const selected = new Set(selectedBinIds);
  const manualBins = bins.filter((bin) => !bin.smart_rule);
  const smartBins = bins.filter((bin) => Boolean(bin.smart_rule));
  const selectedCount = bins.filter((bin) => selected.has(bin.id)).length;
  const label = selectedCount === 0 ? 'No Bin' : `${selectedCount} Bin${selectedCount === 1 ? '' : 's'}`;

  const renderBin = (bin: Bin, smart: boolean) => {
    const active = selected.has(bin.id);
    return (
      <MenuItem
        key={bin.id}
        role="menuitemcheckbox"
        aria-checked={active}
        active={active}
        disabled={smart}
        title={smart ? 'Smart Bin membership is managed automatically' : undefined}
        className="gap-2 px-2.5 py-2"
        onClick={() => {
          const nextSelected = !active;
          if (!nextSelected && bin.id === viewedBinId) setIsOpen(false);
          onToggle(bin.id, nextSelected);
        }}
      >
        <span className="grid h-4 w-4 shrink-0 place-items-center" aria-hidden="true">
          {formatEmojiIcon(bin.icon)}
        </span>
        <OverflowText
          text={bin.name}
          className="min-w-0 flex-1 truncate"
          style={{ color: binTextColor(bin.color) }}
        />
        {smart && <Sparkles className="theme-intelligence-text h-3 w-3 shrink-0" aria-hidden="true" />}
        <span className="grid h-3.5 w-3.5 shrink-0 place-items-center" aria-hidden="true">
          {active && <Check className="h-3.5 w-3.5" />}
        </span>
      </MenuItem>
    );
  };

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="menu-select-trigger clip-bin-picker theme-focusable flex min-w-0 items-center gap-2 border px-2.5 text-left ui-field-radius"
        aria-label="Choose Bins"
        aria-haspopup="menu"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((open) => !open)}
      >
        <Folder className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        <span className="min-w-0 flex-1 truncate py-2 text-xs font-semibold">{label}</span>
        <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
      </button>

      {isOpen && (
        <AnchoredMenu
          anchor={{ kind: 'element', ref: triggerRef, align: 'start' }}
          ariaLabel="Choose Bins"
          onClose={() => setIsOpen(false)}
          className="max-h-80 overflow-y-auto"
          style={{
            width: Math.min(
              Math.max(triggerRef.current?.getBoundingClientRect().width ?? 240, 240),
              window.innerWidth - 16,
            ),
          }}
        >
          <MenuItem
            role="menuitemcheckbox"
            aria-checked={manualBins.every((bin) => !selected.has(bin.id))}
            active={manualBins.every((bin) => !selected.has(bin.id))}
            className="gap-2 px-2.5 py-2"
            onClick={() => {
              if (viewedBinId !== null && viewedBinId !== undefined && selected.has(viewedBinId)) {
                setIsOpen(false);
              }
              onClear();
            }}
          >
            <FolderX className="h-4 w-4 shrink-0" aria-hidden="true" />
            <span className="min-w-0 flex-1">No Bin</span>
          </MenuItem>
          {manualBins.length > 0 && (
            <>
              <div className="theme-text-subtle px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider">Manual Bins</div>
              {manualBins.map((bin) => renderBin(bin, false))}
            </>
          )}
          {smartBins.length > 0 && (
            <>
              <MenuDivider />
              <div className="theme-text-subtle flex items-center gap-1.5 px-2.5 pb-1 pt-1 text-[10px] font-bold uppercase tracking-wider">
                <Sparkles className="h-3 w-3" aria-hidden="true" />
                Smart Bins · Automatic
              </div>
              {smartBins.map((bin) => renderBin(bin, true))}
            </>
          )}
        </AnchoredMenu>
      )}
    </>
  );
}
