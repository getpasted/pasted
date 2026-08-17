import { useEffect, useState } from 'react';
import { Sparkles } from 'lucide-react';
import type { ExecutePlanOutcome, SavedTransform } from '../types';
import { IntentTransformComposer } from './IntentTransformComposer';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { translate } from '../localization/runtime';

interface TransformComposerModalProps {
  isOpen: boolean;
  sampleInput: string;
  transform?: SavedTransform | null;
  onClose: () => void;
  onTestResult: (result: ExecutePlanOutcome) => void;
  onTransformSaved: (transform: SavedTransform) => void;
}

export function TransformComposerModal({
  isOpen,
  sampleInput,
  transform,
  onClose,
  onTestResult,
  onTransformSaved,
}: TransformComposerModalProps) {
  const [isDirty, setIsDirty] = useState(false);

  useEffect(() => {
    if (isOpen) setIsDirty(false);
  }, [isOpen, transform]);

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="transform-composer-title"
      isDirty={isDirty}
      panelClassName="theme-panel max-h-[90vh] w-full max-w-3xl overflow-hidden border"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading
            id="transform-composer-title"
            title={transform ? translate('component.transformComposerModal.editTransform') : translate('component.transformComposerModal.newTransform')}
            description={transform ? translate('component.transformComposerModal.renameItReviseTheIntentOrRebuildItsReusablePlan') : translate('component.transformComposerModal.describeTheResultThenReviewTheGeneratedPlan')}
            icon={<Sparkles />}
            tone="info"
          />
        </AppDialogHeader>
        <AppDialogBody className="tools-scroll-region">
          <IntentTransformComposer
            sampleInput={sampleInput}
            initialTransform={transform}
            onDirtyChange={setIsDirty}
            onCancel={requestClose}
            onTestResult={onTestResult}
            onTransformSaved={onTransformSaved}
            embedded
          />
        </AppDialogBody>
      </>}
    </AppDialog>
  );
}
