const MAX_ACTIVE_FILE_THUMBNAILS = 2;
const pendingFileThumbnails: Array<{ cancelled: boolean; load: () => Promise<unknown> }> = [];
let activeFileThumbnails = 0;

function pumpFileThumbnails() {
  while (activeFileThumbnails < MAX_ACTIVE_FILE_THUMBNAILS) {
    const task = pendingFileThumbnails.shift();
    if (!task) return;
    if (task.cancelled) continue;
    activeFileThumbnails += 1;
    void Promise.resolve().then(task.load).finally(() => {
      activeFileThumbnails -= 1;
      pumpFileThumbnails();
    });
  }
}

export function queueFileThumbnailLoad(load: () => Promise<unknown>) {
  const task = { cancelled: false, load };
  pendingFileThumbnails.push(task);
  pumpFileThumbnails();
  return () => { task.cancelled = true; };
}

export function loadVisibleThumbnail(stage: HTMLElement, load: () => void | (() => void)): () => void {
  if (typeof IntersectionObserver === 'undefined' || stage.closest('[data-virtual-clip-list]')) {
    return load() || (() => {});
  }
  let stopLoading: void | (() => void);
  const observer = new IntersectionObserver((entries) => {
    if (!entries.some((entry) => entry.isIntersecting)) return;
    observer.disconnect();
    stopLoading = load();
  }, { rootMargin: '240px 0px' });
  observer.observe(stage);
  return () => {
    observer.disconnect();
    stopLoading?.();
  };
}
