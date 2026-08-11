import {
  Code, CreditCard, FileCode2, Files, FileText, Fingerprint, Hash,
  Image as ImageIcon, KeyRound, Landmark, Link, Mail, MapPin, Network,
  Palette, Phone, Router, ScrollText, ShieldKeyhole, TerminalSquare, Type, Variable,
} from 'lucide-react';
import type { ClipContentType } from '../types';

export function ContentTypeIcon({ type, className = 'w-4 h-4' }: { type: ClipContentType; className?: string }) {
  const props = { className, 'aria-hidden': true } as const;
  switch (type) {
    case 'image': return <ImageIcon {...props} />;
    case 'file': return <Files {...props} />;
    case 'file_path': return <MapPin {...props} />;
    case 'color': return <Palette {...props} />;
    case 'link': return <Link {...props} />;
    case 'email': return <Mail {...props} />;
    case 'phone': return <Phone {...props} />;
    case 'code': return <Code {...props} />;
    case 'shell_command': return <TerminalSquare {...props} />;
    case 'env_variable': return <Variable {...props} />;
    case 'env_block': return <FileCode2 {...props} />;
    case 'credential': return <KeyRound {...props} />;
    case 'payment_card': return <CreditCard {...props} />;
    case 'iban': return <Landmark {...props} />;
    case 'jwt': return <ShieldKeyhole {...props} />;
    case 'hash': return <Hash {...props} />;
    case 'ip_address': return <Network {...props} />;
    case 'mac_address': return <Router {...props} />;
    case 'uuid': return <Fingerprint {...props} />;
    case 'prose': return <ScrollText {...props} />;
    case 'text': return <Type {...props} />;
    default: return <FileText {...props} />;
  }
}
