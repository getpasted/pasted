import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import { CONTENT_TYPES, setRuntimeContentTypes } from '../utils/contentTypes';

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
  { id: 'general', label: 'General', sortOrder: 10, isBuiltin: true, isArchived: false, defaults: { label: 'General', sortOrder: 10 } },
  { id: 'developer', label: 'Developer', sortOrder: 20, isBuiltin: true, isArchived: false, defaults: { label: 'Developer', sortOrder: 20 } },
  { id: 'personal_financial', label: 'Personal and financial', sortOrder: 30, isBuiltin: true, isArchived: false, defaults: { label: 'Personal and financial', sortOrder: 30 } },
  { id: 'identifiers', label: 'Identifiers', sortOrder: 40, isBuiltin: true, isArchived: false, defaults: { label: 'Identifiers', sortOrder: 40 } },
  { id: 'custom', label: 'Custom', sortOrder: 50, isBuiltin: true, isArchived: false, defaults: { label: 'Custom', sortOrder: 50 } },
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
    const loaded = await invoke<RegisteredContentType[]>('get_content_types', { includeArchived: true });
    setRuntimeContentTypes(loaded);
    setDefinitions(loaded);
    return loaded;
  }, []);
  const refreshGroups = useCallback(async () => {
    const loaded = await invoke<RegisteredContentTypeGroup[]>('get_content_type_groups', { includeArchived: true });
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
