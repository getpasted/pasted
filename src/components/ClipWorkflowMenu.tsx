import type { RefObject } from 'react';
import { LoaderCircle, Sparkles, Workflow } from 'lucide-react';
import type { SavedTransform } from '../types';
import { AnchoredMenu, MenuDivider, MenuItem } from './AnchoredMenu';
import { OverflowText } from './OverflowText';

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
  return (
    <AnchoredMenu
      anchor={{ kind: 'element', ref: anchorRef, align: 'end', gap: 8 }}
      ariaLabel="Clip workflow"
      onClose={onClose}
      className="w-72"
    >
      <div className="overlay-scroll-region max-h-64">
        {transforms.length > 0 ? transforms.map((transform) => {
          const usesIntelligence = transform.plan.steps.some((step) => step.executor.kind === 'semantic');
          const isActive = activeTransformRef === transform.stableRef;
          return (
            <MenuItem
              key={transform.stableRef}
              type="button"
              disabled={isRunning}
              active={isActive}
              onClick={() => {
                onPreview(transform);
                onClose();
              }}
              className="gap-2.5 px-2.5 py-2"
            >
              {isRunning && isActive
                ? <LoaderCircle className="theme-workflow-text h-4 w-4 shrink-0 animate-spin" />
                : <Workflow className="theme-workflow-text h-4 w-4 shrink-0" />}
              <OverflowText text={transform.name} className="min-w-0 flex-1 truncate" />
              {usesIntelligence && (
                <Sparkles className="theme-intelligence-text h-3.5 w-3.5 shrink-0" aria-label="Uses connected intelligence" />
              )}
            </MenuItem>
          );
        }) : (
          <p className="theme-text-muted px-2.5 py-3 text-[11px] font-normal">
            No saved Transforms yet.
          </p>
        )}
      </div>

      <MenuDivider />
      <MenuItem
        type="button"
        role="menuitem"
        onClick={() => {
          onManageTransforms();
          onClose();
        }}
        className="gap-2.5 px-2.5 py-2"
      >
        <Workflow className="theme-workflow-text h-4 w-4 shrink-0" />
        <span className="flex-1">Manage Transforms…</span>
      </MenuItem>
    </AnchoredMenu>
  );
}
