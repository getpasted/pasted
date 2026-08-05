import type { ClipItem } from '../types';

export function compareClipChronologicalOrder(left: ClipItem, right: ClipItem) {
  return right.created_at.localeCompare(left.created_at) || right.id - left.id;
}

export function compareClipTimelineOrder(left: ClipItem, right: ClipItem) {
  if (left.is_pinned !== right.is_pinned) return left.is_pinned ? -1 : 1;
  if (left.is_pinned) {
    const pinDifference = (left.pin_order ?? 0) - (right.pin_order ?? 0);
    if (pinDifference !== 0) return pinDifference;
  }
  return compareClipChronologicalOrder(left, right);
}

export function sortClipsForTimeline(clips: ClipItem[]) {
  return [...clips].sort(compareClipTimelineOrder);
}

export function sortClipsChronologically(clips: ClipItem[]) {
  return [...clips].sort(compareClipChronologicalOrder);
}
