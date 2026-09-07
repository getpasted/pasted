import { safeInvoke as invoke } from '../utils/tauri';
import type { FullRestoreReport } from '../api/backup';
import { backupApi } from '../api/backup';
import { collectBackupClientState } from '../utils/backupClientState';
import { waitForMinimumLibraryTransition } from './LibraryTransitionDialog';

export type FullBackupRestoreOutcome = 'cancelled' | 'restored-in-place' | 'restarting';

export async function restoreFullBackupWithTransition(
  backupPath?: string,
): Promise<FullBackupRestoreOutcome> {
  return restoreWithTransition(() => backupApi.restoreFull(collectBackupClientState(), backupPath));
}

export async function restoreSnapshotWithTransition(id: string): Promise<FullBackupRestoreOutcome> {
  return restoreWithTransition(() => invoke<FullRestoreReport | null>('restore_snapshot', {
    id, currentClientStateJson: collectBackupClientState(),
  }));
}

async function restoreWithTransition(run: () => Promise<FullRestoreReport | null>): Promise<FullBackupRestoreOutcome> {
  const transitionStartedAt = performance.now();
  await waitForMinimumLibraryTransition(transitionStartedAt);
  const report = await run();
  if (!report) return 'cancelled';

  const isNative = Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
  if (!isNative) return 'restored-in-place';

  // Packaged builds restart natively. Dev builds reload the webview so the
  // supervisor stays alive. In both cases the caller keeps its transition open.
  if (import.meta.env.DEV) window.location.reload();
  return 'restarting';
}
