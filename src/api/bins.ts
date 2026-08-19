import type { Bin } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

export interface BinInput {
  name: string;
  icon: string;
  color: string;
  smartRule: string | null;
}

export const binsApi = {
  list: () => invoke<Bin[]>('get_bins'),
  create: (input: BinInput) => invoke<Bin>('create_bin', { ...input }),
  update: (id: number, input: BinInput) => invoke<void>('update_bin', { id, ...input }),
  delete: (id: number, disposition: string, destinationBinId?: number | null) =>
    invoke<void>('delete_bin', { id, disposition, destinationBinId }),
  updateHotkey: (id: number, hotkey: string | null) => invoke<void>('update_bin_hotkey', { id, hotkey }),
  updateProtection: (id: number, protectClips: boolean) =>
    invoke<void>('update_bin_protection', { id, protectClips }),
  setTransform: (binId: number, transformRef: string | null) =>
    invoke<void>('set_bin_transform_ref', { binId, transformRef }),
};
