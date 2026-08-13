import type { ClipContentType } from '../types';

export interface ContentTypeDescriptor {
  value: ClipContentType;
  label: string;
  icon: string;
  group: 'General' | 'Developer' | 'Personal and financial' | 'Identifiers';
}

export const CONTENT_TYPES: readonly ContentTypeDescriptor[] = [
  { value: 'text', label: 'Plain Text', icon: 'Type', group: 'General' },
  { value: 'prose', label: 'Prose', icon: 'ScrollText', group: 'General' },
  { value: 'link', label: 'Web Link', icon: 'Link', group: 'General' },
  { value: 'email', label: 'Email Address', icon: 'Mail', group: 'General' },
  { value: 'phone', label: 'Phone Number', icon: 'Phone', group: 'General' },
  { value: 'image', label: 'Image', icon: 'Image', group: 'General' },
  { value: 'file', label: 'File', icon: 'Files', group: 'General' },
  { value: 'file_path', label: 'File Path', icon: 'MapPin', group: 'General' },
  { value: 'color', label: 'Color', icon: 'Palette', group: 'General' },
  { value: 'code', label: 'Code', icon: 'Code', group: 'Developer' },
  { value: 'shell_command', label: 'Shell Command', icon: 'TerminalSquare', group: 'Developer' },
  { value: 'env_variable', label: 'Environment Variable', icon: 'Variable', group: 'Developer' },
  { value: 'env_block', label: 'Environment Block', icon: 'FileCode2', group: 'Developer' },
  { value: 'credential', label: 'Credential', icon: 'KeyRound', group: 'Personal and financial' },
  { value: 'payment_card', label: 'Payment Card', icon: 'CreditCard', group: 'Personal and financial' },
  { value: 'iban', label: 'IBAN', icon: 'Landmark', group: 'Personal and financial' },
  { value: 'jwt', label: 'JSON Web Token', icon: 'ShieldKeyhole', group: 'Identifiers' },
  { value: 'hash', label: 'Hash', icon: 'Hash', group: 'Identifiers' },
  { value: 'ip_address', label: 'IP Address', icon: 'Network', group: 'Identifiers' },
  { value: 'mac_address', label: 'MAC Address', icon: 'Router', group: 'Identifiers' },
  { value: 'uuid', label: 'UUID', icon: 'Fingerprint', group: 'Identifiers' },
] as const;

let contentTypeLabels = new Map<string, string>(CONTENT_TYPES.map(({ value, label }) => [value, label]));

export function setRuntimeContentTypes(definitions: ReadonlyArray<{ id: string; label: string }>): void {
  contentTypeLabels = new Map(definitions.map(({ id, label }) => [id, label]));
}

export function contentTypeLabel(type: string): string {
  return contentTypeLabels.get(type)
    ?? type.split('_').map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ');
}

export function isSensitiveContentType(type: ClipContentType): boolean {
  return type === 'credential' || type === 'payment_card' || type === 'jwt';
}
