import { Sparkles } from 'lucide-react';

import { translate } from '../localization/runtime';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';
import { authoringRoleLabel, type ExtractorAuthoringSession } from './contentExtractorModel';

export function ExtractorAuthoringHistoryDialog({
  sessions,
  onClose,
}: {
  sessions: ExtractorAuthoringSession[] | null;
  onClose: () => void;
}) {
  return <AppDialog
    isOpen={sessions !== null}
    onClose={onClose}
    labelledBy="extractor-authoring-history-title"
    panelClassName="theme-panel flex max-h-[80vh] w-full max-w-2xl flex-col overflow-hidden border shadow-2xl"
  >
    {({ requestClose }) => <>
      <AppDialogHeader onClose={requestClose}>
        <AppDialogHeading id="extractor-authoring-history-title" title={translate('component.contentExtractorManagerDialog.authoringHistory')} icon={<Sparkles />} />
      </AppDialogHeader>
      <AppDialogBody className="min-h-0 space-y-3 overflow-y-auto">
        {sessions?.length === 0 && <p className="theme-text-muted text-xs">{translate('component.contentExtractorManagerDialog.noAuthoringHistory')}</p>}
        {sessions?.map((session) => <section key={session.id} className="theme-subtle-surface rounded-xl border p-3 text-xs">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <strong>{session.provider
              ? translate('component.contentExtractorManagerDialog.createdWithProvider', { provider: session.provider })
              : translate('component.contentExtractorManagerDialog.createdManually')}</strong>
            <time className="theme-text-muted text-[10px]" dateTime={dateTimeAttribute(session.createdAt)} title={formatFullDateTime(session.createdAt)}>{formatRelativeTime(session.createdAt)}</time>
          </div>
          {session.originalPrompt && <div className="mt-3">
            <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.originalRequest')}</span>
            <p dir="auto" className="mt-1 whitespace-pre-wrap">{session.originalPrompt}</p>
          </div>}
          {session.messages.length > 0 && <div className="mt-3 space-y-2">
            <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.conversation')}</span>
            {session.messages.map((message, index) => <div key={`${message.createdAt}-${index}`} className="theme-surface rounded-lg border p-2.5">
              <div className="theme-text-muted mb-1 flex items-center justify-between gap-2 text-[10px]">
                <span>{authoringRoleLabel(message.role)}</span>
                <time dateTime={dateTimeAttribute(message.createdAt)} title={formatFullDateTime(message.createdAt)}>{formatRelativeTime(message.createdAt)}</time>
              </div>
              <p dir="auto" className="whitespace-pre-wrap">{message.content}</p>
            </div>)}
          </div>}
        </section>)}
      </AppDialogBody>
      <AppDialogFooter align="end"><AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton></AppDialogFooter>
    </>}
  </AppDialog>;
}
