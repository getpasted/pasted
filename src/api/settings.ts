import { safeInvoke as invoke } from '../utils/tauri';

export const settingsApi = {
  load: () => invoke<Record<string, string>>('get_all_app_settings'),
  save: (key: string, value: string) => invoke<void>('save_app_setting', { key, value }),
  saveMany: (values: Record<string, string>) => invoke<void>('save_app_settings', { values }),
};
