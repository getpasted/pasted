import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

export function useAppEvent<T>(
  eventName: string,
  handler: (payload: T) => void,
  enabled = true,
) {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!enabled) return undefined;
    let disposed = false;
    let stop: (() => void) | undefined;
    void listen<T>(eventName, ({ payload }) => handlerRef.current(payload))
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      })
      .catch(console.error);
    return () => {
      disposed = true;
      stop?.();
    };
  }, [enabled, eventName]);
}
