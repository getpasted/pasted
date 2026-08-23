import React from 'react';
import { Folder } from 'lucide-react';

import { useBinModalForm } from '../hooks/useBinModalForm';
import { translate } from '../localization/runtime';
import type { Bin } from '../types';
import { detectDesktopPlatform } from '../utils/platform';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
  SaveButtonContent,
} from './AppDialogLayout';
import { BinModalBehaviorFields } from './BinModalBehaviorFields';
import { BinModalIdentityFields } from './BinModalIdentityFields';
import { BinModalSmartRules } from './BinModalSmartRules';
import { buildBinModalTargets } from './binModalTargets';
import type { SmartBinFeatures } from './binModalModel';
import { useContentTypes } from './ContentTypeProvider';

interface BinModalProps {
  isOpen: boolean;
  editingBin?: Bin | null;
  features: SmartBinFeatures;
  fileFormats: string[];
  sources: string[];
  onClose: () => void;
  onRefreshBins: () => void;
}

export const BinModal: React.FC<BinModalProps> = ({
  isOpen,
  editingBin,
  features,
  fileFormats,
  sources,
  onClose,
  onRefreshBins,
}) => {
  const {
    definitions: contentTypes,
    groups: contentTypeGroups,
  } = useContentTypes();
  const form = useBinModalForm({
    isOpen,
    editingBin,
    features,
    fileFormats,
    contentTypes,
    onClose,
    onRefreshBins,
  });
  const { targetLabels, targetSectionsFor } = buildBinModalTargets({
    contentTypes,
    contentTypeGroups,
    features,
    fileFormats,
    sources,
    installedApps: form.installedApps,
  });
  const desktopPlatform = detectDesktopPlatform();

  if (!isOpen) return null;

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="bin-modal-title"
      isDirty={form.isDirty}
      panelClassName="bin-modal-card theme-panel w-full max-w-2xl max-h-[90vh] border shadow-2xl overflow-hidden flex flex-col font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading
            id="bin-modal-title"
            title={editingBin
              ? translate('component.binModal.editBin')
              : translate('component.binModal.newBin')}
            description={translate('component.binModal.chooseHowClipsEnterThisBinAndWhatHappensNext')}
            icon={<Folder />}
          />
        </AppDialogHeader>
        <form onSubmit={form.submit} className="flex min-h-0 flex-1 flex-col">
          <AppDialogBody className="space-y-4 text-xs">
            <div className="flex justify-center">
              <div className="flex theme-surface p-1 rounded-xl border space-x-1">
                <button
                  type="button"
                  onClick={() => form.setModalTab('bin')}
                  className={`settings-tab px-4 py-1.5 rounded-lg text-xs font-semibold transition-none border border-transparent ${form.modalTab === 'bin' ? 'is-active' : ''}`}
                >
                  {translate('component.binModal.manual')}
                </button>
                <button
                  type="button"
                  onClick={() => form.setModalTab('smart')}
                  className={`settings-tab px-4 py-1.5 rounded-lg text-xs font-semibold transition-none border border-transparent ${form.modalTab === 'smart' ? 'is-active' : ''}`}
                >
                  {translate('component.binModal.smart')}
                </button>
              </div>
            </div>

            <BinModalIdentityFields form={form} desktopPlatform={desktopPlatform} />
            <BinModalBehaviorFields form={form} features={features} />
            {form.modalTab === 'smart' && (
              <BinModalSmartRules
                form={form}
                targetLabels={targetLabels}
                targetSectionsFor={targetSectionsFor}
              />
            )}
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose}>
              {translate('common.cancel')}
            </AppDialogButton>
            <AppDialogButton type="submit" variant="primary">
              <SaveButtonContent />
            </AppDialogButton>
          </AppDialogFooter>
        </form>
      </>}
    </AppDialog>
  );
};
