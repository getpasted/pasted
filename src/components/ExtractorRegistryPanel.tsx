import { Copy, Plus, ScanText, Trash2 } from 'lucide-react';

import { translate } from '../localization/runtime';
import { localizedBuiltinDescription, localizedBuiltinName } from '../localization/presentation';
import { AppDialogButton } from './AppDialogLayout';
import type { ContentExtractor } from './contentExtractorModel';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelFooter } from './RegistryPanelFooter';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { SettingsSwitch } from './SettingsSwitch';

export function ExtractorRegistryPanel({
  extractors,
  selectedId,
  selected,
  isDirty,
  saving,
  loading,
  onNew,
  onSelect,
  onToggle,
  onDuplicate,
  onRemove,
}: {
  extractors: ContentExtractor[];
  selectedId: number | 'new' | null;
  selected?: ContentExtractor;
  isDirty: boolean;
  saving: boolean;
  loading: boolean;
  onNew: () => void;
  onSelect: (id: number) => void;
  onToggle: (extractor: ContentExtractor) => void;
  onDuplicate: () => void | Promise<void>;
  onRemove: () => void;
}) {
  return <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
    <RegistryPanelHeader title={translate('component.contentExtractorManagerDialog.extractors')} actions={<AppDialogButton onClick={onNew} className="h-7 min-h-7 px-2.5"><Plus className="h-3.5 w-3.5" /> {translate('common.new')}</AppDialogButton>} />
    <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
      {loading && extractors.length === 0 && (
        <p className="theme-text-muted px-3 py-4 text-center text-[10px]" role="status">
          {translate('component.contentExtractorManagerDialog.loadingExtractors')}
        </p>
      )}
      {!loading && extractors.length === 0 && (
        <p className="theme-text-muted px-3 py-4 text-center text-[10px]">{translate('component.contentExtractorManagerDialog.noExtractorsAreAvailableForEnabledFunctionality')}</p>
      )}
      {extractors.map((extractor) => {
        const displayName = localizedBuiltinName('extractor', extractor.stableRef, extractor.name, extractor.isBuiltin, extractor.defaults?.name);
        const displayDescription = localizedBuiltinDescription('extractor', extractor.stableRef, extractor.description, extractor.isBuiltin, extractor.defaults?.description);
        return <RegistryListItem
          key={extractor.id}
          selected={selectedId === extractor.id}
          onSelect={() => onSelect(extractor.id)}
          icon={<ScanText className="h-4 w-4" />}
          title={displayName}
          subtitle={extractor.isAvailable ? displayDescription : extractor.unavailableReason}
          muted={!extractor.isAvailable}
          trailing={<SettingsSwitch
            checked={extractor.enabled}
            label={displayName}
            onClick={() => onToggle(extractor)}
          />}
        />;
      })}
    </div>
    <RegistryPanelFooter align="end">
      <AppDialogButton onClick={() => void onDuplicate()} disabled={!selected || isDirty || saving} title={isDirty ? translate('component.contentExtractorManagerDialog.saveOrCancelChangesBeforeDuplicating') : undefined}><Copy className="h-3.5 w-3.5" /> {translate('common.duplicate')}</AppDialogButton>
      <AppDialogButton variant="danger" onClick={onRemove} disabled={!selected || saving}><Trash2 className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.delete')}</AppDialogButton>
    </RegistryPanelFooter>
  </section>;
}
