import { useRef } from 'react';

import { AnchoredMenu } from './AnchoredMenu';
import { lastEmojiGrapheme } from './binModalEmoji';
import {
  BIN_EMOJI_OPTIONS,
  COLOR_PALETTE,
  emojiLabel,
} from './binModalModel';
import type { BinModalFormController } from '../hooks/useBinModalForm';
import { translate } from '../localization/runtime';
import { formatEmojiIcon } from '../utils/emoji';
import type { DesktopPlatform } from '../utils/platform';
import { safeInvoke as invoke } from '../utils/tauri';

interface BinModalIdentityFieldsProps {
  form: BinModalFormController;
  desktopPlatform: DesktopPlatform;
}

export function BinModalIdentityFields({ form, desktopPlatform }: BinModalIdentityFieldsProps) {
  const nativeEmojiTriggerRef = useRef<HTMLInputElement>(null);
  const {
    name,
    setName,
    selectedColor,
    setSelectedColor,
    icon,
    setIcon,
    isEmojiMenuOpen,
    setIsEmojiMenuOpen,
    emojiTriggerRef,
    errors,
    setErrors,
    submit,
  } = form;

  return <>
    <div className="flex items-center space-x-3">
      <label className={`w-20 text-end font-semibold flex-shrink-0 ${errors.name ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>
        {translate('common.name')}
      </label>
      <input
        type="text"
        placeholder={translate('component.binModal.eGCodeSnippetsSafariClips')}
        value={name}
        onChange={(event) => {
          setName(event.target.value);
          if (errors.name) setErrors((previous) => ({ ...previous, name: false }));
        }}
        onKeyDown={(event) => {
          if (event.key === 'Enter') submit(event);
        }}
        className={`flex-1 theme-input ui-field-radius border px-3 py-2 text-xs focus:outline-none font-medium transition-colors ${errors.name ? 'form-field-error' : 'form-field-valid'}`}
        autoFocus
      />
    </div>

    <div className="flex items-center space-x-3">
      <label className={`w-20 text-end font-semibold flex-shrink-0 ${errors.color ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>
        {translate('component.binModal.color')}
      </label>
      <div className={`flex items-center space-x-2 p-1 rounded-xl border border-transparent transition-colors ${errors.color ? 'form-field-error' : ''}`}>
        {COLOR_PALETTE.map((color) => (
          <button
            key={color.hex}
            type="button"
            onClick={() => {
              setSelectedColor(color.hex);
              if (errors.color) setErrors((previous) => ({ ...previous, color: false }));
            }}
            style={{ backgroundColor: color.hex === 'default' ? 'var(--text-main)' : color.hex }}
            className={`w-5 h-5 rounded-full border border-transparent transition-transform ${selectedColor === color.hex ? 'bin-color-selected scale-110' : 'opacity-80 hover:opacity-100'}`}
            aria-label={translate('component.binModal.labelBinText', { label: color.label })}
            title={color.label}
          />
        ))}
      </div>
    </div>

    <div className="flex items-center space-x-3">
      <label className={`w-20 text-end font-semibold flex-shrink-0 ${errors.icon ? 'theme-danger-text font-bold' : 'theme-text-muted'}`}>
        {translate('component.binModal.icon')}
      </label>
      <div className="flex-1 flex items-center space-x-2.5">
        {desktopPlatform === 'macos' ? (
          <input
            ref={nativeEmojiTriggerRef}
            type="text"
            value={formatEmojiIcon(icon)}
            onChange={(event) => {
              const nextIcon = lastEmojiGrapheme(event.target.value);
              if (!nextIcon) return;
              setIcon(nextIcon);
              setErrors((previous) => ({ ...previous, icon: false }));
            }}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === 'Tab' || event.metaKey || event.ctrlKey) return;
              event.preventDefault();
            }}
            onClick={async (event) => {
              event.currentTarget.select();
              const openedNativePicker = await invoke<boolean>('open_emoji_picker').catch(() => false);
              if (!openedNativePicker) setIsEmojiMenuOpen((open) => !open);
            }}
            onFocus={(event) => event.currentTarget.select()}
            placeholder="📂"
            maxLength={64}
            className={`theme-input theme-input-action ui-field-radius emoji-input-picker elevation-inset w-16 cursor-pointer select-none border py-1.5 text-center font-mono text-lg transition-colors focus:outline-none ${errors.icon ? 'form-field-error' : 'form-field-valid'}`}
            aria-label={translate('component.binModal.chooseBinIcon')}
            title={translate('component.binModal.openEmojiPicker')}
          />
        ) : (
          <button
            ref={emojiTriggerRef}
            type="button"
            onClick={() => setIsEmojiMenuOpen((open) => !open)}
            className={`theme-input theme-input-action ui-field-radius emoji-input-picker elevation-inset w-16 cursor-pointer select-none border py-1.5 text-center font-mono text-lg transition-colors focus:outline-none ${errors.icon ? 'form-field-error' : 'form-field-valid'}`}
            aria-label={translate('component.binModal.chooseBinIcon')}
            aria-haspopup="menu"
            aria-expanded={isEmojiMenuOpen}
            title={translate('component.binModal.chooseBinIcon')}
          >
            {formatEmojiIcon(icon)}
          </button>
        )}
        <span className="text-[11px] theme-text-muted">
          {desktopPlatform === 'macos'
            ? translate('component.binModal.openEmojiPickerShortcut', { shortcut: translate('component.binModal.commandSpace') })
            : translate('component.binModal.chooseAnIconForThisBin')}
        </span>
        {isEmojiMenuOpen && (
          <AnchoredMenu
            anchor={{
              kind: 'element',
              ref: desktopPlatform === 'macos' ? nativeEmojiTriggerRef : emojiTriggerRef,
              align: 'start',
            }}
            ariaLabel={translate('component.binModal.chooseBinIcon')}
            onClose={() => setIsEmojiMenuOpen(false)}
            className="w-72"
          >
            <div className="grid grid-cols-8 gap-1" role="group" aria-label={translate('component.binModal.binIcons')}>
              {BIN_EMOJI_OPTIONS.map(([emoji, labelKey]) => {
                const label = emojiLabel(labelKey);
                return <button
                  key={emoji}
                  type="button"
                  role="menuitemradio"
                  aria-checked={formatEmojiIcon(icon) === emoji}
                  aria-label={label}
                  title={label}
                  className={`theme-menu-item grid h-7 w-7 place-items-center rounded-lg text-base ${formatEmojiIcon(icon) === emoji ? 'is-selected' : ''}`}
                  onClick={() => {
                    setIcon(emoji);
                    setErrors((previous) => ({ ...previous, icon: false }));
                    setIsEmojiMenuOpen(false);
                    emojiTriggerRef.current?.focus();
                  }}
                >
                  {emoji}
                </button>;
              })}
            </div>
          </AnchoredMenu>
        )}
      </div>
    </div>
  </>;
}
