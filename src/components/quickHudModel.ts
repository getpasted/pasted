export interface HudShortcutEvent {
  key: string;
  metaKey: boolean;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

export function isQuickHudRoute(search: string): boolean {
  return new URLSearchParams(search).get('view') === 'hud';
}

export function hudPasteShortcutIndex(event: HudShortcutEvent): number | null {
  const hasPrimaryModifier = event.metaKey !== event.ctrlKey;
  if (!hasPrimaryModifier || event.altKey || event.shiftKey || !/^[1-9]$/.test(event.key)) {
    return null;
  }
  return Number(event.key) - 1;
}

export function hudPrimaryModifierLabel(platform: string | undefined): string {
  return platform === 'macos' ? '⌘' : 'Ctrl';
}
