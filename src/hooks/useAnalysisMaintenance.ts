import { useState } from 'react';

import { analysisApi } from '../api/analysis';
import type { ConfirmationDialogRequest } from '../components/ConfirmationDialog';
import { useToast } from '../components/ToastProvider';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import { errorMessage } from '../utils/errors';
import { safeInvoke as invoke } from '../utils/tauri';

interface AnalysisRescanReport {
  scannedCount: number;
  changedCount: number;
  unchangedCount: number;
  missingCount?: number;
  failedCount: number;
}

export function useAnalysisMaintenance({
  contentClassificationEnabled,
  fileFormatsEnabled,
  refreshContentTypes,
  refreshGroups,
}: {
  contentClassificationEnabled: boolean;
  fileFormatsEnabled: boolean;
  refreshContentTypes: () => unknown | Promise<unknown>;
  refreshGroups: () => unknown | Promise<unknown>;
}) {
  const { showToast } = useToast();
  const { locale } = useLocalization();
  const [restoring, setRestoring] = useState(false);
  const [rescanning, setRescanning] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const requestConfirmation = (request: ConfirmationDialogRequest) => {
    setConfirmation({
      ...request,
      onConfirm: async () => {
        setConfirmation(null);
        await request.onConfirm();
      },
    });
  };

  const restoreAnalysisConfirmed = async () => {
    setRestoring(true);
    try {
      await Promise.all([
        invoke('restore_default_content_classifiers'),
        invoke('restore_default_content_extractors'),
        invoke('restore_default_content_types'),
        invoke('restore_default_content_type_groups'),
      ]);
      await Promise.all([refreshContentTypes(), refreshGroups()]);
      showToast({ tone: 'success', get message() { return translate('component.settingsAnalysisPanel.shippedAnalysisDefaultsRestoredCustomDefinitionsWerePreserved'); } });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setRestoring(false);
    }
  };

  const restoreAnalysis = () => requestConfirmation({
    get title() { return translate('component.settingsAnalysisPanel.resetShippedAnalysisDefinitions'); },
    get description() { return translate('component.settingsAnalysisPanel.shippedExtractorsClassifiersContentTypesAndContentTypeGroupsReturnToTheir'); },
    details: translate('component.settingsAnalysisPanel.customDefinitionsRemainUnchanged'),
    confirmLabel: translate('common.reset'),
    onConfirm: restoreAnalysisConfirmed,
  });

  const rescanHistoryConfirmed = async () => {
    setRescanning(true);
    try {
      const reports = await Promise.all([
        contentClassificationEnabled
          ? analysisApi.rescanClassifications<AnalysisRescanReport>()
          : Promise.resolve(null),
        fileFormatsEnabled
          ? analysisApi.rescanFileFormats<AnalysisRescanReport>()
          : Promise.resolve(null),
      ]);
      const scannedCount = reports.reduce((total, report) => total + (report?.scannedCount ?? 0), 0);
      const changedCount = reports.reduce((total, report) => total + (report?.changedCount ?? 0), 0);
      const unchangedCount = reports.reduce((total, report) => total + (report?.unchangedCount ?? 0), 0);
      const missingCount = reports.reduce((total, report) => total + (report?.missingCount ?? 0), 0);
      const failedCount = reports.reduce((total, report) => total + (report?.failedCount ?? 0), 0);
      const details = [
        changedCount > 0 ? translate('component.settingsAnalysisPanel.rescanUpdated', { count: changedCount }) : null,
        unchangedCount > 0 ? translate('component.settingsAnalysisPanel.rescanUnchanged', { count: unchangedCount }) : null,
        missingCount > 0 ? translate('component.settingsAnalysisPanel.rescanMissing', { count: missingCount }) : null,
        failedCount > 0 ? translate('component.settingsAnalysisPanel.rescanFailed', { count: failedCount }) : null,
      ].filter((detail): detail is string => detail !== null);
      showToast({
        tone: failedCount > 0 ? 'info' : 'success',
        message: details.length > 0
          ? translate('component.settingsAnalysisPanel.rescanSummary', {
            count: scannedCount,
            details: new Intl.ListFormat(locale, { style: 'short', type: 'conjunction' }).format(details),
          })
          : translate('component.settingsAnalysisPanel.rescanSummaryEmpty', { count: scannedCount }),
      });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setRescanning(false);
    }
  };

  const rescanHistory = () => requestConfirmation({
    get title() { return translate('component.settingsAnalysisPanel.rescanExistingClips'); },
    get description() { return translate('component.settingsAnalysisPanel.enabledScannersWillRefreshDerivedClipData'); },
    details: translate('component.settingsAnalysisPanel.rescanCanChangeDerivedOrganization'),
    confirmLabel: translate('component.settingsAnalysisPanel.rescanClips'),
    onConfirm: rescanHistoryConfirmed,
  });

  return {
    confirmation,
    rescanning,
    rescanHistory,
    restoring,
    restoreAnalysis,
    setConfirmation,
  };
}
