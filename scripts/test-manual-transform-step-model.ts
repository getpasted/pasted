import assert from 'node:assert/strict';
import {
  compileManualTransformStep,
  pipelineStepToEditorStep,
} from '../src/components/manualTransformStepModel.ts';

const regexStep = pipelineStepToEditorStep({
  position: 0,
  operationRef: 'builtin:regex',
  configJson: JSON.stringify({
    pattern: 'hello',
    replacement: 'goodbye',
    matchMode: 'literal',
    caseSensitive: true,
  }),
  failurePolicy: 'stop',
}, 0);

assert.equal(regexStep.findPattern, 'hello');
assert.equal(regexStep.replacePattern, 'goodbye');
assert.equal(regexStep.matchMode, 'literal');
assert.equal(regexStep.caseSensitive, true);
assert.deepEqual(JSON.parse(compileManualTransformStep(regexStep).configJson ?? ''), {
  pattern: 'hello',
  replacement: 'goodbye',
  matchMode: 'literal',
  caseSensitive: true,
});

const malformedQuote = pipelineStepToEditorStep({
  position: 1,
  operationRef: 'builtin:quote_text',
  configJson: '{invalid',
  failurePolicy: 'skip',
}, 1);
assert.equal(malformedQuote.quoteBefore, '> ');
assert.equal(malformedQuote.applyToEachLine, true);

const shell = compileManualTransformStep({
  id: 'shell',
  operation_ref: 'builtin:shell_script',
  shellCommand: '',
});
assert.equal(shell.configJson, 'cat');
assert.equal(shell.failurePolicy, 'stop');

console.log('Manual Transform step model tests passed.');
