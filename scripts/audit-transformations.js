import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const pipelineEditor = read('src/components/PipelineEditorModal.tsx');
const commands = read('src-tauri/src/commands.rs');
const service = read('src-tauri/src/transformation_service.rs');
const detector = read('src/utils/smartPipelineDetector.ts');
const clipPreview = read('src/components/ClipPreview.tsx');
const sound = read('src/utils/sound.ts');
const app = read('src/App.tsx');
const analytics = read('src/components/AnalyticsView.tsx');
const database = read('src-tauri/src/db.rs');
const operationEditor = read('src/components/OperationEditorModal.tsx');
const operationManager = read('src/components/OperationsManager.tsx');
const compactEditorTypography = [
  'src/components/PipelineEditorModal.tsx',
  'src/components/OperationEditorModal.tsx',
  'src/components/IntentTransformComposer.tsx',
  'src/components/TransformationLibrary.tsx',
].map(read).join('\n');

assert.match(pipelineEditor, /startPipelinePreview\(testInput, steps\.map\(compilePipelineStep\)\)/,
  'The Pipeline editor must preview the complete unsaved Pipeline through the shared executor');
assert.doesNotMatch(pipelineEditor, /invoke<string>\('transform_text'/,
  'The Pipeline editor must not preview built-ins through the legacy bridge');
assert.match(commands, /pub async fn preview_pipeline_steps/,
  'The GUI must expose canonical unsaved Pipeline previews');
assert.match(service, /pub fn preview_pipeline_steps/,
  'Unsaved Pipeline preview must live in the shared Rust transformation service');
assert.match(service, /unsaved_pipeline_preview_uses_the_canonical_operation_executor/,
  'Canonical Pipeline preview must retain native regression coverage');
assert.doesNotMatch(compactEditorTypography, /\btext-(?:sm|base|lg|xl|2xl)\b/,
  'Transformation editors must use the compact GUI typography scale');
assert.doesNotMatch(operationEditor, /<datalist|list="custom-operation-categories"/,
  'Operation categories must use the shared searchable menu instead of a native datalist');
assert.match(operationEditor, /searchPlaceholder="Search categories…"/,
  'The Operation category menu must remain searchable');
assert.match(operationManager, /theme-subtle-surface divide-y theme-divide/,
  'Operation contracts must use the compact shared definition well');

for (const command of ['create_pipeline', 'delete_pipeline']) {
  const start = commands.indexOf(`pub fn ${command}`);
  assert.notEqual(start, -1, `${command} must remain available`);
  const body = commands.slice(start, start + 900);
  assert.match(body, /register_all_app_shortcuts\(&app\)/,
    `${command} must reconcile global shortcuts immediately`);
}

assert.match(detector, /SavedTransform/, 'Smart Actions must understand Saved Transforms');
assert.match(detector, /recommendedTransforms/, 'Smart Actions must return modern Transform recommendations');
assert.match(clipPreview, /recommendedTransforms\.map/, 'Clip Preview must render modern Transform recommendations');

assert.match(sound, /setEnabled\(enabled: boolean\)/, 'Interaction sound state must have one global authority');
assert.match(app, /soundManager\.setEnabled\(appSettings\.enableSounds\)/,
  'The hydrated setting must configure the global sound authority');
assert.doesNotMatch(read('src/components/ClipPreview.tsx'), /play(?:Copy|Paste|Stack)Sound\(true\)/,
  'Clip Preview must not bypass Interaction Sounds');

assert.doesNotMatch(analytics, /Storage Compressed|['"]VS Code['"]|(?:bg|text|border)-blue-/,
  'Analytics must not present estimated compression, invented defaults, or palette-specific styling');
assert.doesNotMatch(database, /kb_saved/, 'The database must not expose fabricated storage-compression metrics');

console.log('Transformations and neglected-surface audit passed.');
