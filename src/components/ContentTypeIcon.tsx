import {
  AlignLeft, AtSign, Binary, BookOpen, Box, Braces, Calendar, CheckSquare,
  CircleDollarSign, Clipboard, Clock, Database,
  Code, CreditCard, FileCode2, Files, FileText, Fingerprint, Hash,
  FileJson, FileSpreadsheet, Folder, Globe, Heart, Image as ImageIcon, KeyRound,
  Landmark, Link, List, Lock, Mail, MapPin, MessageSquare, Network, Package,
  Palette, Phone, Receipt, Router, ScrollText, Search, Settings, ShieldKeyhole,
  Star, Tag, TerminalSquare, Type, User, Variable, Wallet, Wrench, Zap,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import type { ClipContentType } from '../types';
import { useContentTypes } from './ContentTypeProvider';

const CONTENT_TYPE_ICONS: Record<string, LucideIcon> = {
  AlignLeft, AtSign, Binary, BookOpen, Box, Braces, Calendar, CheckSquare,
  CircleDollarSign, Clipboard, Clock, Database,
  Code, CreditCard, FileCode2, FileText, Files, Fingerprint, Hash, Image: ImageIcon,
  FileJson, FileSpreadsheet, Folder, Globe, Heart, KeyRound, Landmark, Link, List,
  Lock, Mail, MapPin, MessageSquare, Network, Package, Palette, Phone, Receipt,
  Router, ScrollText, Search, Settings, ShieldKeyhole, Star, Tag, TerminalSquare,
  Type, User, Variable, Wallet, Wrench, Zap,
};

export function ContentTypeIcon({ type, className = 'w-4 h-4' }: { type: ClipContentType; className?: string }) {
  const { definitions } = useContentTypes();
  const structuralIcon = type === 'text' ? 'Type' : type === 'image' ? 'Image' : type === 'file' ? 'Files' : 'FileText';
  const iconName = definitions.find(({ id }) => id === type)?.icon ?? structuralIcon;
  return <ContentTypeGlyph icon={iconName} className={className} />;
}

export function ContentTypeGlyph({ icon, className = 'w-4 h-4' }: { icon: string; className?: string }) {
  const props = { className, 'aria-hidden': true } as const;
  const Icon = CONTENT_TYPE_ICONS[icon] ?? FileText;
  return <Icon {...props} />;
}
