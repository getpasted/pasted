import { createElement, useState } from 'react';
import { analysisResetChanges } from '../analysisResetChanges';
import { SettingsResetChanges } from '../components/SettingsResetChanges';
import type { ConfirmationDialogRequest } from '../components/ConfirmationDialog';
import { useToast } from '../components/ToastProvider';
import { translate } from '../localization/runtime';
import { errorMessage } from '../utils/errors';
import { safeInvoke as invoke } from '../utils/tauri';

export function useAnalysisReset({
  refreshContentTypes,
  refreshGroups,
  requestConfirmation,
}: {
  refreshContentTypes: () => unknown | Promise<unknown>;
  refreshGroups: () => unknown | Promise<unknown>;
  requestConfirmation: (request: ConfirmationDialogRequest) => void;
}) {
  const { showToast } = useToast();
  const [restoring, setRestoring] = useState(false);

  const restoreConfirmed = async () => {
    setRestoring(true);
    try {
      await Promise.all([
        invoke('restore_default_content_classifiers'), invoke('restore_default_content_extractors'),
        invoke('restore_default_content_types'), invoke('restore_default_content_type_groups'),
      ]);
      await Promise.all([refreshContentTypes(), refreshGroups()]);
      showToast({ tone: 'success', get message() { return translate('component.settingsAnalysisPanel.shippedAnalysisDefaultsRestoredCustomDefinitionsWerePreserved'); } });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setRestoring(false);
    }
  };

  const restoreAnalysis = async () => {
    setRestoring(true);
    try {
      const changes = await analysisResetChanges();
      requestConfirmation({
        title: translate('component.settingsAnalysisPanel.resetShippedAnalysisDefinitions'),
        description: translate('component.settingsResetChanges.description'),
        details: createElement(SettingsResetChanges, { changes }),
        confirmLabel: translate('common.reset'),
        confirmDisabled: changes.length === 0,
        onConfirm: restoreConfirmed,
      });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setRestoring(false);
    }
  };

  return { restoring, restoreAnalysis };
}
