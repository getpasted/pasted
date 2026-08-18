import assert from 'node:assert/strict';
import { errorMessage } from '../src/utils/errors.ts';

assert.equal(errorMessage('Connection failed'), 'Connection failed');
assert.equal(errorMessage(new Error('Connection failed')), 'Connection failed');
assert.equal(
  errorMessage({ code: 'provider_failed', message: 'Codex CLI returned an error.' }),
  'Codex CLI returned an error.',
);
assert.equal(errorMessage({ code: 'provider_failed' }), '{"code":"provider_failed"}');

console.log('Structured error-message tests passed.');
