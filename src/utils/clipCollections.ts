import type { Bin } from '../types';
import type { FeatureId } from './features';
import { contentTypeLabel } from './contentTypes';
import { translate, type TranslationKey } from '../localization/runtime';
import type { ClipPropertyAssociationId } from './clipPropertyAssociations';

export type ClipCollectionTab =
  | 'all'
  | 'search'
  | 'sequential'
  | 'pinned'
  | 'protected'
  | 'concealed'
  | 'named'
  | 'notes'
  | 'trash'
  | 'bin';

export type ClipDropAction = 'queue' | 'pin' | 'protect' | 'conceal' | 'trash';
export type ClipCollectionIcon = 'all' | 'search' | 'queue' | 'pin' | 'protect' | 'conceal' | 'name' | 'note' | 'trash' | 'bin';
export type ClipCollectionMembership = 'all' | 'search' | 'queue' | 'pinned' | 'protected' | 'concealed' | 'named' | 'noted' | 'trash' | 'bin' | 'facet';
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
  association?: ClipPropertyAssociationId;
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
    key: 'system:all', tab: 'all', label: 'History', title: 'History', tooltip: 'Active clipboard history', icon: 'all', membership: 'all', ordering: 'chronological',
    capabilities: calculated(), emptyTitle: 'No clips yet', emptyDescription: 'Copy something in any app. It will appear here automatically.',
  },
  {
    key: 'system:queue', tab: 'sequential', label: 'Queued', title: 'Queued', icon: 'queue', feature: 'queue', membership: 'queue', ordering: 'queue',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'queue', canReorder: true, allowsDuplicateMembership: true }),
    emptyTitle: 'Queue is empty', emptyDescription: 'Add text clips or record copies to paste them back in sequence.',
  },
  {
    key: 'system:pinned', tab: 'pinned', label: 'Pinned', title: 'Pinned', icon: 'pin', feature: 'pinning', association: 'pin', membership: 'pinned', ordering: 'pinned',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'pin', canReorder: true }),
    emptyTitle: 'No pinned clips', emptyDescription: 'Pin a clip to keep it at the top and find it here.',
  },
  {
    key: 'system:protected', tab: 'protected', label: 'Protected', title: 'Protected', icon: 'protect', feature: 'protection', association: 'protect', membership: 'protected', ordering: 'chronological',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'protect' }),
    emptyTitle: 'No protected clips', emptyDescription: 'Protect a clip to keep it safe from automatic cleanup.',
  },
  {
    key: 'system:concealed', tab: 'concealed', label: 'Concealed', title: 'Concealed', icon: 'conceal', feature: 'concealment', association: 'conceal', membership: 'concealed', ordering: 'chronological',
    capabilities: calculated({ acceptsClipDrop: true, dropAction: 'conceal' }),
    emptyTitle: 'No concealed clips', emptyDescription: 'Conceal a clip to hide its contents until revealed.',
  },
  {
    key: 'system:named', tab: 'named', label: 'Named', title: 'Named', icon: 'name', feature: 'naming', association: 'name', membership: 'named', ordering: 'chronological',
    capabilities: calculated(), emptyTitle: 'No named clips', emptyDescription: 'Name a clip to identify it and find it here.',
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

const COLLECTION_MESSAGE_KEYS: Record<string, { label: TranslationKey; tooltip?: TranslationKey; emptyTitle: TranslationKey; emptyDescription: TranslationKey }> = {
  'system:all': {
    label: 'collection.history',
    tooltip: 'collection.activeClipboardHistory',
    emptyTitle: 'collection.noClipsYet',
    emptyDescription: 'collection.copySomethingInAnyApp',
  },
  'system:queue': {
    label: 'collection.queue',
    emptyTitle: 'collection.queueIsEmpty',
    emptyDescription: 'collection.addTextClipsToQueue',
  },
  'system:pinned': {
    label: 'collection.pinned',
    emptyTitle: 'collection.noPinnedClips',
    emptyDescription: 'collection.pinAClip',
  },
  'system:protected': {
    label: 'collection.protected',
    emptyTitle: 'collection.noProtectedClips',
    emptyDescription: 'collection.protectAClip',
  },
  'system:concealed': {
    label: 'collection.concealed',
    emptyTitle: 'collection.noConcealedClips',
    emptyDescription: 'collection.concealAClip',
  },
  'system:named': {
    label: 'collection.named',
    emptyTitle: 'collection.noNamedClips',
    emptyDescription: 'collection.nameAClip',
  },
  'system:noted': {
    label: 'collection.noted',
    emptyTitle: 'collection.noNotedClips',
    emptyDescription: 'collection.addANoteToAClip',
  },
  'system:trash': {
    label: 'collection.trashed',
    emptyTitle: 'collection.trashIsEmpty',
    emptyDescription: 'collection.clipsMovedToTrash',
  },
  'system:search': {
    label: 'collection.search',
    emptyTitle: 'collection.searchYourClips',
    emptyDescription: 'collection.searchActiveAndTrashedClips',
  },
};

function localizeCollection(collection: ClipCollectionDefinition): ClipCollectionDefinition {
  const keys = COLLECTION_MESSAGE_KEYS[collection.key];
  if (!keys) return collection;
  const label = translate(keys.label);
  return {
    ...collection,
    label,
    title: label,
    tooltip: keys.tooltip ? translate(keys.tooltip) : collection.tooltip,
    emptyTitle: translate(keys.emptyTitle),
    emptyDescription: translate(keys.emptyDescription),
  };
}

export type ClipFacetKind = 'clip_type' | 'content_type' | 'file_format' | 'source';

export function clipFacetRoute(kind: ClipFacetKind, value: string): string {
  return `${kind}-${encodeURIComponent(value)}`;
}

export function parseClipFacetRoute(route: string): { kind: ClipFacetKind; value: string } | null {
  const separator = route.indexOf('-');
  if (separator < 1) return null;
  const kind = route.slice(0, separator);
  if (kind !== 'clip_type' && kind !== 'content_type' && kind !== 'file_format' && kind !== 'source') return null;
  try {
    return { kind, value: decodeURIComponent(route.slice(separator + 1)) };
  } catch {
    return null;
  }
}

function facetLabel(kind: ClipFacetKind, value: string) {
  if (kind === 'source') return value || translate('common.unknownSource');
  if (kind === 'clip_type') {
    if (value === 'text') return translate('component.analyticsView.text');
    if (value === 'image') return translate('component.analyticsView.image');
    if (value === 'file') return translate('component.analyticsView.files');
  }
  if (kind === 'file_format') return value.toUpperCase();
  return contentTypeLabel(value);
}

export function getSystemClipCollections(features: Record<FeatureId, boolean>): ClipCollectionDefinition[] {
  return SYSTEM_CLIP_COLLECTIONS
    .filter(({ feature }) => !feature || features[feature])
    .map(localizeCollection);
}

export function getClipCollection(tab: string, bin?: Bin | null): ClipCollectionDefinition | undefined {
  if (tab === 'search') return localizeCollection(SEARCH_COLLECTION);
  const facet = parseClipFacetRoute(tab);
  if (facet) {
    const label = facetLabel(facet.kind, facet.value);
    return {
      key: `${facet.kind}:${facet.value}`,
      tab: 'all',
      label,
      title: label,
      icon: 'all',
      membership: 'facet',
      ordering: 'chronological',
      capabilities: calculated(),
      emptyTitle: translate('collection.noFacetClips', { label: facet.kind === 'source' ? label : label.toLowerCase() }),
      emptyDescription: facet.kind !== 'source'
        ? translate('collection.typeClipsAppearAutomatically', { label: label.toLowerCase() })
        : translate('collection.sourceClipsAppearAutomatically', { label }),
    };
  }
  if (tab === 'bin') {
    const isSmart = Boolean(bin?.smart_rule);
    return {
      key: `bin:${bin?.id ?? 'none'}`,
      tab: 'bin',
      label: bin?.name ?? translate('collection.bin'),
      title: bin?.name ?? translate('collection.bin'),
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
      emptyTitle: isSmart
        ? translate('collection.noMatchingClips')
        : bin
          ? translate('collection.noClipsInBin', { name: bin.name })
          : translate('collection.thisBinIsEmpty'),
      emptyDescription: isSmart
        ? translate('collection.smartBinMatchesAppearAutomatically', { name: bin?.name ?? translate('collection.thisBin') })
        : translate('collection.dragClipsHere'),
    };
  }
  const collection = SYSTEM_CLIP_COLLECTIONS.find((candidate) => candidate.tab === tab);
  return collection ? localizeCollection(collection) : undefined;
}
