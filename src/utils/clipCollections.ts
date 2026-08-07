import type { Bin } from '../types';
import type { FeatureId } from './features';

export type ClipCollectionTab =
  | 'all'
  | 'search'
  | 'sequential'
  | 'pinned'
  | 'protected'
  | 'notes'
  | 'trash'
  | 'bin';

export type ClipDropAction = 'queue' | 'pin' | 'protect' | 'trash';
export type ClipCollectionIcon = 'all' | 'search' | 'queue' | 'pin' | 'protect' | 'note' | 'trash' | 'bin';
export type ClipCollectionMembership = 'all' | 'search' | 'queue' | 'pinned' | 'protected' | 'noted' | 'trash' | 'bin';
export type ClipCollectionOrdering = 'chronological' | 'queue' | 'pinned' | 'collection';

export interface ClipCollectionCapabilities {
  acceptsClipDrop: boolean;
  dropAction?: ClipDropAction;
  canReorder: boolean;
  allowsDuplicateMembership: boolean;
  isCalculated: boolean;
  isReadOnly: boolean;
}

export interface ClipCollectionDefinition {
  key: string;
  tab: ClipCollectionTab;
  label: string;
  title: string;
  tooltip?: string;
  icon: ClipCollectionIcon;
  feature?: FeatureId;
  membership: ClipCollectionMembership;
  ordering: ClipCollectionOrdering;
  capabilities: ClipCollectionCapabilities;
  emptyTitle: string;
  emptyDescription: string;
}

const calculated = (overrides: Partial<ClipCollectionCapabilities> = {}): ClipCollectionCapabilities => ({
  acceptsClipDrop: false,
  canReorder: false,
  allowsDuplicateMembership: false,
  isCalculated: true,
  isReadOnly: false,
  ...overrides,
});

export const SYSTEM_CLIP_COLLECTIONS: readonly ClipCollectionDefinition[] = [
  {
    key: 'system:all', tab: 'all', label: 'All', title: 'All', tooltip: 'All Clips', icon: 'all', membership: 'all', ordering: 'chronological',
    capabilities: calculated(), emptyTitle: 'No clips yet', emptyDescription: 'Your copied items will appear here automatically.',
  },
  {
    key: 'system:queue', tab: 'sequential', label: 'Queue', title: 'Queue', icon: 'queue', feature: 'queue', membership: 'queue', ordering: 'queue',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'queue', canReorder: true, allowsDuplicateMembership: true }),
    emptyTitle: 'Queue is empty', emptyDescription: 'Add text clips or record copies to paste them back in sequence.',
  },
  {
    key: 'system:pinned', tab: 'pinned', label: 'Pinned', title: 'Pinned', icon: 'pin', feature: 'pinning', membership: 'pinned', ordering: 'pinned',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'pin', canReorder: true }),
    emptyTitle: 'No pinned clips', emptyDescription: 'Pin a clip to keep it at the top and find it here.',
  },
  {
    key: 'system:protected', tab: 'protected', label: 'Protected', title: 'Protected', icon: 'protect', feature: 'protection', membership: 'protected', ordering: 'chronological',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'protect' }),
    emptyTitle: 'No protected clips', emptyDescription: 'Protect a clip to keep it safe from automatic cleanup.',
  },
  {
    key: 'system:noted', tab: 'notes', label: 'Noted', title: 'Noted', icon: 'note', feature: 'notes', membership: 'noted', ordering: 'chronological',
    capabilities: calculated(), emptyTitle: 'No noted clips', emptyDescription: 'Add a note to any clip to annotate it and find it here later.',
  },
  {
    key: 'system:trash', tab: 'trash', label: 'Trashed', title: 'Trashed', icon: 'trash', feature: 'trash', membership: 'trash', ordering: 'chronological',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'trash', isReadOnly: true }),
    emptyTitle: 'Trash is empty', emptyDescription: 'Clips moved to Trash will stay here until it is emptied.',
  },
];

const SEARCH_COLLECTION: ClipCollectionDefinition = {
  key: 'system:search', tab: 'search', label: 'Search', title: 'Search', icon: 'search', membership: 'search', ordering: 'chronological',
  capabilities: calculated(), emptyTitle: 'Search your clips', emptyDescription: 'Search active and trashed clips.',
};

export function getSystemClipCollections(features: Record<FeatureId, boolean>): ClipCollectionDefinition[] {
  return SYSTEM_CLIP_COLLECTIONS.filter(({ feature }) => !feature || features[feature]);
}

export function getClipCollection(tab: string, bin?: Bin | null): ClipCollectionDefinition | undefined {
  if (tab === 'search') return SEARCH_COLLECTION;
  if (tab === 'bin') {
    const isSmart = Boolean(bin?.smart_rule);
    return {
      key: `bin:${bin?.id ?? 'none'}`,
      tab: 'bin',
      label: bin?.name ?? 'Bin',
      title: bin?.name ?? 'Bin',
      icon: 'bin',
      membership: 'bin',
      ordering: 'collection',
      capabilities: {
        acceptsClipDrop: !isSmart,
        canReorder: true,
        allowsDuplicateMembership: false,
        isCalculated: isSmart,
        isReadOnly: false,
      },
      emptyTitle: isSmart ? 'No matching clips' : bin ? `No clips in ${bin.name}` : 'This Bin is empty',
      emptyDescription: isSmart
        ? `Clips matching ${bin?.name ?? 'this Bin'}’s rules will appear here automatically.`
        : 'Drag clips here or choose this Bin from a clip.',
    };
  }
  return SYSTEM_CLIP_COLLECTIONS.find((collection) => collection.tab === tab);
}
