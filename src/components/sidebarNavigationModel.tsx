import React from 'react';
import {
  Activity,
  BarChart3,
  Clipboard,
  EyeOff,
  HelpCircle,
  ListOrdered,
  Pin,
  Settings,
  Shield,
  StickyNote,
  Trash2,
  Workflow,
  FilePenLine,
} from 'lucide-react';

import { translate } from '../localization/runtime';
import type { ClipCollectionSummary, SequentialStatus } from '../types';
import { CLIP_PROPERTY_ASSOCIATIONS } from '../utils/clipPropertyAssociations';
import {
  getSystemClipCollections,
  type ClipCollectionIcon,
  type ClipDropAction,
} from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';

export interface SidebarNavItem {
  tab: string;
  label: string;
  title: string;
  tooltip?: string;
  icon: React.ReactElement<{ className: string; strokeWidth?: number }>;
  dropAction?: ClipDropAction;
}

function collectionIcon(icon: ClipCollectionIcon) {
  if (icon === 'queue') return <ListOrdered className="sidebar-icon-secondary w-5 h-5" />;
  if (icon === 'pin') return <Pin className="sidebar-icon-success w-5 h-5 pin-icon" />;
  if (icon === 'protect') return <Shield className="sidebar-icon-info w-5 h-5" />;
  if (icon === 'conceal') return <EyeOff className="sidebar-icon-warning w-5 h-5" />;
  if (icon === 'name') return <FilePenLine className="sidebar-icon-named w-5 h-5" />;
  if (icon === 'note') return <StickyNote className="sidebar-icon-note w-5 h-5" />;
  if (icon === 'trash') return <Trash2 className="sidebar-icon-danger w-5 h-5" />;
  return <Clipboard className="sidebar-icon-primary w-5 h-5" />;
}

export function buildSidebarNavigation(features: Record<FeatureId, boolean>) {
  const clipNavItems: SidebarNavItem[] = getSystemClipCollections(features).map((collection) => ({
    ...collection,
    icon: collectionIcon(collection.icon),
    dropAction: collection.capabilities.dropAction,
  }));
  const allToolNavItems: Array<SidebarNavItem & { feature?: FeatureId }> = [
    { tab: 'transformations', label: translate('destination.transformations'), title: translate('destination.transformations'), icon: <Workflow className="sidebar-icon-primary w-5 h-5" />, feature: 'transformations' },
    { tab: 'analytics', label: translate('destination.insights'), title: translate('destination.insights'), icon: <BarChart3 className="sidebar-icon-primary w-5 h-5" />, feature: 'analytics' },
    { tab: 'activity', label: translate('destination.activity'), title: translate('destination.activity'), icon: <Activity className="sidebar-icon-info w-5 h-5" />, feature: 'activityLog' },
    { tab: 'help', label: translate('destination.help'), title: translate('destination.help'), icon: <HelpCircle className="sidebar-icon-info w-5 h-5" />, feature: 'help' },
    { tab: 'settings', label: translate('destination.settings'), title: translate('destination.settings'), icon: <Settings className="sidebar-icon-primary w-5 h-5" /> },
  ];
  return {
    clipNavItems,
    toolNavItems: allToolNavItems.filter(({ feature }) => !feature || features[feature]),
  };
}

export function buildClipCountByTab(
  clipCollectionSummary: ClipCollectionSummary,
  totalClipCount: number,
  seqStatus: SequentialStatus | null,
) {
  const counts: Record<string, number> = {
    all: totalClipCount,
    sequential: seqStatus?.total_count ?? 0,
    notes: clipCollectionSummary.notedCount,
    trash: clipCollectionSummary.trashCount,
  };
  for (const association of CLIP_PROPERTY_ASSOCIATIONS) {
    counts[association.membership] = clipCollectionSummary[association.countKey];
  }
  return counts;
}
