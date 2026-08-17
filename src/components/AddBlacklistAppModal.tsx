import { useState, type FormEvent } from 'react';
import { Lock } from 'lucide-react';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';
import { translate } from '../localization/runtime';

interface AddBlacklistAppModalProps {
  suggestions: Array<{ label: string; apps: string[] }>;
  onAdd: (appName: string) => void;
  onClose: () => void;
}

export function AddBlacklistAppModal({ suggestions, onAdd, onClose }: AddBlacklistAppModalProps) {
  const [appName, setAppName] = useState('');

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const name = appName.trim();
    if (!name) return;
    onAdd(name);
    onClose();
  };

  return (
    <AppDialog
      isOpen
      onClose={onClose}
      labelledBy="add-blacklist-app-title"
      isDirty={Boolean(appName.trim())}
      panelClassName="theme-panel border rounded-2xl w-full max-w-md shadow-2xl overflow-hidden"
    >
      {({ requestClose }) => (
        <>
          <AppDialogHeader onClose={requestClose} closeLabel={translate('component.addBlacklistAppModal.closeAddAppDialog')}>
            <AppDialogHeading
              id="add-blacklist-app-title"
              title={translate('component.addBlacklistAppModal.addAppExclusion')}
              description={translate('component.addBlacklistAppModal.ignoreTheSelectedContentAndHotkeysWhileThisAppIsActive')}
              icon={<Lock />}
            />
          </AppDialogHeader>
          <form onSubmit={submit}>
            <AppDialogBody className="space-y-4">
              <div className="space-y-1.5 text-xs theme-text-muted">
                <span className="block font-semibold">{translate('component.addBlacklistAppModal.suggestedApp')}</span>
                <MenuSelect
                  label={translate('component.addBlacklistAppModal.suggestedApp')}
                  onChange={setAppName}
                  value={suggestions.some((group) => group.apps.includes(appName)) ? appName : ''}
                  options={[
                    { value: '', get label() { return translate('component.addBlacklistAppModal.selectAnInstalledOrPopularApp'); }, disabled: true },
                    ...suggestions.flatMap((group) => group.apps.map((name) => ({
                      value: name,
                      label: name,
                      group: group.label,
                    }))),
                  ]}
                  className="w-full"
                />
              </div>
              <label className="block space-y-1.5 text-xs theme-text-muted">
                <span className="block font-semibold">{translate('component.addBlacklistAppModal.appName')}</span>
                <input
                  autoFocus
                  type="text"
                  placeholder={translate('component.addBlacklistAppModal.signalBitwardenTerminal')}
                  value={appName}
                  onChange={(event) => setAppName(event.target.value)}
                  className="theme-input ui-field-radius w-full border px-3 py-2.5 focus:outline-none"
                />
              </label>
            </AppDialogBody>
            <AppDialogFooter>
              <AppDialogButton onClick={requestClose}>{translate('common.cancel')}</AppDialogButton>
              <AppDialogButton type="submit" variant="primary" disabled={!appName.trim()}>
                {translate('component.addBlacklistAppModal.addApp')}
              </AppDialogButton>
            </AppDialogFooter>
          </form>
        </>
      )}
    </AppDialog>
  );
}
