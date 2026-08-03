export interface ClipNote {
  id: string;
  text: string;
  created_at: string;
}

export function parseClipNotes(noteField?: string | null): ClipNote[] {
  if (!noteField || !noteField.trim()) return [];
  try {
    const parsed = JSON.parse(noteField);
    if (Array.isArray(parsed)) {
      return parsed.map((n, idx) => {
        if (typeof n === 'string') {
          return { id: `note-${idx}`, text: n, created_at: new Date().toISOString() };
        }
        return {
          id: n.id || `note-${idx}`,
          text: n.text || '',
          created_at: n.created_at || new Date().toISOString(),
        };
      });
    }
  } catch {
    // Legacy single string note
    return [{ id: 'note-legacy', text: noteField, created_at: new Date().toISOString() }];
  }
  return [{ id: 'note-legacy', text: noteField, created_at: new Date().toISOString() }];
}

export function serializeClipNotes(notes: ClipNote[]): string | null {
  const validNotes = notes.filter((n) => n.text.trim().length > 0);
  if (validNotes.length === 0) return null;
  return JSON.stringify(validNotes);
}

export function getClipNoteSummary(noteField?: string | null): string {
  const notes = parseClipNotes(noteField);
  if (notes.length === 0) return '';
  return notes.map((n) => n.text.trim()).filter((t) => t.length > 0).join(' • ');
}

export function isSensitiveText(text: string | null): boolean {
  if (!text) return false;
  const trimmed = text.trim();
  if (
    /(?:sk_live_|sk_test_|ghp_|gho_|xoxb-|xoxp-|AKIA[0-9A-Z]{16}|sk-proj-|sk-ant-)\w+/i.test(trimmed) ||
    /bearer\s+[a-zA-Z0-9_\-\.=]+/i.test(trimmed) ||
    /-----BEGIN (?:RSA )?PRIVATE KEY-----/.test(trimmed)
  ) {
    return true;
  }
  const ccDigits = trimmed.replace(/[\s-]/g, '');
  if (/^\d{13,19}$/.test(ccDigits) && !isNaN(Number(ccDigits))) {
    return true;
  }
  if (/^(?:password|passwd|secret_key|api_secret)\s*[:=]/i.test(trimmed)) {
    return true;
  }
  return false;
}

export function maskSensitiveText(text: string | null): string {
  if (!text) return '';
  const trimmed = text.trim();
  if (trimmed.length <= 8) {
    return '•••• ••••';
  }
  const lastFour = trimmed.slice(-4);
  return `•••• •••• •••• ${lastFour}`;
}

export interface ClipItem {
  id: number;
  content_type: 'text' | 'image' | 'color' | 'link' | 'code';
  text_content: string | null;
  html_content: string | null;
  image_base64: string | null;
  image_path?: string | null;
  content_hash: string;
  source_app: string;
  is_pinned: boolean;
  is_protected?: boolean;
  pin_order?: number;
  bin_id: number | null;
  bin_ids?: number[];
  note?: string | null;
  created_at: string;
}

export interface Bin {
  id: number;
  name: string;
  icon: string;
  color: string;
  smart_rule?: string | null;
  bin_type?: 'category' | 'tag';
  shortcut?: string | null;
  clip_count?: number | null;
  created_at: string;
}

export interface ClipVersion {
  id: number;
  clip_id: number;
  text_content: string;
  created_at: string;
}

export interface FilterRule {
  id: number;
  name: string;
  filter_type: string;
  config: string | null;
  shortcut?: string | null;
  created_at: string;
}

export interface Operation {
  id: number;
  name: string;
  op_type: string;
  config: string | null;
  category: string;
  created_at: string;
}

export interface SequentialStatus {
  is_active: boolean;
  queue: string[];
  current_index: number;
  total_count: number;
}

export interface AppSettings {
  textSize: number;
  enableSounds: boolean;
  openAtLogin: boolean;
  dockMenubarIcon: 'auto_hide' | 'both' | 'menubar_only';
  maxClipSizeMb: number;
  keepClipCount: number;
  alwaysPastePlainText: boolean;
  rowHeight: 'small' | 'medium' | 'large';
  iCloudSync: boolean;
  themeMode: 'system' | 'dark' | 'light';
  spotlightSync: boolean;
  enableActivityLog: boolean;
  activityLogCapacity: number;
  enableTrash: boolean;
  trashCapacityCount: number;
  hudHotkey?: string;
  seqToggleHotkey?: string;
  seqPopHotkey?: string;
  pasteLastFilterHotkey?: string;
  openFilterWindowHotkey?: string;
  openMainWindowHotkey?: string;
  pasteClip1Hotkey?: string;
  pasteClip2Hotkey?: string;
  pasteClip3Hotkey?: string;
  pasteClip4Hotkey?: string;
  pasteClip5Hotkey?: string;
  pasteClip6Hotkey?: string;
  pasteClip7Hotkey?: string;
  pasteClip8Hotkey?: string;
  pasteClip9Hotkey?: string;
}

export interface BlacklistApp {
  id: string;
  name: string;
  icon: string;
  ignoreText: boolean;
  ignoreImages: boolean;
  ignoreShortcuts: boolean;
}
