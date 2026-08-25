import { translate } from '../localization/runtime';
import type { SmartConditionTarget } from './binModalModel';

export function smartConditionPlaceholder(target: SmartConditionTarget) {
  if (target === 'file_extension') return translate('component.binModal.eGPdfZipPng');
  if (target === 'file_path') return translate('component.binModal.eGProjectsOrDownloads');
  if (target === 'visual_label') return translate('component.binModal.eGDogOrPizza');
  return translate('component.binModal.eGHttpFunction');
}
