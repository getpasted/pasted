import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const readFilesRecursively = (directory, extensions) => fs.readdirSync(directory, { withFileTypes: true })
  .flatMap((entry) => {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) return readFilesRecursively(path, extensions);
    return extensions.some((extension) => entry.name.endsWith(extension))
      ? [fs.readFileSync(path, 'utf8')]
      : [];
  });

const tauriConfig = readJson('src-tauri/tauri.conf.json');
const capability = readJson('src-tauri/capabilities/default.json');
const packageJson = readJson('package.json');
const frontendSource = readFilesRecursively('src', ['.ts', '.tsx']).join('\n');
const rustSource = readFilesRecursively('src-tauri/src', ['.rs']).join('\n');
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const clipActions = fs.readFileSync('src/hooks/useClipActions.ts', 'utf8');
const plainText = fs.readFileSync('src/utils/plainText.ts', 'utf8');
const safeRasterImage = fs.readFileSync('src/components/SafeRasterImage.tsx', 'utf8');
const codeqlWorkflow = fs.readFileSync('.github/workflows/codeql.yml', 'utf8');
const safeRasterConsumers = [
  'src/components/CaptureFeedbackWindow.tsx',
  'src/components/ClipCardThumbnails.tsx',
  'src/components/ClipPreviewContent.tsx',
  'src/components/QuickHudWindow.tsx',
  'src/components/Sidebar.tsx',
].map((path) => fs.readFileSync(path, 'utf8'));
const security = tauriConfig.app?.security;

assert.match(codeqlWorkflow, /schedule:\s*\n\s*- cron:/, 'CodeQL must retain a scheduled full scan');
assert.match(
  codeqlWorkflow,
  /\.github\/workflows\/codeql\.yml\|\.github\/codeql\/\*[\s\S]*?actions=true[\s\S]*?javascript=true[\s\S]*?rust=true/,
  'CodeQL configuration changes must exercise every language analyzer',
);
for (const language of ['actions', 'javascript-typescript', 'rust']) {
  assert.match(
    codeqlWorkflow,
    new RegExp(`name: Analyze \\(${language}\\)[\\s\\S]*?languages: ${language}`),
    `CodeQL must retain the ${language} analyzer`,
  );
}
assert.match(codeqlWorkflow, /languages: rust\s*\n\s*build-mode: none/, 'Rust CodeQL must use its supported buildless mode');
assert.match(
  codeqlWorkflow,
  /codeql:\s*\n\s*name: CodeQL[\s\S]*?needs: \[scope, analyze-actions, analyze-javascript, analyze-rust\]/,
  'The CodeQL summary check must aggregate every scoped analyzer',
);

assert.ok(security?.csp, 'Production Tauri CSP must remain enabled');
assert.equal(security.freezePrototype, true, 'Tauri must freeze Object.prototype in packaged webviews');
assert.match(security.csp['default-src'], /'self'/, 'CSP default-src must be self-restricted');
assert.match(security.csp['connect-src'], /ipc:/, 'CSP must permit Tauri IPC');
assert.equal(security.csp['object-src'], "'none'", 'CSP must block embedded objects');
assert.equal(security.csp['base-uri'], "'none'", 'CSP must block base URL rewriting');
assert.equal(security.csp['frame-src'], "'none'", 'CSP must block framed content');

assert.ok(!capability.permissions.includes('opener:default'), 'Unused opener permission must not return');
assert.ok(!packageJson.dependencies?.['@tauri-apps/plugin-opener'], 'Unused opener dependency must not return');
assert.ok(
  !capability.permissions.some((permission) => permission.startsWith('shell:')),
  'The webview must not receive Tauri shell permissions',
);
assert.ok(!packageJson.dependencies?.['@tauri-apps/plugin-shell'], 'The frontend must not gain shell access');
assert.doesNotMatch(cargoToml, /tauri-plugin-shell/, 'The backend must not enable the Tauri shell plugin');
assert.match(
  rustSource,
  /SQLITE_DBCONFIG_DEFENSIVE/,
  'SQLite connections must retain defensive mode as a second layer behind bound parameters',
);
assert.match(rustSource, /MAX_CLIP_TEXT_BYTES/, 'Untrusted clipboard text must remain bounded');
assert.match(rustSource, /MAX_PROVIDER_WORKSPACE_BYTES/, 'Provider disk output must remain bounded');
assert.match(rustSource, /PROVIDER_EXECUTION_TIMEOUT_SECS/, 'Provider execution must retain a timeout');
assert.doesNotMatch(frontendSource, /dangerouslySetInnerHTML/, 'Render untrusted clip content as text, never raw HTML');
assert.match(safeRasterImage, /decodeSafeRasterDataUrl\(source\)/, 'Dynamic image sources must pass the shared raster decoder');
assert.match(safeRasterImage, /URL\.createObjectURL\(new Blob/, 'Validated raster bytes must render through an inert object URL');
assert.match(
  frontendSource,
  /decodedByteLength > MAX_RENDERABLE_RASTER_BYTES[\s\S]*atob\(payload\)/,
  'Dynamic raster sources must enforce the decoded-byte ceiling before allocating decoded data',
);
for (const consumer of safeRasterConsumers) {
  assert.match(consumer, /SafeRasterImage/, 'Every dynamic raster surface must use SafeRasterImage');
  assert.doesNotMatch(consumer, /<img\b/, 'Dynamic raster surfaces must not bypass SafeRasterImage');
}
assert.match(rustSource, /validate_raster_data_url/, 'Native clip and icon boundaries must validate raster data URLs');
assert.doesNotMatch(frontendSource, /\b(?:eval|Function)\s*\(/, 'Frontend dynamic code execution is forbidden');
assert.match(clipActions, /htmlToPlainText\(clip\.text_content\)/, 'Plain-text copying must use the shared HTML parser');
assert.match(plainText, /new DOMParser\(\)\.parseFromString\(value, 'text\/html'\)/, 'HTML-to-text conversion must use DOM parsing');
assert.match(plainText, /script, style, template, noscript/, 'HTML-to-text conversion must discard non-visible executable content');
assert.doesNotMatch(clipActions, /replace\(\/<\[\^>\]\*>\/g/, 'Do not restore one-pass regex HTML stripping');
assert.doesNotMatch(
  rustSource,
  /Command::new\(\s*"(?:\/[^"\s]+\/)?(?:ba|z|fi)?sh"\s*\)/,
  'Never restore a general-purpose shell interpreter to a transformation path',
);
assert.doesNotMatch(
  rustSource,
  /vaultPasscodeHash|set_vault_passcode|verify_vault_passcode/,
  'Do not expose the removed fast-hash passcode API without a reviewed credential design',
);

console.log('Security configuration, process, and frontend trust-boundary audit passed.');
