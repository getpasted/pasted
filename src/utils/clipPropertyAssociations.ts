import type { ClipItem } from '../types';
import type { FeatureId } from './features';

export type ClipPropertyAssociationId = 'pin' | 'protect' | 'conceal' | 'name';
export type ClipPropertyMembership = 'pinned' | 'protected' | 'concealed' | 'named';
export type ClipPropertyDropAction = 'pin' | 'protect' | 'conceal';

export interface ClipPropertyAssociation {
  id: ClipPropertyAssociationId;
  membership: ClipPropertyMembership;
  dropAction?: ClipPropertyDropAction;
  feature: FeatureId;
  countKey: 'pinnedCount' | 'protectedCount' | 'concealedCount' | 'namedCount';
  isMember: (clip: ClipItem) => boolean;
}

export const CLIP_PROPERTY_ASSOCIATIONS = [
  {
    id: 'pin',
    membership: 'pinned',
    dropAction: 'pin',
    feature: 'pinning',
    countKey: 'pinnedCount',
    isMember: (clip) => Boolean(clip.is_pinned),
  },
  {
    id: 'protect',
    membership: 'protected',
    dropAction: 'protect',
    feature: 'protection',
    countKey: 'protectedCount',
    isMember: (clip) => Boolean(clip.is_protected),
  },
  {
    id: 'conceal',
    membership: 'concealed',
    dropAction: 'conceal',
    feature: 'concealment',
    countKey: 'concealedCount',
    isMember: (clip) => Boolean(clip.is_concealed),
  },
  {
    id: 'name',
    membership: 'named',
    dropAction: undefined,
    feature: 'naming',
    countKey: 'namedCount',
    isMember: (clip) => Boolean(clip.name?.trim()),
  },
] as const satisfies readonly ClipPropertyAssociation[];

export function getClipPropertyAssociation(
  id: ClipPropertyAssociationId | undefined,
): ClipPropertyAssociation | undefined {
  return CLIP_PROPERTY_ASSOCIATIONS.find((association) => association.id === id);
}

export function getClipPropertyAssociationForDropAction(
  action: string,
): ClipPropertyAssociation | undefined {
  return CLIP_PROPERTY_ASSOCIATIONS.find((association) => association.dropAction === action);
}
