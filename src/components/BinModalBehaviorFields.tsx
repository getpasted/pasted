import type { BinModalFormController } from '../hooks/useBinModalForm';
import { translate } from '../localization/runtime';
import type { SmartBinFeatures } from './binModalModel';
import { MenuSelect } from './MenuSelect';
import { SettingsSwitch } from './SettingsSwitch';

interface BinModalBehaviorFieldsProps {
  form: BinModalFormController;
  features: SmartBinFeatures;
}

export function BinModalBehaviorFields({ form, features }: BinModalBehaviorFieldsProps) {
  const {
    modalTab,
    transforms,
    transformRef,
    setTransformRef,
    protectClips,
    setProtectClips,
    concealClips,
    setConcealClips,
  } = form;

  return <>
    <div className="flex items-center gap-3">
      <span className="w-20 shrink-0 text-end text-xs font-semibold theme-text-muted">
        {translate('component.binModal.transform')}
      </span>
      <MenuSelect
        value={transformRef}
        options={[
          { value: '', get label() { return translate('component.binModal.doNothing'); } },
          ...transforms
            .map((transform, sourceIndex) => {
              const group = transform.authoringKind === 'manual'
                ? translate('component.transformationPlayground.manuallyBuiltTransforms')
                : transform.executionCharacter === 'replayable'
                  ? translate('component.transformationPlayground.plannedLocalTransforms')
                  : translate('component.transformationPlayground.aiAssistedTransforms');
              const groupOrder = transform.authoringKind === 'manual'
                ? 2
                : transform.executionCharacter === 'replayable' ? 1 : 0;
              return {
                value: transform.stableRef,
                label: transform.name,
                group,
                groupOrder,
                sourceIndex,
              };
            })
            .sort((left, right) => left.groupOrder - right.groupOrder || left.sourceIndex - right.sourceIndex)
            .map(({ groupOrder: _groupOrder, sourceIndex: _sourceIndex, ...option }) => option),
        ]}
        onChange={setTransformRef}
        label={translate('component.binModal.transform')}
        className="min-w-0 flex-1"
        searchable
        searchPlaceholder={translate('component.binModal.searchTransforms')}
      />
    </div>

    {features.protection && (
      <div className="flex items-center gap-3">
        <span className="w-20 shrink-0 text-end text-xs font-semibold theme-text-muted">
          {translate('component.binModal.protect')}
        </span>
        <div
          className={`bin-setting-toggle-well theme-surface flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl border p-3 ${modalTab === 'bin' ? 'cursor-pointer' : 'is-disabled cursor-help'}`}
          title={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotProtectClips') : undefined}
          onClick={(event) => {
            if (modalTab !== 'bin' || (event.target as HTMLElement).closest('button')) return;
            setProtectClips((value) => !value);
          }}
        >
          <div className="min-w-0">
            <div className="text-xs font-semibold theme-text-main">
              {translate('component.binModal.clipsInThisBinAreSafeFromDeletion')}
            </div>
          </div>
          <SettingsSwitch
            checked={modalTab === 'bin' && protectClips}
            disabled={modalTab === 'smart'}
            label={translate('component.binModal.clipsInThisBinAreSafeFromDeletion')}
            ariaLabel={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotProtectClips') : undefined}
            onClick={() => setProtectClips((value) => !value)}
          />
        </div>
      </div>
    )}

    {features.concealment && (
      <div className="flex items-center gap-3">
        <span className="w-20 shrink-0 text-end text-xs font-semibold theme-text-muted">
          {translate('component.binModal.conceal')}
        </span>
        <div
          className={`bin-setting-toggle-well theme-surface flex min-w-0 flex-1 items-center justify-between gap-3 rounded-xl border p-3 ${modalTab === 'bin' ? 'cursor-pointer' : 'is-disabled cursor-help'}`}
          title={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotConcealClips') : undefined}
          onClick={(event) => {
            if (modalTab !== 'bin' || (event.target as HTMLElement).closest('button')) return;
            setConcealClips((value) => !value);
          }}
        >
          <div className="min-w-0">
            <div className="text-xs font-semibold theme-text-main">
              {translate('component.binModal.clipsInThisBinAreConcealed')}
            </div>
          </div>
          <SettingsSwitch
            checked={modalTab === 'bin' && concealClips}
            disabled={modalTab === 'smart'}
            label={translate('component.binModal.clipsInThisBinAreConcealed')}
            ariaLabel={modalTab === 'smart' ? translate('component.binModal.smartBinsCannotConcealClips') : undefined}
            onClick={() => setConcealClips((value) => !value)}
          />
        </div>
      </div>
    )}
  </>;
}
