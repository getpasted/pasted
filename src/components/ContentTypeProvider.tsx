import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import { analysisApi } from '../api/analysis';
import { CONTENT_TYPES, setRuntimeContentTypes } from '../utils/contentTypes';
import { translate } from '../localization/runtime';

export interface RegisteredContentType {
  id: string;
  label: string;
  icon: string;
  group: string;
  isBuiltin: boolean;
  isArchived: boolean;
  defaults: Pick<RegisteredContentType, 'label' | 'icon' | 'group'> | null;
}

export interface RegisteredContentTypeGroup {
  id: string;
  label: string;
  sortOrder: number;
  isBuiltin: boolean;
  isArchived: boolean;
  defaults: Pick<RegisteredContentTypeGroup, 'label' | 'sortOrder'> | null;
}

const fallbackTypes: RegisteredContentType[] = CONTENT_TYPES.map(({ value, label, icon, group }) => {
  const groupId = group.toLowerCase().replace(/ & /g, '_').replace(/ /g, '_');
  return { id: value, label, icon, group: groupId, isBuiltin: true, isArchived: false, defaults: { label, icon, group: groupId } };
});

const fallbackGroups: RegisteredContentTypeGroup[] = [
  { id: 'general', get label() { return translate('component.contentTypeProvider.general'); }, sortOrder: 10, isBuiltin: true, isArchived: false, defaults: { get label() { return translate('component.contentTypeProvider.general'); }, sortOrder: 10 } },
  { id: 'developer', get label() { return translate('component.contentTypeProvider.developer'); }, sortOrder: 20, isBuiltin: true, isArchived: false, defaults: { get label() { return translate('component.contentTypeProvider.developer'); }, sortOrder: 20 } },
  { id: 'personal_financial', get label() { return translate('component.contentTypeProvider.personalAndFinancial'); }, sortOrder: 30, isBuiltin: true, isArchived: false, defaults: { get label() { return translate('component.contentTypeProvider.personalAndFinancial'); }, sortOrder: 30 } },
  { id: 'identifiers', get label() { return translate('component.contentTypeProvider.identifiers'); }, sortOrder: 40, isBuiltin: true, isArchived: false, defaults: { get label() { return translate('component.contentTypeProvider.identifiers'); }, sortOrder: 40 } },
  { id: 'custom', get label() { return translate('common.custom'); }, sortOrder: 50, isBuiltin: true, isArchived: false, defaults: { get label() { return translate('common.custom'); }, sortOrder: 50 } },
];

const ContentTypeContext = createContext({
  definitions: fallbackTypes,
  groups: fallbackGroups,
  refresh: async () => fallbackTypes,
  refreshGroups: async () => fallbackGroups,
});

export function ContentTypeProvider({ children }: { children: ReactNode }) {
  const [definitions, setDefinitions] = useState(fallbackTypes);
  const [groups, setGroups] = useState(fallbackGroups);
  const refresh = useCallback(async () => {
    const loaded = await analysisApi.listContentTypes<RegisteredContentType>();
    setRuntimeContentTypes(loaded);
    setDefinitions(loaded);
    return loaded;
  }, []);
  const refreshGroups = useCallback(async () => {
    const loaded = await analysisApi.listContentTypeGroups<RegisteredContentTypeGroup>();
    setGroups(loaded);
    return loaded;
  }, []);

  useEffect(() => { void refresh(); void refreshGroups(); }, [refresh, refreshGroups]);
  const value = useMemo(() => ({ definitions, groups, refresh, refreshGroups }), [definitions, groups, refresh, refreshGroups]);
  return <ContentTypeContext.Provider value={value}>{children}</ContentTypeContext.Provider>;
}

export function useContentTypes() {
  return useContext(ContentTypeContext);
}
