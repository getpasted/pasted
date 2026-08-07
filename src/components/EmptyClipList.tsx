import {
  Clipboard,
  FolderOpen,
  ListOrdered,
  Pin,
  Search,
  Shield,
  StickyNote,
  Trash2,
} from 'lucide-react';
import type { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { getClipCollection } from '../utils/clipCollections';

interface EmptyClipListProps {
  currentTab: string;
  searchQuery: string;
  selectedBin?: Bin;
}

export function EmptyClipList({ currentTab, searchQuery, selectedBin }: EmptyClipListProps) {
  const trimmedSearch = searchQuery.trim();
  const collection = getClipCollection(currentTab, selectedBin);
  let icon = <Clipboard className="sidebar-icon-primary w-10 h-10 stroke-1" />;
  let title = collection?.emptyTitle ?? 'No clips yet';
  let description = collection?.emptyDescription ?? 'Your copied items will appear here automatically.';

  if (currentTab === 'search') {
    icon = <Search className="sidebar-icon-primary w-10 h-10 stroke-1" />;
    title = trimmedSearch ? 'No matching clips' : 'Search your clips';
    description = trimmedSearch
      ? 'Try another search or filter.'
      : description;
  } else if (currentTab === 'sequential') {
    icon = <ListOrdered className="sidebar-icon-secondary w-10 h-10 stroke-1" />;
  } else if (currentTab === 'pinned') {
    icon = <Pin className="sidebar-icon-success pin-icon w-10 h-10 stroke-1" />;
  } else if (currentTab === 'protected') {
    icon = <Shield className="sidebar-icon-info w-10 h-10 stroke-1" />;
  } else if (currentTab === 'notes') {
    icon = <StickyNote className="sidebar-icon-note w-10 h-10 stroke-1" />;
  } else if (currentTab === 'trash') {
    icon = <Trash2 className="sidebar-icon-danger w-10 h-10 stroke-1" />;
  } else if (currentTab === 'bin') {
    icon = selectedBin
      ? <span className="text-3xl leading-none">{formatEmojiIcon(selectedBin.icon)}</span>
      : <FolderOpen className="sidebar-icon-primary w-10 h-10 stroke-1" />;
  }

  return (
    <div className="theme-text-subtle h-full flex flex-col items-center justify-center text-center p-6 select-none">
      <div className="mb-3 opacity-55" aria-hidden="true">{icon}</div>
      <p className="theme-text-muted text-xs font-medium">{title}</p>
      <p className="text-[11px] mt-1 max-w-56 leading-relaxed">{description}</p>
    </div>
  );
}
