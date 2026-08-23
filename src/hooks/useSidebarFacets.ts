import React from 'react';

import { localizedSourceName } from '../localization/presentation';
import { translate } from '../localization/runtime';
import type { ClipCollectionSummary } from '../types';
import { clipFacetRoute } from '../utils/clipCollections';
import { contentTypeLabel } from '../utils/contentTypes';
import { safeInvoke as invoke } from '../utils/tauri';

export interface SidebarFacetItem {
  value: string;
  count: number;
  route: string;
  label: string;
}

export function useSidebarFacets(
  clipCollectionSummary: ClipCollectionSummary,
  contentTypes: Array<{ id: string }>,
  locale: string,
  sourcesEnabled: boolean,
) {
  const typeItems = React.useMemo(() => {
    const order = new Map(contentTypes.map(({ id }, index) => [id, index]));
    const labels = new Map(contentTypes.map(({ id }) => [id, contentTypeLabel(id)]));
    return clipCollectionSummary.typeCounts.map(({ content_type: value, count }) => ({
      value,
      count,
      route: clipFacetRoute('content_type', value),
      label: labels.get(value) ?? value.split('_').map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' '),
    })).sort((left, right) => (order.get(left.value) ?? Number.MAX_SAFE_INTEGER) - (order.get(right.value) ?? Number.MAX_SAFE_INTEGER));
  }, [clipCollectionSummary.typeCounts, contentTypes, locale]);
  const clipTypeItems = React.useMemo(() => {
    const definitions = [
      { value: 'text', label: translate('component.analyticsView.text') },
      { value: 'image', label: translate('component.analyticsView.image') },
      { value: 'file', label: translate('component.analyticsView.files') },
    ];
    const counts = new Map(clipCollectionSummary.clipTypeCounts.map(({ clip_type, count }) => [clip_type, count]));
    return definitions
      .map(({ value, label }) => ({ value, label, count: counts.get(value as 'text' | 'image' | 'file') ?? 0, route: clipFacetRoute('clip_type', value) }))
      .filter(({ count }) => count > 0);
  }, [clipCollectionSummary.clipTypeCounts, locale]);
  const fileFormatItems = React.useMemo(() => clipCollectionSummary.fileFormatCounts.map(({ file_format: value, count }) => ({
    value,
    count,
    route: clipFacetRoute('file_format', value),
    label: value.toUpperCase(),
  })), [clipCollectionSummary.fileFormatCounts]);
  const sourceItems = React.useMemo(() => clipCollectionSummary.sourceCounts.map(({ name: value, count }) => ({
    value,
    count,
    route: clipFacetRoute('source', value),
    label: localizedSourceName(value),
  })).sort((left, right) => right.count - left.count || left.label.localeCompare(right.label)), [clipCollectionSummary.sourceCounts, locale]);
  const [sourceIcons, setSourceIcons] = React.useState<Record<string, string>>({});
  const sourceIconsRef = React.useRef<Record<string, string>>({});
  const requestedSourceIconsRef = React.useRef(new Set<string>());
  const sourceIconNames = React.useMemo(
    () => [...new Set(sourceItems.map(({ value }) => value))].sort((left, right) => left.localeCompare(right)).slice(0, 128),
    [sourceItems],
  );
  const sourceIconSignature = JSON.stringify(sourceIconNames);
  React.useEffect(() => {
    if (!sourcesEnabled || sourceIconNames.length === 0) return undefined;
    const missingSources = sourceIconNames.filter(
      (name) => !sourceIconsRef.current[name] && !requestedSourceIconsRef.current.has(name),
    );
    if (missingSources.length === 0) return undefined;
    missingSources.forEach((name) => requestedSourceIconsRef.current.add(name));
    void invoke<Record<string, string>>('get_source_icons', { sources: missingSources }).then((icons) => {
      const merged = { ...sourceIconsRef.current, ...(icons ?? {}) };
      sourceIconsRef.current = merged;
      setSourceIcons(merged);
    }).catch((error) => {
      console.warn('Source icons are unavailable; restart Pasted after native updates.', error);
      missingSources.forEach((name) => requestedSourceIconsRef.current.delete(name));
    });
    return undefined;
  }, [sourcesEnabled, sourceIconSignature]);

  return { typeItems, clipTypeItems, fileFormatItems, sourceItems, sourceIcons };
}
