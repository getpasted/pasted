import { useEffect, useState } from 'react';
import { Info, X } from 'lucide-react';
import { useLocalization } from '../localization/LocalizationProvider';
import { safeInvoke as invoke } from '../utils/tauri';

interface LibraryRecoveryNotice {
  outcome: 'recovered' | 'fresh' | 'continued';
  occurredAt: string;
  previousPath: string | null;
  recoveryCreatedAt: string | null;
  preservedPath: string;
}

export function LibraryRecoveryNote() {
  const { t, formatDateTime } = useLocalization();
  const [notice, setNotice] = useState<LibraryRecoveryNotice | null>(null);
  useEffect(() => {
    let cancelled = false;
    void invoke<LibraryRecoveryNotice | null>('get_library_recovery_notice')
      .then(value => { if (!cancelled) setNotice(value); })
      .catch(error => console.error('Could not read the library recovery note:', error));
    return () => { cancelled = true; };
  }, []);
  const [dismissing, setDismissing] = useState(false);
  const dismiss = async () => {
    if (!notice) return;
    setDismissing(true);
    try {
      await invoke('dismiss_library_recovery_notice', { occurredAt: notice.occurredAt, preservedPath: notice.preservedPath });
      setNotice(null);
    } catch (error) { console.error('Could not dismiss the library recovery note:', error); }
    finally { setDismissing(false); }
  };
  if (!notice) return null;
  const description = notice.outcome === 'recovered' && notice.recoveryCreatedAt
    ? t('libraryRecovery.restoredNote', { date: formatDateTime(notice.recoveryCreatedAt, { dateStyle: 'medium', timeStyle: 'short' }) })
    : notice.outcome === 'continued' ? t('libraryRecovery.continuedNote') : t('libraryRecovery.freshNote');
  const title = notice.outcome === 'recovered' ? t('libraryRecovery.restoredTitle')
    : notice.outcome === 'continued' ? t('libraryRecovery.continuedTitle') : t('libraryRecovery.freshTitle');
  return <aside className="theme-status-info flex gap-3 rounded-xl border p-3" aria-labelledby="library-recovery-note-title">
    <Info className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
    <div className="min-w-0 flex-1 space-y-1">
      <h3 id="library-recovery-note-title" className="theme-text-main text-xs font-semibold">
        {title}
      </h3>
      <p className="theme-text-muted text-xs leading-relaxed">{description}</p>
      <details className="theme-text-muted pt-1 text-xs">
        <summary className="cursor-pointer">{t('libraryRecovery.details')}</summary>
        <dl className="mt-2 space-y-2">
          {notice.previousPath && <div><dt>{t('libraryRecovery.previousLocation')}</dt><dd className="mt-1 select-text break-all font-mono" dir="ltr">{notice.previousPath}</dd></div>}
          <div><dt>{t('libraryRecovery.preservedFiles')}</dt><dd className="mt-1 select-text break-all font-mono" dir="ltr">{notice.preservedPath}</dd></div>
        </dl>
      </details>
    </div>
    <button type="button" onClick={() => { void dismiss(); }} disabled={dismissing}
      className="theme-icon-button -me-1 -mt-1 flex h-7 w-7 shrink-0 items-center justify-center rounded-md"
      aria-label={t('common.dismiss')} title={t('common.dismiss')}>
      <X className="h-4 w-4" aria-hidden="true" />
    </button>
  </aside>;
}
