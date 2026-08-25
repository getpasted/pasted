import type { ClipPreviewRevisionsController } from '../hooks/useClipPreviewRevisions';
import { ClipRevisionHistory } from './ClipRevisionHistory';

export function ClipPreviewRevisionHistoryPanel({
  visible,
  readOnly,
  revisions,
}: {
  visible: boolean;
  readOnly: boolean;
  revisions: ClipPreviewRevisionsController;
}) {
  if (!visible) return null;
  return <ClipRevisionHistory
    versions={revisions.versions}
    versionCount={revisions.count}
    isLoading={revisions.isLoading}
    readOnly={readOnly}
    onClose={() => revisions.setIsOpen(false)}
    previewedVersionId={revisions.previewedVersion?.id ?? null}
    restoringVersionId={revisions.restoringVersionId}
    deletingVersionId={revisions.deletingVersionId}
    hasMore={revisions.hasMore}
    isLoadingMore={revisions.isLoadingMore}
    onLoadMore={() => void revisions.loadMore()}
    onPreview={revisions.togglePreview}
    onPreviewStart={revisions.beginTransientPreview}
    onPreviewEnd={revisions.endTransientPreview}
    onRestore={(version) => void revisions.restore(version)}
    onDelete={revisions.deleteVersion}
  />;
}
