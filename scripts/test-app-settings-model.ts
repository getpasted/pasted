import assert from 'node:assert/strict';
import {
  DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP,
  DEFAULT_REVISION_HISTORY_LIMIT,
  isAnalysisFunctionalityEnabled,
  storedRetentionNumber,
} from '../src/appSettingsRetentionModel.ts';

assert.equal(DEFAULT_REVISION_HISTORY_LIMIT, 10);
assert.equal(DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP, 10);
assert.equal(storedRetentionNumber({}, 'analysisAttemptsPerClip', DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP), 10);
assert.equal(storedRetentionNumber({ analysisAttemptsPerClip: '25' }, 'analysisAttemptsPerClip', 10), 25,
  'an existing configured Analysis limit must be preserved');
assert.equal(isAnalysisFunctionalityEnabled({ enableOcr: false, enableTranscriptions: false }), false);
assert.equal(isAnalysisFunctionalityEnabled({ enableOcr: true, enableTranscriptions: false }), true);
assert.equal(isAnalysisFunctionalityEnabled({ enableOcr: false, enableTranscriptions: true }), true);

console.log('App settings model tests passed.');
