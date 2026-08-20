import assert from 'node:assert/strict';
import fs from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const read = (path) => fs.readFileSync(path, 'utf8');
const englishCatalog = JSON.parse(read('src/locales/en.json'));
const manualTransformEditor = read('src/components/ManualTransformEditorModal.tsx');
const commands = read('src-tauri/src/commands.rs');
const service = read('src-tauri/src/transformation_service.rs');
const suggestion = read('src-tauri/src/content_suggestions.rs');
const suggestionExecution = read('src-tauri/src/suggestion_execution.rs');
const clipPreview = read('src/components/ClipPreview.tsx');
const sound = read('src/utils/sound.ts');
const app = read('src/App.tsx');
const analytics = read('src/components/AnalyticsView.tsx');
const database = read('src-tauri/src/db.rs');
const operationEditor = read('src/components/OperationEditorModal.tsx');
const operationManager = read('src/components/OperationsManager.tsx');
const transformationLibrary = read('src/components/TransformationLibrary.tsx');
const transformationPlayground = read('src/components/TransformationPlayground.tsx');
const cli = readRustModuleTree('src-tauri/src/bin/pasted.rs', 'src-tauri/src/cli');
const transformStorageDecision = read('docs/TRANSFORM_STORAGE_DECISION.md');
const compactEditorTypography = [
  'src/components/ManualTransformEditorModal.tsx',
  'src/components/OperationEditorModal.tsx',
  'src/components/IntentTransformComposer.tsx',
  'src/components/TransformationLibrary.tsx',
].map(read).join('\n');

assert.match(manualTransformEditor, /startManualTransformPreview\(testInput, steps\.map\(compileManualTransformStep\)\)/,
  'The Manual Transform editor must preview the complete unsaved manual Transform through the shared executor');
assert.doesNotMatch(manualTransformEditor, /invoke<string>\('transform_text'/,
  'The Manual Transform editor must not preview built-ins through the legacy bridge');
assert.match(commands, /pub async fn preview_manual_transform_steps/,
  'The GUI must expose canonical unsaved manual Transform previews');
assert.match(service, /pub fn preview_manual_transform_steps/,
  'Unsaved manual Transform preview must live in the shared Rust transformation service');
assert.match(service, /unsaved_pipeline_preview_uses_the_canonical_operation_executor/,
  'Canonical manual Transform preview must retain native regression coverage');
assert.doesNotMatch(compactEditorTypography, /\btext-(?:sm|base|lg|xl|2xl)\b/,
  'Transformation editors must use the compact GUI typography scale');
assert.doesNotMatch(operationEditor, /<datalist|list="custom-operation-categories"/,
  'Operation categories must use the shared searchable menu instead of a native datalist');
assert.match(operationEditor, /searchPlaceholder=\{translate\('component\.operationEditorModal\.searchCategories'\)\}/,
  'The Operation category menu must remain searchable');
assert.equal(englishCatalog['component.operationEditorModal.searchCategories'], 'Search categories…');
assert.match(operationManager, /theme-subtle-surface divide-y theme-divide/,
  'Operation contracts must use the compact shared definition well');
assert.match(transformationLibrary, /translate\('component\.transformationLibrary\.buildManually'\)/,
  'The Transform library must present deterministic composition as a creation method, not a separate asset kind');
assert.equal(englishCatalog['component.transformationLibrary.buildManually'], 'Build Manually');
assert.doesNotMatch(transformationLibrary, />Pipelines</,
  'The Transform library must not split compatibility-backed items into a separate Pipeline section');
assert.match(transformationPlayground, /translate\('component\.transformationPlayground\.runATransformOrOperationWithoutChangingAClip'\)/,
  'The playground must use the consolidated Transform vocabulary');
assert.equal(englishCatalog['component.transformationPlayground.runATransformOrOperationWithoutChangingAClip'],
  'Run a Transform or Operation without changing a clip.');
assert.match(cli, /db\.get_transform_definitions\(\)\?/,
  'The primary Transform CLI listing must use the canonical definition facade');
assert.match(service, /ExecutionTarget::ManualTransform \{ transform_ref \} => ExecutionTarget::Transform/,
  'The canonical Transform executor must normalize manual Transform targets immediately');
assert.match(cli, /let target = ExecutionTarget::Transform/,
  'The primary Transform CLI runner must remain storage-agnostic');
for (const lifecycleCommand of ['"get"', '"create" | "new"', '"update" | "edit"', '"duplicate" | "copy"', '"delete" | "remove"']) {
  assert.ok(cli.includes(lifecycleCommand),
    `The primary Transform CLI must expose ${lifecycleCommand} lifecycle parity`);
}
assert.match(database, /pub struct TransformDefinition/,
  'Rust must expose one canonical definition contract over both Transform authoring forms');
assert.match(database, /ALTER TABLE clip_transformations ADD COLUMN transform_ref TEXT/,
  'Pre-1.0 migration must add durable stable-reference provenance');
assert.match(database, /manually_built_transform_applies_with_revision_and_stable_provenance/,
  'Manual Transform replacement must retain focused revision and provenance coverage');
assert.match(database, /fn migrate_pipelines_to_saved_transforms/,
  'Pre-1.0 startup must include the transactional Pipeline-to-Transform migration');
assert.match(database, /legacy_pipelines_migrate_atomically_to_canonical_transforms/,
  'The physical consolidation must retain focused reference-rewrite coverage');
assert.match(transformStorageDecision, /stores every reusable Transform in `saved_transforms`/,
  'The 1.0 storage decision must document the canonical single-table model');

for (const command of ['create_manual_transform', 'delete_manual_transform']) {
  const start = commands.indexOf(`pub fn ${command}`);
  assert.notEqual(start, -1, `${command} must remain available`);
  const body = commands.slice(start, start + 900);
  assert.match(body, /register_all_app_shortcuts\(&app\)/,
    `${command} must reconcile global shortcuts immediately`);
}

assert.match(suggestion, /TransformDefinition/, 'Smart Actions must consume canonical Transform definitions');
assert.match(suggestion, /transform_ref:\s*transform\.stable_ref\.clone\(\)/,
  'Smart Actions must return stable Transform references');
assert.match(suggestionExecution, /get_transform_definitions/,
  'Smart Actions must resolve suggestions from the canonical Transform facade');
assert.match(clipPreview, /smartActions\.result\.actions\.map/,
  'Clip Preview must render shared Smart Action suggestions');

assert.match(sound, /setEnabled\(enabled: boolean\)/, 'Interaction sound state must have one global authority');
assert.match(app, /soundManager\.setEnabled\(appSettings\.enableSounds\)/,
  'The hydrated setting must configure the global sound authority');
assert.doesNotMatch(read('src/components/ClipPreview.tsx'), /play(?:Copy|Paste|Stack)Sound\(true\)/,
  'Clip Preview must not bypass Interaction Sounds');

assert.doesNotMatch(analytics, /Storage Compressed|['"]VS Code['"]|(?:bg|text|border)-blue-/,
  'Analytics must not present estimated compression, invented defaults, or palette-specific styling');
assert.doesNotMatch(database, /kb_saved/, 'The database must not expose fabricated storage-compression metrics');

console.log('Transformations and neglected-surface audit passed.');
