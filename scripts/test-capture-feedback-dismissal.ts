import assert from 'node:assert/strict';
import { createCaptureFeedbackDismissal } from '../src/components/captureFeedbackDismissal.ts';

function fixture() {
  let now = 0;
  let sequence = 0;
  let delay = 2;
  const tasks = new Map<number, { at: number; callback: () => void }>();
  const items = [{ id: 1, clip: { isPinned: false }, exiting: false }];
  const phases = new Map<number, string>();
  const finished: number[] = [];
  const controller = createCaptureFeedbackDismissal({
    items: () => items,
    delaySeconds: () => delay,
    setPhase: (id, phase) => {
      phases.set(id, phase);
      const item = items.find((candidate) => candidate.id === id);
      if (item) item.exiting = phase === 'collapsing';
    },
    finish: (id) => { finished.push(id); },
    setTimeout: (callback, milliseconds) => {
      const id = ++sequence;
      tasks.set(id, { at: now + milliseconds, callback });
      return id;
    },
    clearTimeout: (id) => { tasks.delete(id); },
  });
  const advance = (milliseconds: number) => {
    const end = now + milliseconds;
    while (true) {
      const next = [...tasks].sort((a, b) => a[1].at - b[1].at)[0];
      if (!next || next[1].at > end) break;
      now = next[1].at;
      tasks.delete(next[0]);
      next[1].callback();
    }
    now = end;
  };
  return { controller, advance, items, phases, finished, tasks, setDelay: (value: number) => { delay = value; } };
}

// Hover arrives before asynchronous display finishes scheduling the timer.
{
  const f = fixture();
  f.controller.setInteraction('native-pointer', true);
  f.controller.schedule(1);
  f.advance(10_000);
  assert.equal(f.phases.size, 0);
  assert.deepEqual(f.finished, []);
  f.controller.setInteraction('native-pointer', false);
  f.advance(1_999);
  assert.equal(f.phases.size, 0);
  f.advance(1_161);
  assert.deepEqual(f.finished, [1]);
}

// Unpinning and newly arriving cards cannot restart dismissal during hover.
{
  const f = fixture();
  f.controller.setInteraction('dom-pointer', true);
  f.items[0].clip.isPinned = true;
  f.controller.pause(1);
  f.items[0].clip.isPinned = false;
  f.controller.schedule(1);
  f.items.push({ id: 2, clip: { isPinned: false }, exiting: false });
  f.controller.schedule(2);
  f.advance(10_000);
  assert.deepEqual(f.finished, []);
  f.controller.setInteraction('dom-pointer', false);
  f.advance(3_160);
  assert.deepEqual(f.finished, [1, 2]);
}

// Hover and focus independently hold the stack; clearing one is insufficient.
{
  const f = fixture();
  f.controller.schedule(1);
  f.controller.setInteraction('native-pointer', true);
  f.controller.setInteraction('focus', true);
  f.controller.setInteraction('native-pointer', false);
  f.advance(10_000);
  assert.deepEqual(f.finished, []);
  f.controller.setInteraction('focus', false);
  f.advance(3_160);
  assert.deepEqual(f.finished, [1]);
}

// Interaction restores a card during both fade and automatic collapse.
for (const elapsed of [2_500, 3_080]) {
  const f = fixture();
  f.controller.schedule(1);
  f.advance(elapsed);
  f.controller.setInteraction('native-pointer', true);
  assert.equal(f.phases.get(1), 'visible');
  assert.equal(f.items[0].exiting, false);
  f.advance(10_000);
  assert.deepEqual(f.finished, []);
  f.controller.setInteraction('native-pointer', false);
  f.advance(3_160);
  assert.deepEqual(f.finished, [1]);
}

// Pinned cards, disabled timers, explicit cancellation, and teardown stay safe.
{
  const f = fixture();
  f.items[0].clip.isPinned = true;
  f.controller.schedule(1);
  assert.equal(f.tasks.size, 0);
  f.items[0].clip.isPinned = false;
  f.setDelay(0);
  f.controller.schedule(1);
  assert.equal(f.tasks.size, 0);
  f.setDelay(2);
  f.controller.schedule(1);
  f.controller.pause(1);
  f.advance(10_000);
  assert.deepEqual(f.finished, []);
  f.controller.schedule(1);
  f.controller.clear();
  f.advance(10_000);
  assert.deepEqual(f.finished, []);
}
console.log('Capture feedback interaction and dismissal regression checks passed.');
