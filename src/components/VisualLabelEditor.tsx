import { Plus, RotateCcw, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

import { translate } from '../localization/runtime';
import type { EffectiveVisualLabels } from './clipPreviewModel';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';

export function VisualLabelEditor({
  visualLabels,
  readOnly,
  onAdd,
  onRemove,
  onReset,
}: {
  visualLabels: EffectiveVisualLabels;
  readOnly: boolean;
  onAdd: (label: string) => void | Promise<void>;
  onRemove: (label: string) => void | Promise<void>;
  onReset: () => void | Promise<void>;
}) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState('');
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (adding) inputRef.current?.focus();
  }, [adding]);

  const submit = async () => {
    const label = draft.trim();
    if (!label) return;
    await onAdd(label);
    setDraft('');
    setAdding(false);
  };

  const requestReset = () => setConfirmation({
    title: translate('component.clipPreviewContent.resetVisualLabelsTitle'),
    description: translate('component.clipPreviewContent.resetVisualLabelsDescription'),
    confirmLabel: translate('common.reset'),
    onConfirm: async () => {
      setConfirmation(null);
      await onReset();
    },
  });

  return <>
    <div
      className="theme-code-surface min-h-12 rounded-lg border p-2.5"
      onClick={(event) => {
        if (!readOnly && event.target === event.currentTarget) setAdding(true);
      }}
    >
      <ul className="flex min-h-6 flex-wrap items-center gap-2" dir="auto">
        {visualLabels.labels.map((label) => (
          <li key={label.value.toLocaleLowerCase()} className="theme-surface group relative flex h-6 items-center rounded-lg border text-xs">
            <span className="px-2.5">{label.value}</span>
            {!readOnly && (
              <button
                type="button"
                onClick={() => void onRemove(label.value)}
                className="theme-icon-button theme-focusable pointer-events-none absolute end-0.5 top-1/2 -translate-y-1/2 rounded p-0.5 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100"
                aria-label={translate('component.clipPreviewContent.removeVisualLabelName', { name: label.value })}
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </li>
        ))}
        {!readOnly && (adding ? (
          <li>
            <input
              ref={inputRef}
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') {
                  event.preventDefault();
                  void submit();
                } else if (event.key === 'Escape') {
                  setDraft('');
                  setAdding(false);
                }
              }}
              onBlur={() => { if (!draft.trim()) setAdding(false); }}
              maxLength={120}
              className="theme-input ui-field-radius h-6 w-36 border px-2 py-0.5 text-xs"
              placeholder={translate('component.clipPreviewContent.visualLabelPlaceholder')}
              aria-label={translate('component.clipPreviewContent.addVisualLabel')}
            />
          </li>
        ) : (
          <li>
            <button
              type="button"
              onClick={() => setAdding(true)}
              className="theme-icon-button theme-focusable flex h-6 w-6 items-center justify-center rounded-lg border"
              aria-label={translate('component.clipPreviewContent.addVisualLabel')}
              title={translate('component.clipPreviewContent.addVisualLabel')}
            >
              <Plus className="h-3.5 w-3.5" />
            </button>
          </li>
        ))}
      </ul>
      {!readOnly && visualLabels.hasOverrides && (
        <div className="mt-2 flex justify-end">
          <button type="button" onClick={requestReset} className="theme-secondary-button theme-focusable flex items-center gap-1.5 rounded-lg border px-2 py-1 text-[10px] font-semibold">
            <RotateCcw className="h-3 w-3" />
            {translate('common.resetWithEllipsis')}
          </button>
        </div>
      )}
    </div>
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  </>;
}
