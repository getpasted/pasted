import { useEffect, useState } from 'react';
import { CheckCircle2, Copy, Scale } from 'lucide-react';
import type { ThirdPartyLicenseDocument } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

export function OpenSourceLicensesDialog({
  isOpen,
  onClose,
}: {
  isOpen: boolean;
  onClose: () => void;
}) {
  const [document, setDocument] = useState<ThirdPartyLicenseDocument | null>(null);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!isOpen || document) return;
    invoke<ThirdPartyLicenseDocument>('get_third_party_licenses')
      .then(setDocument)
      .catch((reason) => setError(String(reason)));
  }, [document, isOpen]);

  const copyNotices = async () => {
    if (!document) return;
    await navigator.clipboard.writeText(document.noticeText);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1_500);
  };

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="open-source-licenses-title"
      panelClassName="theme-panel flex max-h-[85vh] w-full max-w-3xl flex-col overflow-hidden rounded-2xl border font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} closeLabel="Close open-source licenses">
          <AppDialogHeading
            id="open-source-licenses-title"
            title="Open Source Licenses"
            description={document
              ? `${document.componentCount} bundled Rust and JavaScript components`
              : 'Licenses and acknowledgements for bundled software.'}
            icon={<Scale />}
            tone="info"
          />
        </AppDialogHeader>
        <AppDialogBody>
          {document ? (
            <pre className="theme-card-idle select-text whitespace-pre-wrap break-words border p-4 font-mono text-[10px] leading-relaxed">
              {document.noticeText}
            </pre>
          ) : error ? (
            <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">
              Could not load the bundled license notices: {error}
            </div>
          ) : (
            <div className="theme-text-muted p-6 text-center text-xs">Loading bundled notices…</div>
          )}
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={() => void copyNotices()} disabled={!document}>
            {copied ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? 'Copied' : 'Copy Notices'}
          </AppDialogButton>
          <AppDialogButton variant="primary" onClick={requestClose}>Done</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
