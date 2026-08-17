import { useMemo, useState } from 'react';
import { Inbox, Trash2 } from 'lucide-react';
import type { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';
import { translate } from '../localization/runtime';

export type BinDeleteDisposition = 'keep' | 'trash' | 'move';

interface DeleteBinDialogProps {
  bin: Bin;
  bins: Bin[];
  onCancel: () => void;
  onConfirm: (bin: Bin, disposition: BinDeleteDisposition, destinationBinId?: number) => void | Promise<void>;
}

export function DeleteBinDialog({ bin, bins, onCancel, onConfirm }: DeleteBinDialogProps) {
  const hasAssignedClips = !bin.smart_rule && (bin.clip_count ?? 0) > 0;
  const destinationBins = useMemo(
    () => bins.filter((candidate) => (
      candidate.id !== bin.id
      && !candidate.smart_rule
      && candidate.bin_type !== 'tag'
    )),
    [bin.id, bins],
  );
  const [destination, setDestination] = useState('none');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const submit = async () => {
    if (isSubmitting) return;
    const disposition: BinDeleteDisposition = destination === 'trash'
      ? 'trash'
      : destination.startsWith('bin:')
        ? 'move'
        : 'keep';
    const destinationBinId = disposition === 'move'
      ? Number(destination.slice('bin:'.length))
      : undefined;
    setIsSubmitting(true);
    try {
      await onConfirm(
        bin,
        hasAssignedClips ? disposition : 'keep',
        destinationBinId,
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy="delete-bin-title"
      panelClassName="app-dialog-danger theme-panel border rounded-2xl max-w-md w-full overflow-hidden"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading id="delete-bin-title" title={translate('component.deleteBinDialog.deleteNamedBin', { name: bin.name })} icon={<Trash2 />} tone="danger" />
        </AppDialogHeader>
        <AppDialogBody>
          {hasAssignedClips ? (
            <div className="delete-bin-options">
              <p className="theme-text-muted text-xs">{translate('component.deleteBinDialog.binContentsQuestion', { count: bin.clip_count ?? 0 })}</p>
              <MenuSelect
                value={destination}
                onChange={setDestination}
                label={translate('component.deleteBinDialog.moveClipsTo')}
                className="delete-bin-destination"
                options={[
                  { value: 'none', get label() { return translate('common.noBin'); }, icon: <Inbox className="h-4 w-4" /> },
                  { value: 'trash', get label() { return translate('component.deleteBinDialog.trash'); }, icon: <Trash2 className="h-4 w-4" /> },
                  ...destinationBins.map((candidate) => ({
                    value: `bin:${candidate.id}`,
                    label: candidate.name,
                    group: translate('component.sidebar.bins'),
                    icon: <span>{formatEmojiIcon(candidate.icon)}</span>,
                  })),
                ]}
              />
              {destination === 'trash' && (
                <p className="theme-text-subtle text-[11px]">{translate('component.deleteBinDialog.protectedClipsWillBeKeptInNoBin')}</p>
              )}
            </div>
          ) : (
            <p className="theme-text-muted text-xs">
              {bin.smart_rule
                ? translate('component.deleteBinDialog.clipsMatchedByThisSmartBinWillBePreserved')
                : translate('component.deleteBinDialog.thisBinIsEmptyNoClipsWillBeAffected')}
            </p>
          )}
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus disabled={isSubmitting}>{translate('common.cancel')}</AppDialogButton>
          <AppDialogButton
            variant="danger"
            disabled={isSubmitting}
            onClick={submit}
          >
            {translate('component.deleteBinDialog.deleteBin')}
          </AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
