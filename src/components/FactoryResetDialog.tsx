import { useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';
import {
  LibraryTransitionDialog,
  waitForMinimumLibraryTransition,
} from './LibraryTransitionDialog';

interface FactoryResetDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onExport: () => void | Promise<void>;
  onReset: () => void | Promise<void>;
}

export function FactoryResetDialog({
  isOpen,
  onClose,
  onExport,
  onReset,
}: FactoryResetDialogProps) {
  const [confirmation, setConfirmation] = useState('');
  const [isResetting, setIsResetting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const close = () => {
    if (isResetting) return;
    setConfirmation('');
    setError(null);
    onClose();
  };

  const reset = async () => {
    if (confirmation !== 'RESET' || isResetting) return;
    setIsResetting(true);
    setError(null);
    try {
      // A reset is drastic enough that an instant restart reads like a crash. Give
      // the transition time to communicate what is happening before native work
      // and the restart begin, while respecting reduced-motion preferences.
      await waitForMinimumLibraryTransition(performance.now());
      await onReset();
    } catch (resetError) {
      console.error('Factory reset failed:', resetError);
      setError('Pasted could not be reset. Your existing data has not been partially removed.');
      setIsResetting(false);
    }
  };

  if (isResetting) {
    return (
      <LibraryTransitionDialog
        isOpen={isOpen}
        variant="reset"
        title="Resetting Pasted"
        description="Clearing saved data and restoring defaults…"
      />
    );
  }

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={close}
      labelledBy="factory-reset-title"
      panelClassName="app-dialog-danger theme-panel w-full max-w-lg rounded-2xl border overflow-hidden font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} closeLabel="Close reset dialog">
          <AppDialogHeading
            id="factory-reset-title"
            title="Reset Pasted?"
            description="Return this installation to its first-launch state."
            icon={<AlertTriangle />}
            tone="danger"
          />
        </AppDialogHeader>
        <AppDialogBody className="space-y-4">
          <div className="theme-status-danger rounded-xl border p-3 text-xs leading-relaxed">
            This permanently deletes clips, Bins, Transforms, connections, activity history, and preferences.
            Full backup files and the original files referenced by clips are not deleted.
          </div>
          <div>
            <label htmlFor="factory-reset-confirmation" className="block text-xs font-semibold theme-title">
              Type <span className="font-mono">RESET</span> to continue
            </label>
            <input
              id="factory-reset-confirmation"
              autoFocus
              autoComplete="off"
              spellCheck={false}
              value={confirmation}
              onChange={(event) => setConfirmation(event.target.value)}
              className="theme-input ui-field-radius mt-2 w-full border px-3 py-2.5 font-mono text-xs focus:outline-none"
              disabled={isResetting}
            />
          </div>
          {error && <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{error}</div>}
        </AppDialogBody>
        <AppDialogFooter align="between">
          <AppDialogButton onClick={() => void onExport()} disabled={isResetting}>Create Full Backup…</AppDialogButton>
          <div className="flex items-center gap-2">
            <AppDialogButton onClick={requestClose} disabled={isResetting}>Cancel</AppDialogButton>
            <AppDialogButton
              variant="danger"
              onClick={() => void reset()}
              disabled={confirmation !== 'RESET' || isResetting}
            >
              {isResetting ? 'Resetting…' : 'Reset Pasted'}
            </AppDialogButton>
          </div>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
