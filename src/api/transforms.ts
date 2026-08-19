import type { ManualTransform } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

export interface ManualTransformInput {
  name: string;
  steps: Array<{
    operationRef: string;
    configJson: string | null;
    failurePolicy: 'stop' | 'skip';
  }>;
  hotkey: string | null;
}

export const transformsApi = {
  listManual: () => invoke<ManualTransform[]>('get_manual_transforms'),
  createManual: ({ name, steps, hotkey }: ManualTransformInput) =>
    invoke<ManualTransform>('create_manual_transform', { name, steps, hotkey }),
  updateManual: (transformRef: string, input: ManualTransformInput) =>
    invoke<ManualTransform>('update_manual_transform', { transformRef, ...input }),
  updateManualHotkey: (transformRef: string, hotkey: string | null) =>
    invoke<void>('update_manual_transform_hotkey', { transformRef, hotkey }),
  deleteManual: (transformRef: string) =>
    invoke<void>('delete_manual_transform', { transformRef }),
};
