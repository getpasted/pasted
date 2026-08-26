import { useEffect } from 'react';
import { soundManager } from '../utils/sound';

export function useSoundSettings(enabled: boolean) {
  useEffect(() => soundManager.setEnabled(enabled), [enabled]);
}
