import React, { useState, useEffect } from 'react';
import { Keyboard, X } from 'lucide-react';

interface HotkeyRecorderProps {
  value?: string | null;
  onChange: (shortcut: string | null) => void;
  placeholder?: string;
}

const OPTION_KEY_MAP: Record<string, string> = {
  'ç': 'C', 'Ç': 'C',
  '√': 'V', '◊': 'V',
  'µ': 'M', 'Â': 'M',
  '≈': 'X',
  'ß': 'S', 'Í': 'S',
  '∂': 'D', 'Î': 'D',
  'ƒ': 'F', 'Ï': 'F',
  '©': 'G', '˝': 'G',
  '®': 'R', '‰': 'R',
  '†': 'T', 'ˇ': 'T',
  '¥': 'Y', 'Á': 'Y',
  'ø': 'O', 'Ø': 'O',
  'π': 'P', '∏': 'P',
  'å': 'A', 'Å': 'A',
  '∫': 'B',
  '∆': 'J', 'Ô': 'J',
  '˚': 'K', '': 'K',
  '¬': 'L', 'Ò': 'L',
  'Ω': 'Z', '¸': 'Z',
  'œ': 'Q', 'Œ': 'Q',
  '∑': 'W', '„': 'W',
  '´': 'E',
  '¡': '1', '™': '2', '£': '3', '¢': '4', '∞': '5',
  '§': '6', '¶': '7', '•': '8', 'ª': '9', 'º': '0',
};

const getLayoutKey = (e: KeyboardEvent): string => {
  if (e.key && OPTION_KEY_MAP[e.key]) {
    return OPTION_KEY_MAP[e.key];
  }
  if (e.key && e.key.length === 1) {
    return e.key.toUpperCase();
  }
  if (e.code === 'Space') return 'Space';
  if (e.code === 'Tab') return 'Tab';
  if (e.code === 'Enter') return 'Enter';
  return e.key;
};

export const HotkeyRecorder: React.FC<HotkeyRecorderProps> = ({
  value,
  onChange,
  placeholder,
}) => {
  const [isRecording, setIsRecording] = useState(false);

  useEffect(() => {
    if (!isRecording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        setIsRecording(false);
        return;
      }

      if (e.key === 'Backspace' || e.key === 'Delete') {
        onChange(null);
        setIsRecording(false);
        return;
      }

      const parts: string[] = [];
      if (e.metaKey) parts.push('Command');
      if (e.ctrlKey) parts.push('Control');
      if (e.altKey) parts.push('Alt');
      if (e.shiftKey) parts.push('Shift');

      // Ignore standalone modifier keys
      if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) {
        return;
      }

      const keyStr = getLayoutKey(e);
      parts.push(keyStr);

      const hotkeyStr = parts.join('+');
      onChange(hotkeyStr);
      setIsRecording(false);
    };

    window.addEventListener('keydown', handleKeyDown, true);
    return () => window.removeEventListener('keydown', handleKeyDown, true);
  }, [isRecording, onChange]);

  const formatHotkeyDisplay = (str: string | null | undefined) => {
    if (!str) return null;
    return str
      .replace(/CmdOrCtrl/g, '⌘')
      .replace(/Command/g, '⌘')
      .replace(/Super/g, '⌘')
      .replace(/Cmd/g, '⌘')
      .replace(/Control/g, '⌃')
      .replace(/Ctrl/g, '⌃')
      .replace(/Alt/g, '⌥')
      .replace(/Option/g, '⌥')
      .replace(/Shift/g, '⇧')
      .replace(/Key([A-Z])/g, '$1')
      .replace(/Digit/g, '')
      .replace(/\+/g, '');
  };

  return (
    <div className="flex items-center font-sans select-none">
      <div
        className={`hotkey-recorder h-7 rounded-lg text-xs font-mono font-semibold border box-border transition-[background-color,border-color,color,box-shadow,width] flex items-center shrink-0 ${
          isRecording
            ? 'is-recording px-2.5 animate-pulse'
            : value
            ? 'has-value pl-2.5 pr-1.5 shadow-sm'
            : 'is-empty px-2'
        }`}
      >
        <button
          type="button"
          onClick={() => setIsRecording(true)}
          className="flex items-center space-x-1"
          title={value ? `Shortcut: ${formatHotkeyDisplay(value)}` : 'Set Shortcut'}
        >
          <Keyboard className="hotkey-recorder-icon w-3.5 h-3.5 opacity-80 shrink-0" />
          {(isRecording || value || placeholder) && (
            <span>
              {isRecording
                ? 'Press hotkey...'
                : value
                ? formatHotkeyDisplay(value)
                : placeholder}
            </span>
          )}
        </button>

        {value && !isRecording && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onChange(null);
            }}
            className="hotkey-recorder-clear ml-1.5 p-0.5 rounded transition-colors"
            title="Clear Shortcut"
          >
            <X className="w-3 h-3" />
          </button>
        )}
      </div>
    </div>
  );
};
