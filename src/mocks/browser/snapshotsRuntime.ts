import { unhandledValue } from './result';

const deleted = new Set<string>();
const createdSnapshots: Array<{ id: string; createdAt: string; clipCount: number; sizeBytes: number; sha256: string }> = [];

export function invokeSnapshotsBrowserMock<T>(cmd: string, args?: Record<string, unknown>): T | typeof unhandledValue {
  switch (cmd) {
    case 'get_snapshot_status':
      return { automaticCreationFailed: false, nextAutomaticSnapshotAt: new Date(Date.now() + 3600000).toISOString(), waitingForNewClips: false } as T;
    case 'enforce_snapshot_retention':
      return undefined as T;
    case 'list_snapshots':
      return [...createdSnapshots, ...[0, 1, 2].map(index => ({
        id: `100-${index}`, createdAt: new Date(Date.now() - index * 3600000).toISOString(),
        clipCount: 42 - index * 3, sizeBytes: 1450000, sha256: 'mock',
      }))].filter(snapshot => !deleted.has(snapshot.id)) as T;
    case 'delete_snapshot':
      deleted.add(String(args?.id));
      return undefined as T;
    case 'create_snapshot': {
      const snapshot = { id: `100-${Date.now()}`, createdAt: new Date().toISOString(), clipCount: 42, sizeBytes: 1450000, sha256: 'mock' };
      createdSnapshots.unshift(snapshot);
      return snapshot as T;
    }
    case 'export_snapshot':
      return { path: '/mock/Pasted_Snapshot.pastedbackup', createdAt: new Date().toISOString(), sizeBytes: 1450000 } as T;
    case 'restore_snapshot':
      return { recoveryPath: '/mock/Pasted/pre-restore.pastedbackup', backupCreatedAt: new Date().toISOString() } as T;
    default: return unhandledValue;
  }
}
