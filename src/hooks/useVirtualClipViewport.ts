import { useLayoutEffect, useState, type RefObject } from 'react';

interface VirtualClipViewport {
  height: number;
  scrollTop: number;
}

export function useVirtualClipViewport(
  scrollRef: RefObject<HTMLDivElement | null>,
  disabled: boolean,
  layoutSize: number,
): VirtualClipViewport {
  const [viewport, setViewport] = useState<VirtualClipViewport>({ height: 800, scrollTop: 0 });

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element || disabled) return undefined;
    let frame = 0;
    const update = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        setViewport({ height: element.clientHeight, scrollTop: element.scrollTop });
      });
    };
    update();
    element.addEventListener('scroll', update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
      element.removeEventListener('scroll', update);
    };
  }, [disabled, scrollRef]);

  useLayoutEffect(() => {
    const element = scrollRef.current;
    if (!element || disabled) return;
    const next = { height: element.clientHeight, scrollTop: element.scrollTop };
    setViewport((current) => (
      current.height === next.height && current.scrollTop === next.scrollTop ? current : next
    ));
  }, [disabled, layoutSize, scrollRef]);

  return viewport;
}
