import { translate } from '../localization/runtime';

export const UI_COPY = {
  get copy() { return translate('action.copy'); },
  get copied() { return translate('action.copied'); },
  get moveToTrash() { return translate('action.moveToTrash'); },
  get deletePermanently() { return translate('action.deletePermanently'); },
  get restore() { return translate('action.restore'); },
  get pin() { return translate('action.pin'); },
  get unpin() { return translate('action.unpin'); },
  get protect() { return translate('action.protect'); },
  get unprotect() { return translate('action.unprotect'); },
} as const;

export function clipDeleteLabel({
  trashEnabled,
  permanent = false,
}: {
  trashEnabled: boolean;
  permanent?: boolean;
}) {
  return !trashEnabled || permanent ? UI_COPY.deletePermanently : UI_COPY.moveToTrash;
}

export function selectedClipDeleteLabel({
  count,
  trashEnabled,
  permanent = false,
}: {
  count: number;
  trashEnabled: boolean;
  permanent?: boolean;
}) {
  if (count <= 1) return clipDeleteLabel({ trashEnabled, permanent });
  return !trashEnabled || permanent
    ? translate('action.deleteCountPermanently', { count })
    : translate('action.moveCountToTrash', { count });
}
