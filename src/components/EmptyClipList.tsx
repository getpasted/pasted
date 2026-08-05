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

interface EmptyClipListProps {
  currentTab: string;
  searchQuery: string;
  selectedBin?: Bin;
}

export function EmptyClipList({ currentTab, searchQuery, selectedBin }: EmptyClipListProps) {
  const trimmedSearch = searchQuery.trim();
  let icon = <Clipboard className="sidebar-icon-primary w-10 h-10 stroke-1" />;
  let title = 'No clips yet';
  let description = 'Your copied items will appear here automatically.';

  if (currentTab === 'search') {
    icon = <Search className="sidebar-icon-primary w-10 h-10 stroke-1" />;
    title = trimmedSearch ? 'No matching clips' : 'Search your clips';
    description = trimmedSearch
      ? 'Try another word, phrase, or search filter.'
      : 'Start typing to search active and trashed clips.';
  } else if (currentTab === 'sequential') {
    icon = <ListOrdered className="sidebar-icon-secondary w-10 h-10 stroke-1" />;
    title = 'Queue is empty';
    description = 'Add clips to Queue to paste them back in sequence.';
  } else if (currentTab === 'pinned') {
    icon = <Pin className="sidebar-icon-warning pin-icon w-10 h-10 stroke-1" />;
    title = 'No pinned clips';
    description = 'Pin a clip to keep it at the top and find it here.';
  } else if (currentTab === 'protected') {
    icon = <Shield className="sidebar-icon-info w-10 h-10 stroke-1" />;
    title = 'No protected clips';
    description = 'Protect a clip to keep it safe from automatic cleanup.';
  } else if (currentTab === 'notes') {
    icon = <StickyNote className="sidebar-icon-success w-10 h-10 stroke-1" />;
    title = 'No noted clips';
    description = 'Add a note to a clip and it will appear here.';
  } else if (currentTab === 'trash') {
    icon = <Trash2 className="sidebar-icon-danger w-10 h-10 stroke-1" />;
    title = 'Trash is empty';
    description = 'Clips moved to Trash will stay here until it is emptied.';
  } else if (currentTab === 'bin') {
    icon = selectedBin
      ? <span className="text-3xl leading-none">{formatEmojiIcon(selectedBin.icon)}</span>
      : <FolderOpen className="sidebar-icon-primary w-10 h-10 stroke-1" />;
    if (selectedBin?.smart_rule) {
      title = 'No matching clips';
      description = `Clips matching ${selectedBin.name}’s rules will appear here automatically.`;
    } else {
      title = selectedBin ? `No clips in ${selectedBin.name}` : 'This Bin is empty';
      description = 'Drag clips here or choose this Bin from a clip.';
    }
  }

  return (
    <div className="theme-text-subtle h-full flex flex-col items-center justify-center text-center p-6 select-none">
      <div className="mb-3 opacity-55" aria-hidden="true">{icon}</div>
      <p className="theme-text-muted text-xs font-medium">{title}</p>
      <p className="text-[11px] mt-1 max-w-56 leading-relaxed">{description}</p>
    </div>
  );
}
