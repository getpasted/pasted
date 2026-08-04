import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { LoaderCircle, Sparkles, Workflow } from 'lucide-react';
import type { SavedTransform } from '../types';

interface ClipWorkflowMenuProps {
  transforms: SavedTransform[];
  activeTransformRef: string | null;
  isRunning: boolean;
  anchorRef: RefObject<HTMLElement | null>;
  onClose: () => void;
  onPreview: (transform: SavedTransform) => void;
  onManageTransforms: () => void;
}

export function ClipWorkflowMenu({
  transforms,
  activeTransformRef,
  isRunning,
  anchorRef,
  onClose,
  onPreview,
  onManageTransforms,
}: ClipWorkflowMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ left: 8, top: 8, ready: false });

  useLayoutEffect(() => {
    const positionMenu = () => {
      const anchor = anchorRef.current;
      if (!anchor) return;

      const viewportPadding = 8;
      const gap = 8;
      const menuWidth = 288;
      const anchorRect = anchor.getBoundingClientRect();
      const measuredHeight = menuRef.current?.getBoundingClientRect().height ?? 0;
      const left = Math.min(
        Math.max(viewportPadding, anchorRect.right - menuWidth),
        window.innerWidth - menuWidth - viewportPadding,
      );
      const fitsBelow = !measuredHeight
        || anchorRect.bottom + gap + measuredHeight <= window.innerHeight - viewportPadding;
      const top = fitsBelow
        ? anchorRect.bottom + gap
        : Math.max(viewportPadding, anchorRect.top - gap - measuredHeight);

      setPosition({ left, top, ready: true });
    };

    positionMenu();
    window.addEventListener('resize', positionMenu);
    window.addEventListener('scroll', positionMenu, true);
    return () => {
      window.removeEventListener('resize', positionMenu);
      window.removeEventListener('scroll', positionMenu, true);
    };
  }, [anchorRef]);

  useEffect(() => {
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (target instanceof Element && target.closest('.clip-workflow-shell')) return;
      if (!menuRef.current?.contains(target as Node)) onClose();
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };

    window.addEventListener('pointerdown', handlePointerDown);
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('pointerdown', handlePointerDown);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [onClose]);

  return createPortal(
    <div
      ref={menuRef}
      style={{ left: position.left, top: position.top, visibility: position.ready ? 'visible' : 'hidden' }}
      className="theme-menu fixed w-72 rounded-xl border p-1.5 text-xs font-medium select-none"
      role="menu"
      aria-label="Clip workflow"
      onPointerDown={(event) => event.stopPropagation()}
    >
      <div className="overlay-scroll-region max-h-64">
        {transforms.length > 0 ? transforms.map((transform) => {
          const usesIntelligence = transform.plan.steps.some((step) => step.executor.kind === 'semantic');
          const isActive = activeTransformRef === transform.stableRef;
          return (
            <button
              key={transform.stableRef}
              type="button"
              role="menuitem"
              disabled={isRunning}
              onClick={() => {
                onPreview(transform);
                onClose();
              }}
              className={`theme-menu-item flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left disabled:opacity-50 ${isActive ? 'is-selected' : ''}`}
            >
              {isRunning && isActive
                ? <LoaderCircle className="h-4 w-4 shrink-0 animate-spin text-cyan-500" />
                : <Workflow className="h-4 w-4 shrink-0 text-cyan-500" />}
              <span className="min-w-0 flex-1 truncate">{transform.name}</span>
              {usesIntelligence && (
                <Sparkles className="h-3.5 w-3.5 shrink-0 text-violet-400" aria-label="Uses connected intelligence" />
              )}
            </button>
          );
        }) : (
          <p className="theme-text-muted px-2.5 py-3 text-[11px] font-normal">
            No saved Transforms yet.
          </p>
        )}
      </div>

      <div className="theme-menu-divider my-1 border-t" />
      <button
        type="button"
        role="menuitem"
        onClick={() => {
          onManageTransforms();
          onClose();
        }}
        className="theme-menu-item flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left"
      >
        <Workflow className="h-4 w-4 shrink-0 text-cyan-500" />
        <span className="flex-1">Manage Transforms…</span>
      </button>
    </div>,
    document.body,
  );
}
