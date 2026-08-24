import { useEffect } from 'react';

export function useAuxiliaryWindowReady(ready: boolean) {
  useEffect(() => {
    if (!ready) {
      delete document.documentElement.dataset.windowReady;
      return undefined;
    }
    const frame = window.requestAnimationFrame(() => {
      document.documentElement.dataset.windowReady = 'true';
    });
    return () => {
      window.cancelAnimationFrame(frame);
      delete document.documentElement.dataset.windowReady;
    };
  }, [ready]);
}
