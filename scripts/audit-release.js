import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const packageJson = readJson('package.json');
const packageLock = readJson('package-lock.json');
const tauriConfig = readJson('src-tauri/tauri.conf.json');
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const installationDiagnostics = fs.readFileSync('src-tauri/src/installation_diagnostics.rs', 'utf8');
const appSettingsHook = fs.readFileSync('src/hooks/useAppSettings.ts', 'utf8');
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const diagnosticsIdentifier = installationDiagnostics.match(/APP_IDENTIFIER:\s*&str\s*=\s*"([^"]+)"/)?.[1];
const rootLockPackage = packageLock.packages?.[''];
const packageScripts = packageJson.scripts ?? {};
const gitignore = fs.readFileSync('.gitignore', 'utf8');
const releaseWorkflow = fs.readFileSync('.github/workflows/desktop-release.yml', 'utf8');
const desktopBuildWorkflow = fs.readFileSync('.github/workflows/desktop-builds.yml', 'utf8');
const universalMacCliBuild = fs.readFileSync('scripts/build-macos-universal-cli.sh', 'utf8');

assert.equal(packageJson.name, 'pasted', 'Frontend package must use the Pasted product name');
assert.equal(packageLock.name, packageJson.name, 'Package lock name must match package.json');
assert.equal(rootLockPackage?.name, packageJson.name, 'Locked root package name must match package.json');
assert.equal(packageLock.version, packageJson.version, 'Package lock version must match package.json');
assert.equal(rootLockPackage?.version, packageJson.version, 'Locked root package version must match package.json');
assert.equal(tauriConfig.productName, 'Pasted', 'Native product name must remain Pasted');
assert.equal(
  tauriConfig.mainBinaryName,
  'pasted-app',
  'The private GUI executable must not collide with the public pasted CLI command',
);
assert.equal(tauriConfig.version, packageJson.version, 'Tauri and frontend versions must match');
assert.equal(cargoVersion, packageJson.version, 'Rust crate and frontend versions must match');
assert.equal(
  tauriConfig.identifier,
  'software.jjj.pasted',
  'The public bundle identifier must remain under the developer-owned jjj.software namespace',
);
assert.equal(
  diagnosticsIdentifier,
  tauriConfig.identifier,
  'Installation diagnostics and CLI data discovery must use the public bundle identifier',
);
assert.match(
  tauriConfig.identifier,
  /^[a-zA-Z][a-zA-Z0-9-]*(?:\.[a-zA-Z0-9-]+){2,}$/,
  'Bundle identifier must be a stable reverse-domain identifier',
);
assert.equal(tauriConfig.bundle?.active, true, 'Release bundling must remain enabled');
assert.ok(tauriConfig.bundle?.icon?.length > 0, 'Release bundles must include app icons');
assert.equal(
  tauriConfig.bundle?.macOS?.dmg?.background,
  'dmg/background.png',
  'The macOS installer must retain its branded DMG background',
);
assert.equal(
  fs.existsSync('src-tauri/dmg/background.png'),
  true,
  'The branded DMG background must exist',
);
assert.deepEqual(
  tauriConfig.bundle?.macOS?.dmg?.windowSize,
  { width: 660, height: 400 },
  'The DMG canvas must remain aligned with its branded artwork',
);
for (const [workflowName, workflow] of [
  ['desktop release', releaseWorkflow],
  ['desktop build', desktopBuildWorkflow],
]) {
  assert.match(
    workflow,
    /TAURI_BUNDLER_DMG_IGNORE_CI:\s*true/,
    `The ${workflowName} workflow must preserve branded Finder metadata in CI-built DMGs`,
  );
}
assert.match(
  appSettingsHook,
  /dockMenubarIcon:\s*'both'/,
  'Fresh installations must expose the native app menu and Dock/taskbar presence by default',
);
assert.match(cargoToml, /default-run\s*=\s*"pasted-app"/, 'Cargo must run the private GUI binary by default');
assert.match(cargoToml, /name\s*=\s*"pasted-app"\s*\npath\s*=\s*"src\/main\.rs"/, 'Cargo must build the GUI as pasted-app');
assert.match(cargoToml, /name\s*=\s*"pasted"\s*\npath\s*=\s*"src\/bin\/pasted_cli\.rs"/, 'Cargo must build the public CLI as pasted');

