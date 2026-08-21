import {
  Clipboard,
  FolderOpen,
  ListOrdered,
  Pin,
  Search,
  Shield,
  EyeOff,
  FilePenLine,
  StickyNote,
  Trash2,
} from 'lucide-react';
import type { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import { getClipCollection } from '../utils/clipCollections';
import { translate } from '../localization/runtime';
import { useLocalization } from '../localization/LocalizationProvider';

interface EmptyClipListProps {
  currentTab: string;
  searchQuery: string;
  selectedBin?: Bin;
}

export function EmptyClipList({ currentTab, searchQuery, selectedBin }: EmptyClipListProps) {
  useLocalization();
  const trimmedSearch = searchQuery.trim();
  const collection = getClipCollection(currentTab, selectedBin);
  let icon = <Clipboard className="sidebar-icon-primary w-10 h-10 stroke-1" />;
  let title = collection?.emptyTitle ?? translate('component.emptyClipList.noClipsYet');
  let description = collection?.emptyDescription ?? translate('component.emptyClipList.yourCopiedItemsWillAppearHereAutomatically');

  if (currentTab === 'search') {
    icon = <Search className="sidebar-icon-primary w-10 h-10 stroke-1" />;
    title = trimmedSearch ? translate('collection.noMatchingClips') : translate('collection.searchYourClips');
    description = trimmedSearch
      ? translate('collection.tryAnotherSearchOrFilter')
      : description;
  } else if (currentTab === 'sequential') {
    icon = <ListOrdered className="sidebar-icon-secondary w-10 h-10 stroke-1" />;
  } else if (currentTab === 'pinned') {
    icon = <Pin className="sidebar-icon-success pin-icon w-10 h-10 stroke-1" />;
  } else if (currentTab === 'protected') {
    icon = <Shield className="sidebar-icon-info w-10 h-10 stroke-1" />;
  } else if (currentTab === 'concealed') {
    icon = <EyeOff className="sidebar-icon-warning w-10 h-10 stroke-1" />;
  } else if (currentTab === 'named') {
    icon = <FilePenLine className="sidebar-icon-named w-10 h-10 stroke-1" />;
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
