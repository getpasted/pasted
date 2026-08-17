import type { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { translate } from '../localization/runtime';

interface ClipBinSummaryProps {
  bins: Bin[];
  primaryBinId: number | null;
}

export function ClipBinSummary({ bins, primaryBinId }: ClipBinSummaryProps) {
  if (bins.length === 0) return null;
  const primaryBin = bins.find((bin) => bin.id === primaryBinId)
    ?? bins.find((bin) => !bin.smart_rule)
    ?? bins[0];
  const names = bins.map((bin) => bin.name).join(', ');

  return (
    <span
      role="img"
      aria-label={translate('component.clipBinSummary.binsNames', { names: names })}
      className="clip-meta-item clip-meta-icon-only clip-bin-emoji"
    >
      {formatEmojiIcon(primaryBin.icon)}
    </span>
  );
}
