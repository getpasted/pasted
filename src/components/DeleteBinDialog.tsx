import { useMemo, useState } from 'react';
import { Inbox, Trash2 } from 'lucide-react';
import type { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';

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
          <AppDialogHeading id="delete-bin-title" title={<>Delete Bin &quot;{bin.name}&quot;?</>} icon={<Trash2 />} tone="danger" />
        </AppDialogHeader>
        <AppDialogBody>
          {hasAssignedClips ? (
            <div className="delete-bin-options">
              <p className="theme-text-muted text-xs">
                This Bin contains {bin.clip_count} {bin.clip_count === 1 ? 'clip' : 'clips'}. What should happen to {bin.clip_count === 1 ? 'it' : 'them'}?
              </p>
              <MenuSelect
                value={destination}
                onChange={setDestination}
                label="Move clips to"
                className="delete-bin-destination"
                options={[
                  { value: 'none', label: 'No Bin', icon: <Inbox className="h-4 w-4" /> },
                  { value: 'trash', label: 'Trash', icon: <Trash2 className="h-4 w-4" /> },
                  ...destinationBins.map((candidate) => ({
                    value: `bin:${candidate.id}`,
                    label: candidate.name,
                    group: 'Bins',
                    icon: <span>{formatEmojiIcon(candidate.icon)}</span>,
                  })),
                ]}
              />
              {destination === 'trash' && (
                <p className="theme-text-subtle text-[11px]">Protected clips will be kept in No Bin.</p>
              )}
            </div>
          ) : (
            <p className="theme-text-muted text-xs">
              {bin.smart_rule
                ? 'Clips matched by this Smart Bin will be preserved.'
                : 'This Bin is empty. No clips will be affected.'}
            </p>
          )}
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus disabled={isSubmitting}>Cancel</AppDialogButton>
          <AppDialogButton
            variant="danger"
            disabled={isSubmitting}
            onClick={submit}
          >
            Delete Bin
          </AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
