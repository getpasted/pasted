import type { ClipContentType } from '../types';

export interface ContentTypeDescriptor {
  value: ClipContentType;
  label: string;
  group: 'General' | 'Developer' | 'Personal & financial' | 'Identifiers';
}

export const CONTENT_TYPES: readonly ContentTypeDescriptor[] = [
  { value: 'text', label: 'Plain Text', group: 'General' },
  { value: 'prose', label: 'Prose', group: 'General' },
  { value: 'link', label: 'Web Link', group: 'General' },
  { value: 'email', label: 'Email Address', group: 'General' },
  { value: 'phone', label: 'Phone Number', group: 'General' },
  { value: 'image', label: 'Image', group: 'General' },
  { value: 'file', label: 'File', group: 'General' },
  { value: 'file_path', label: 'File Path', group: 'General' },
  { value: 'color', label: 'Color', group: 'General' },
  { value: 'code', label: 'Code', group: 'Developer' },
  { value: 'shell_command', label: 'Shell Command', group: 'Developer' },
  { value: 'env_variable', label: 'Environment Variable', group: 'Developer' },
  { value: 'env_block', label: 'Environment Block', group: 'Developer' },
  { value: 'credential', label: 'Credential', group: 'Personal & financial' },
  { value: 'payment_card', label: 'Payment Card', group: 'Personal & financial' },
  { value: 'iban', label: 'IBAN', group: 'Personal & financial' },
  { value: 'jwt', label: 'JSON Web Token', group: 'Identifiers' },
  { value: 'hash', label: 'Hash', group: 'Identifiers' },
  { value: 'ip_address', label: 'IP Address', group: 'Identifiers' },
  { value: 'mac_address', label: 'MAC Address', group: 'Identifiers' },
  { value: 'uuid', label: 'UUID', group: 'Identifiers' },
] as const;

const CONTENT_TYPE_LABELS = new Map(CONTENT_TYPES.map(({ value, label }) => [value, label]));

export function contentTypeLabel(type: string): string {
  return CONTENT_TYPE_LABELS.get(type as ClipContentType)
    ?? type.split('_').map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' ');
}

export function isSensitiveContentType(type: ClipContentType): boolean {
  return type === 'credential' || type === 'payment_card' || type === 'jwt';
}