for (const scriptName of ['release:macos:local', 'release:macos', 'release:macos:verify']) {
  assert.equal(typeof packageScripts[scriptName], 'string', `Missing ${scriptName} release script`);
}

for (const path of [
  'scripts/release-macos.sh',
  'scripts/build-macos-universal-cli.sh',
  'scripts/render-dmg-background.sh',
  'scripts/verify-macos-release.sh',
  'docs/MACOS_RELEASE.md',
  'docs/RELEASE_AUTOMATION.md',
  '.github/workflows/desktop-builds.yml',
  '.github/workflows/desktop-release.yml',
  'scripts/render-homebrew-cask.js',
  'docs/HOMEBREW.md',
]) {
  assert.equal(fs.existsSync(path), true, `Missing release asset: ${path}`);
}

for (const ignoredCredential of [
  '*.p12',
  '*.pfx',
  '*.mobileprovision',
  '*.key',
  'AuthKey_*.p8',
  'src-tauri/tauri.windows.signing.conf.json',
]) {
  assert.equal(
    gitignore.split(/\r?\n/).includes(ignoredCredential),
    true,
    `Signing credential must remain ignored: ${ignoredCredential}`,
  );
}

assert.match(
  releaseWorkflow,
  /Pasted_\$\{RELEASE_VERSION\}_universal\.dmg/,
  'The release workflow must publish a deterministic universal DMG for Homebrew',
);
assert.match(
  releaseWorkflow,
  /bash scripts\/build-macos-universal-cli\.sh[\s\S]*tauri -- build --target universal-apple-darwin/,
  'The hosted macOS release must stage a universal CLI before Tauri bundles the universal app',
);
assert.match(
  releaseWorkflow,
  /codesign[\s\S]*--sign "\$APPLE_SIGNING_IDENTITY"[\s\S]*universal-apple-darwin\/release\/pasted[\s\S]*tauri -- build --target universal-apple-darwin/,
  'The hosted macOS release must sign the bundled CLI before Tauri signs the enclosing app',
);
assert.match(
  releaseWorkflow,
  /notarytool submit "\$dmg_path"[\s\S]*stapler staple "\$dmg_path"[\s\S]*stapler validate "\$dmg_path"/,
  'The hosted macOS release must notarize and staple the outer DMG after Tauri notarizes the app',
);
assert.match(
  universalMacCliBuild,
  /lipo "\$output_path" -verify_arch arm64 x86_64/,
  'The universal CLI audit must use Apple architecture names',
);
assert.match(
  releaseWorkflow,
  /render-homebrew-cask\.js/,
  'The release workflow must publish its matching Homebrew Cask',
);
assert.match(
  releaseWorkflow,
  /windows:[\s\S]*runs-on: windows-latest[\s\S]*tauri -- build --bundles nsis/,
  'The tagged release must build its experimental Windows NSIS installer on Windows',
);
assert.match(
  releaseWorkflow,
  /pasted-windows-x86_64\.exe[\s\S]*SHA256SUMS-windows-x86_64\.txt/,
  'The tagged release must include a portable Windows executable and checksum manifest',
);
assert.match(
  releaseWorkflow,
  /needs: \[metadata, macos, linux, windows\]/,
  'GitHub Release assembly must wait for every published desktop platform',
);
for (const appleSecret of [
  'APPLE_CERTIFICATE',
  'APPLE_CERTIFICATE_PASSWORD',
  'KEYCHAIN_PASSWORD',
  'APPLE_ID',
  'APPLE_PASSWORD',
  'APPLE_TEAM_ID',
]) {
  assert.match(
    releaseWorkflow,
    new RegExp(`secrets\\.${appleSecret}`),
    `The hosted macOS release must receive ${appleSecret} through an environment secret`,
  );
}
assert.doesNotMatch(
  releaseWorkflow,
  /APPLE_API_PRIVATE_KEY/,
  'The 1.0 release must not depend on App Store Connect business/API setup',
);

console.log(`Release metadata audit passed for Pasted ${packageJson.version}.`);
