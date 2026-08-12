import { useEffect, useState } from 'react';
import { Check, ClipboardCopy, ClipboardPaste } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { UI_COPY } from '../utils/uiCopy';
import { ActionButton } from './AppDialogLayout';

interface TransformationOutputActionsProps {
  output: string;
}

type OutputAction = 'copied' | 'pasted' | null;

export function TransformationOutputActions({ output }: TransformationOutputActionsProps) {
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
    <div className="flex flex-wrap items-center justify-end gap-2">
      <ActionButton
        onClick={copyResult}
        disabled={!output}
        className="h-9 min-h-9 px-3"
      >
        {lastAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <ClipboardCopy className="h-3.5 w-3.5" />}
        <span>{lastAction === 'copied' ? UI_COPY.copied : 'Copy Result'}</span>
      </ActionButton>
      <ActionButton
        onClick={pasteResult}
        disabled={!output}
        className="h-9 min-h-9 px-3"
      >
        {lastAction === 'pasted' ? <Check className="h-3.5 w-3.5" /> : <ClipboardPaste className="h-3.5 w-3.5" />}
        <span>{lastAction === 'pasted' ? 'Pasted' : 'Paste Result'}</span>
      </ActionButton>
    </div>
  );
}
