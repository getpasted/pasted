import { History, Trash2 } from 'lucide-react';
import { translate } from '../localization/runtime';

export function ActivityVersionEventBadge({ type }: { type: string }) {
  const deleted = type === 'clip_version_deleted';
  return (
    <div className={`${deleted ? 'theme-status-danger' : 'theme-status-info'} flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold`}>
      {deleted
        ? <Trash2 className="w-3.5 h-3.5" />
        : <History className="w-3.5 h-3.5" />}
      <span>
        {translate(deleted
          ? 'component.activityLogView.versionDeleted'
          : 'component.activityLogView.versionRestored')}
      </span>
    </div>
  );
}
