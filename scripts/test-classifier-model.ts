import assert from 'node:assert/strict';

import {
  classifierDraftIsDirty,
  classifierModifiedFields,
  emptyClassifierInput,
  nextClassifierSelection,
  normalizedClassifierInput,
} from '../src/components/classifierModel.ts';

const baseline = emptyClassifierInput('Custom Classifier');
const normalized = normalizedClassifierInput({
  ...baseline,
  name: '  Custom Classifier  ',
  description: '  Description  ',
}, ' ^one$ \n\n ^two$ ');

assert.equal(normalized.name, 'Custom Classifier');
assert.equal(normalized.description, 'Description');
assert.deepEqual(normalized.patterns, ['^one$', '^two$']);
assert.equal(classifierDraftIsDirty(baseline, baseline), false,
  'an unchanged Classifier draft must remain clean');
assert.equal(classifierDraftIsDirty({ ...baseline, enabled: false }, baseline), true,
  'a changed Classifier field must make the draft dirty');

assert.deepEqual(
  classifierModifiedFields({ ...baseline, name: 'Changed' }, baseline, false),
  {
    name: true,
    content_type: false,
    description: false,
    patterns: false,
    validator: false,
    enabled: false,
    priority: false,
  },
  'modified-field policy must identify only fields that differ from the comparison definition',
);
assert.ok(
  Object.values(classifierModifiedFields({ ...baseline, name: 'Changed' }, baseline, true))
    .every((modified) => !modified),
  'new Classifiers must not present shipped-definition modification markers',
);

const classifiers = [{ id: 12 }, { id: 24 }] as Parameters<typeof nextClassifierSelection>[0];
assert.equal(nextClassifierSelection(classifiers, null), 12,
  'opening Classifier management must select the first available definition');
assert.equal(nextClassifierSelection(classifiers, 24), 24,
  'reloading Classifiers must preserve a selection that still exists');
assert.equal(nextClassifierSelection(classifiers, 99), 12,
  'reloading Classifiers must replace a stale external selection');
assert.equal(nextClassifierSelection([], 99), 'new',
  'an empty Classifier registry must fall back to a new draft');

console.log('Classifier model tests passed.');
