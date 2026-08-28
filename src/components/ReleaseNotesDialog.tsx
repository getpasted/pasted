import { FileText } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

import { translate } from '../localization/runtime';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

export function ReleaseNotesDialog({
  isOpen,
  notes,
  version,
  onClose,
}: {
  isOpen: boolean;
  notes: string;
  version: string;
  onClose: () => void;
}) {
  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="release-notes-title"
      panelClassName="theme-panel flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader
          onClose={requestClose}
          closeLabel={translate('component.settingsUpdateSection.closeReleaseNotes')}
        >
          <AppDialogHeading
            id="release-notes-title"
            title={translate('component.settingsUpdateSection.changesInVersion', { version })}
            icon={<FileText />}
            tone="info"
          />
        </AppDialogHeader>
        <AppDialogBody className="select-text overflow-y-auto">
          <div className="release-notes">
            <ReactMarkdown
              remarkPlugins={[remarkGfm]}
              components={{
                a: ({ children, ...props }) => (
                  <a {...props} target="_blank" rel="noreferrer">{children}</a>
                ),
              }}
            >
              {notes}
            </ReactMarkdown>
          </div>
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton variant="primary" onClick={requestClose} autoFocus>
            {translate('common.done')}
          </AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
