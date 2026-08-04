import { useEffect, useState } from 'react';
import { Check, ClipboardCopy, ClipboardPaste } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { UI_COPY } from '../utils/uiCopy';

interface TransformationOutputActionsProps {
  output: string;
  accent: 'pipelines' | 'operations';
}

type OutputAction = 'copied' | 'pasted' | null;

export function TransformationOutputActions({ output, accent }: TransformationOutputActionsProps) {
  const [lastAction, setLastAction] = useState<OutputAction>(null);

  useEffect(() => setLastAction(null), [output]);

  const copyResult = async () => {
    if (!output) return;
    await invoke('copy_clip_to_system', { text: output, imageBase64: null });
    setLastAction('copied');
  };

  const pasteResult = async () => {
    if (!output) return;
    await invoke('paste_text_to_frontmost', { text: output });
    setLastAction('pasted');
  };

  return (
    <div className="grid grid-cols-2 gap-2">
      <button
        type="button"
        onClick={copyResult}
        disabled={!output}
        className="theme-secondary-button flex h-9 items-center justify-center gap-2 rounded-xl border px-3 text-xs font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
      >
        {lastAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <ClipboardCopy className="h-3.5 w-3.5" />}
        <span>{lastAction === 'copied' ? UI_COPY.copied : 'Copy Result'}</span>
      </button>
      <button
        type="button"
        onClick={pasteResult}
        disabled={!output}
        className={`transform-workspace-action ${accent} flex h-9 items-center justify-center gap-2 rounded-xl px-3 text-xs font-bold shadow-sm transition-[background-color,color,transform] active:scale-[0.99] disabled:cursor-not-allowed disabled:opacity-40`}
      >
        {lastAction === 'pasted' ? <Check className="h-3.5 w-3.5" /> : <ClipboardPaste className="h-3.5 w-3.5" />}
        <span>{lastAction === 'pasted' ? 'Pasted' : 'Paste Result'}</span>
      </button>
    </div>
  );
}
