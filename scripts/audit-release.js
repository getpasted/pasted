import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const packageJson = readJson('package.json');
const packageLock = readJson('package-lock.json');
const tauriConfig = readJson('src-tauri/tauri.conf.json');
const tauriCliSidecarConfig = readJson('src-tauri/tauri.cli-sidecar.conf.json');
const tauriReleaseConfig = readJson('src-tauri/tauri.release.conf.json');
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const cargoBuildScript = fs.readFileSync('src-tauri/build.rs', 'utf8');
const installationDiagnostics = fs.readFileSync('src-tauri/src/installation_diagnostics.rs', 'utf8');
const settingsContract = readJson('shared/settings-contract.json');
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const cargoPackageName = cargoToml.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
const diagnosticsIdentifier = installationDiagnostics.match(/APP_IDENTIFIER:\s*&str\s*=\s*"([^"]+)"/)?.[1];
const rootLockPackage = packageLock.packages?.[''];
const packageScripts = packageJson.scripts ?? {};
const gitignore = fs.readFileSync('.gitignore', 'utf8');
const nodeVersion = fs.readFileSync('.node-version', 'utf8').trim();
const workflowPaths = fs
  .readdirSync('.github/workflows')
  .filter((name) => name.endsWith('.yml') || name.endsWith('.yaml'))
  .map((name) => `.github/workflows/${name}`);
const nodeWorkflowEntries = workflowPaths
  .map((path) => ({ path, source: fs.readFileSync(path, 'utf8') }))
  .filter(({ source }) => source.includes('actions/setup-node@'));
const workflowEntries = workflowPaths.map((path) => ({ path, source: fs.readFileSync(path, 'utf8') }));
const movingActionExceptions = new Set(['dtolnay/rust-toolchain@stable']);
const linuxDockerfile = fs.readFileSync('packaging/linux/Dockerfile', 'utf8');
const dependabotConfig = fs.readFileSync('.github/dependabot.yml', 'utf8');
const releaseWorkflow = fs.readFileSync('.github/workflows/desktop-release.yml', 'utf8');
const updaterFeedWorkflow = fs.readFileSync('.github/workflows/updater-feed.yml', 'utf8');
const desktopBuildWorkflow = fs.readFileSync('.github/workflows/desktop-builds.yml', 'utf8');
const macosPackageJob = desktopBuildWorkflow.match(
  /\n  package-macos:\n[\s\S]*?(?=\n  package-macos-artifact:)/,
)?.[0];
const desktopPackageJob = desktopBuildWorkflow.match(
  /\n  package:\n[\s\S]*?(?=\n  package-macos:)/,
)?.[0];
const dependencyPolicyWorkflow = fs.readFileSync('.github/workflows/dependency-policy.yml', 'utf8');
const universalMacCliBuild = fs.readFileSync('scripts/build-macos-universal-cli.sh', 'utf8');
const linuxReleaseScript = fs.readFileSync('scripts/release-linux-appimage.sh', 'utf8');
const thirdPartyLicenses = readJson('THIRD_PARTY_LICENSES.json');
const thirdPartyNotices = fs.readFileSync('THIRD_PARTY_NOTICES.txt', 'utf8');
const sourceSbom = readJson('THIRD_PARTY_SBOM.spdx.json');

assert.equal(nodeVersion, '24', 'Repository automation must use the current Node.js LTS major');
assert.ok(nodeWorkflowEntries.length > 0, 'At least one workflow must configure Node.js explicitly');
for (const { path, source } of nodeWorkflowEntries) {
  assert.match(
    source,
    /node-version-file:\s*\.node-version/,
    `${path} must read the shared Node.js version file`,
  );
  assert.doesNotMatch(
    source,
    /node-version:\s*["']?\d+/,
    `${path} must not maintain an independent Node.js version`,
  );
}
for (const { path, source } of workflowEntries) {
  for (const match of source.matchAll(/^\s*(?:-\s*)?uses:\s*([^\s#]+)/gm)) {
    const actionReference = match[1];
    if (actionReference.startsWith('./') || actionReference.startsWith('docker://')) continue;
    const separator = actionReference.lastIndexOf('@');
    const version = actionReference.slice(separator + 1);
    assert.ok(separator > 0, `${path} contains an unversioned action reference: ${actionReference}`);
    assert.ok(
      /^[0-9a-f]{40}$/.test(version)
        || /^v?\d+\.\d+(?:\.\d+)?$/.test(version)
        || movingActionExceptions.has(actionReference),
      `${path} must use an exact Action release, immutable commit, or reviewed moving-channel exception: ${actionReference}`,
    );
  }
}
for (const exception of movingActionExceptions) {
  assert.ok(
    workflowEntries.some(({ source }) => source.includes(`uses: ${exception}`)),
    `Reviewed moving-channel exception is stale: ${exception}`,
  );
}
assert.match(
  linuxDockerfile,
  new RegExp(`^FROM node:${nodeVersion}-bookworm@sha256:[a-f0-9]{64}$`, 'm'),
  'The Linux packaging image must match the shared Node.js LTS major and pin its digest',
);
assert.match(
  dependabotConfig,
  /package-ecosystem:\s*docker[\s\S]*?directory:\s*\/packaging\/linux/,
  'Dependabot must monitor the Linux packaging base image',
);
assert.match(
  dependabotConfig,
  /package-ecosystem:\s*docker[\s\S]*?dependency-name:\s*node[\s\S]*?version-update:semver-major/,
  'Dependabot must not replace the reviewed Node.js LTS major automatically',
);

assert.match(
  desktopBuildWorkflow,
  /pull_request:\s*\n\s*types:\s*\[[^\]]*ready_for_review[^\]]*\]/,
  'Desktop builds must run the complete PR matrix when a draft becomes ready',
);
assert.match(
  desktopBuildWorkflow,
  /allow-dependencies-licenses:\s*pkg:githubactions\/Swatinem\/rust-cache/,
  'The CI-only rust-cache license exception must stay scoped to that Action',
);
assert.match(
  desktopBuildWorkflow,
  /allow-dependencies-licenses:[^\n]*pkg:cargo\/webpki-root-certs/,
  'The target-specific root-certificate dataset license exception must stay scoped to that crate',
);
assert.doesNotMatch(
  desktopBuildWorkflow,
  /allow-licenses:[^\n]*CDLA-Permissive-2\.0/,
  'The unshipped root-certificate dataset must not broaden the general product license allowlist',
);
assert.doesNotMatch(
  desktopBuildWorkflow,
  /allow-licenses:[^\n]*LGPL/,
  'LGPL must not enter the general product dependency license allowlist',
);
assert.match(
  desktopBuildWorkflow,
  /validation-scope:[\s\S]*?src-tauri\/\*[\s\S]*?package\.json[\s\S]*?desktop-builds\.yml[\s\S]*?git diff --name-only/,
  'Desktop builds must detect native-impacting changes without trusting a client-supplied label',
);
assert.match(
  desktopBuildWorkflow,
  /validate-frontend:[\s\S]*?github\.event\.pull_request\.draft == false[\s\S]*?npm run test:frontend/,
  'Frontend validation must remain deferred while a pull request is a draft',
);
assert.match(
  desktopBuildWorkflow,
  /env:\s*\n\s*CARGO_PROFILE_DEV_DEBUG: "1"\s*\n\s*CARGO_PROFILE_TEST_DEBUG: "1"/,
  'Native CI must retain function-level diagnostics without caching full debug line tables',
);
assert.match(
  desktopBuildWorkflow,
  /validate-native:[\s\S]*?github\.event_name != 'pull_request'[\s\S]*?shared-key: native-debug1-linux[\s\S]*?timeout-minutes: 8[\s\S]*?npm run test:native/,
  'Main-branch native validation must warm the shared Linux cache and bound dependency setup time',
);
assert.match(
  desktopBuildWorkflow,
  /validate:\s*\n\s*name: Validate[\s\S]*?needs: \[validation-scope, validate-frontend, validate-native, smoke-macos, smoke-linux, smoke-windows\]/,
  'The protected Validate check must aggregate frontend validation and the applicable native path',
);
assert.match(
  desktopBuildWorkflow,
  /smoke-macos:\s*\n\s*name: Smoke macOS native[\s\S]*?needs\.validation-scope\.outputs\.native == 'true'[\s\S]*?shared-key: native-debug1-macos[\s\S]*?cargo test/,
  'The statically named macOS smoke check must skip before runner allocation for frontend-only PRs',
);
assert.match(
  desktopBuildWorkflow,
  /smoke-linux:\s*\n\s*name: Smoke Linux x86_64[\s\S]*?needs\.validation-scope\.outputs\.native == 'true'[\s\S]*?shared-key: native-debug1-linux[\s\S]*?timeout-minutes: 8[\s\S]*?npm run test:native/,
  'The Linux smoke check must reuse the default-branch native cache and run complete validation',
);
assert.match(
  desktopBuildWorkflow,
  /smoke-windows:\s*\n\s*name: Smoke Windows x86_64[\s\S]*?needs\.validation-scope\.outputs\.native == 'true'[\s\S]*?shared-key: native-debug1-windows[\s\S]*?cargo test/,
  'The statically named Windows smoke check must skip before runner allocation for frontend-only PRs',
);
assert.ok(desktopPackageJob, 'Main desktop packaging must retain its platform matrix job');
assert.match(
  desktopPackageJob,
  /name: Package \$\{\{ matrix\.name \}\}[\s\S]*?always\(\)[\s\S]*?needs\.validation-scope\.result == 'success'[\s\S]*?needs\.validate-frontend\.result == 'success'[\s\S]*?needs\.dependency-policy\.result == 'success'[\s\S]*?needs: \[validation-scope, validate-frontend, dependency-policy\]/,
  'Main platform packaging must start after fast gates so it can overlap native validation',
);
assert.doesNotMatch(
  desktopPackageJob,
  /needs\.validate\.result/,
  'Main platform packaging must not serialize behind the aggregate native validation job',
);
assert.match(
  desktopPackageJob,
  /CARGO_PROFILE_RELEASE_LTO: 'false'[\s\S]*?CARGO_PROFILE_RELEASE_CODEGEN_UNITS: '16'/,
  'Ephemeral Linux and Windows packages must avoid production-grade release linking',
);
assert.match(
  desktopPackageJob,
  /Build headless CLI[\s\S]*?--no-default-features --features cli --bin pasted[\s\S]*?stage:cli-sidecar[\s\S]*?tauri -- build/,
  'Linux and Windows packages must build and stage the headless CLI before Tauri bundles the app',
);
assert.equal(
  (desktopPackageJob.match(/--config src-tauri\/tauri\.cli-sidecar\.conf\.json/g) ?? []).length,
  2,
  'Ephemeral Linux and Windows packages must activate sidecar bundling explicitly',
);
assert.ok(macosPackageJob, 'Main macOS packaging must use one shared-runner job');
assert.match(
  macosPackageJob,
  /name: Package macOS universal CLI and DMG[\s\S]*?always\(\)[\s\S]*?needs\.validation-scope\.result == 'success'[\s\S]*?needs\.validate-frontend\.result == 'success'[\s\S]*?needs\.dependency-policy\.result == 'success'[\s\S]*?needs: \[validation-scope, validate-frontend, dependency-policy\]/,
  'Main macOS packaging must start after fast gates so it can overlap native validation',
);
assert.doesNotMatch(
  macosPackageJob,
  /needs\.validate\.result/,
  'Main macOS packaging must not serialize behind the aggregate native validation job',
);
assert.match(
  macosPackageJob,
  /CARGO_PROFILE_RELEASE_LTO: 'false'[\s\S]*?CARGO_PROFILE_RELEASE_CODEGEN_UNITS: '16'/,
  'Ephemeral macOS packages must avoid production-grade release linking',
);
assert.match(
  macosPackageJob,
  /shared-key: macos-universal-package[\s\S]*?build-macos-universal-cli\.sh[\s\S]*?tauri -- build --target universal-apple-darwin/,
  'The macOS CLI and app must reuse one release target tree before the DMG is bundled',
);
assert.doesNotMatch(
  macosPackageJob,
  /actions\/download-artifact/,
  'The macOS packaging job must retain its compiled CLI locally instead of restoring it on a fresh runner',
);
assert.match(
  desktopBuildWorkflow,
  /package-macos-artifact:\s*\n\s*name: Audit macOS packaged artifact[\s\S]*?always\(\)[\s\S]*?needs\.package-macos\.result == 'success'[\s\S]*?needs: \[package-macos\]/,
  'The macOS artifact audit must run after the consolidated package succeeds',
);

assert.equal(packageJson.name, 'pasted', 'Frontend package must use the Pasted product name');
assert.equal(packageLock.name, packageJson.name, 'Package lock name must match package.json');
assert.equal(rootLockPackage?.name, packageJson.name, 'Locked root package name must match package.json');
assert.equal(packageLock.version, packageJson.version, 'Package lock version must match package.json');
assert.equal(rootLockPackage?.version, packageJson.version, 'Locked root package version must match package.json');
assert.equal(tauriConfig.productName, 'Pasted', 'Native product name must remain Pasted');
assert.equal(cargoPackageName, 'pasted-app', 'The Cargo package name must select the private GUI executable for Tauri bundling');
assert.equal(tauriConfig.mainBinaryName, undefined, 'Tauri must derive its main binary from the Cargo package name');
assert.deepEqual(
  tauriCliSidecarConfig.bundle?.externalBin,
  ['binaries/pasted'],
  'Desktop installers must bundle the staged headless CLI',
);
assert.deepEqual(
  tauriReleaseConfig.bundle?.externalBin,
  ['binaries/pasted'],
  'Release installers must bundle the staged headless CLI',
);
assert.equal(
  tauriReleaseConfig.bundle?.createUpdaterArtifacts,
  true,
  'Release installers must emit signed updater artifacts',
);
assert.deepEqual(
  tauriReleaseConfig.plugins?.updater,
  {},
  'Release builds must retain the updater plugin configuration required for artifact signing',
);
assert.equal(
  packageScripts['stage:cli-sidecar'],
  'node scripts/stage-tauri-cli-sidecar.js',
  'Desktop packages must share one target-aware CLI sidecar staging command',
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
  tauriConfig.bundle?.resources?.['../THIRD_PARTY_NOTICES.txt'],
  'THIRD_PARTY_NOTICES.txt',
  'Desktop bundles must include the offline third-party notice file',
);
assert.equal(thirdPartyLicenses.schemaVersion, 1, 'Third-party license data must use the reviewed schema');
assert.equal(thirdPartyLicenses.componentCount, thirdPartyLicenses.components.length, 'Third-party component count must be consistent');
assert.equal(thirdPartyLicenses.noticeText, thirdPartyNotices, 'Structured and plain-text notices must match');
assert.equal(sourceSbom.spdxVersion, 'SPDX-2.3', 'Release source SBOM must use SPDX 2.3');
assert.equal(sourceSbom.packages.length, thirdPartyLicenses.componentCount, 'Source SBOM must cover every inventoried component');
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
assert.equal(
  settingsContract.settings.find(({ key }) => key === 'dockMenubarIcon')?.default,
  'both',
  'Fresh installations must expose the native app menu and Dock/taskbar presence by default',
);
assert.match(cargoToml, /default-run\s*=\s*"pasted-app"/, 'Cargo must run the private GUI binary by default');
assert.match(
  cargoToml,
  /autobins\s*=\s*false/,
  'Cargo binary auto-discovery must stay disabled so CLI support modules are not mistaken for bundle executables',
);
assert.match(cargoToml, /name\s*=\s*"pasted-app"\s*\npath\s*=\s*"src\/main\.rs"/, 'Cargo must build the GUI as pasted-app');
assert.match(
  cargoToml,
  /name\s*=\s*"pasted"\s*\npath\s*=\s*"src\/bin\/pasted\.rs"/,
  'The public CLI target name and entrypoint stem must both be pasted so Tauri bundles the built executable',
);
assert.match(
  cargoToml,
  /name\s*=\s*"pasted-app"[\s\S]*?required-features\s*=\s*\["gui"\][\s\S]*?name\s*=\s*"pasted"[\s\S]*?required-features\s*=\s*\["cli"\]/,
  'GUI and CLI binaries must require distinct Cargo features',
);
assert.match(cargoToml, /default\s*=\s*\["gui"\]/, 'Normal Cargo builds must retain the GUI by default');
for (const dependency of [
  'tauri',
  'tauri-build',
  'tauri-plugin-autostart',
  'tauri-plugin-dialog',
  'tauri-plugin-global-shortcut',
  'tauri-plugin-single-instance',
  'tauri-plugin-window-state',
  'window-vibrancy',
]) {
  assert.match(
    cargoToml,
    new RegExp(`^${dependency.replaceAll('-', '\\-')}\\s*=.*optional\\s*=\\s*true`, 'm'),
    `${dependency} must remain optional for headless CLI builds`,
  );
}
assert.match(
  cargoBuildScript,
  /#\[cfg\(feature = "gui"\)\]\s*tauri_build::build\(\)/,
  'The CLI build must not compile or run the Tauri build helper',
);
assert.match(
  packageScripts['test:cli-build'] ?? '',
  /^cargo build .*--locked --no-default-features --features cli --bin pasted$/,
  'Native validation must build the headless CLI executable before integration tests',
);
assert.deepEqual(
  fs.readdirSync('src-tauri/src/bin').sort(),
  ['pasted.rs'],
  'Only executable entrypoints may live under src/bin because Tauri treats support modules as bundle binaries',
);

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
assert.doesNotMatch(
  releaseWorkflow,
  /CARGO_PROFILE_RELEASE_(?:LTO|CODEGEN_UNITS)/,
  'Signed release builds must retain the production Cargo release profile',
);
assert.match(
  releaseWorkflow,
  /bash scripts\/build-macos-universal-cli\.sh[\s\S]*tauri -- build --target universal-apple-darwin/,
  'The hosted macOS release must stage a universal CLI before Tauri bundles the universal app',
);
assert.equal(
  (releaseWorkflow.match(/--no-default-features --features cli --bin pasted/g) ?? []).length,
  2,
  'Linux and Windows releases must build headless CLIs before their GUI packages',
);
assert.equal(
  (releaseWorkflow.match(/stage:cli-sidecar/g) ?? []).length,
  3,
  'Every desktop release must stage its headless CLI into the installer',
);
assert.equal(
  (releaseWorkflow.match(/--config src-tauri\/tauri\.release\.conf\.json/g) ?? []).length,
  3,
  'Every desktop release must activate CLI sidecar and updater bundling',
);
assert.match(
  releaseWorkflow,
  /codesign[\s\S]*universal-apple-darwin\/release\/pasted[\s\S]*stage:cli-sidecar:macos-universal[\s\S]*tauri -- build --target universal-apple-darwin --bundles dmg --config src-tauri\/tauri\.release\.conf\.json/,
  'The hosted macOS release must stage the signed universal CLI before bundling the app',
);
assert.equal(
  (releaseWorkflow.match(/TAURI_SIGNING_PRIVATE_KEY: \$\{\{ secrets\.TAURI_SIGNING_PRIVATE_KEY \}\}/g) ?? []).length,
  3,
  'Every desktop release must receive the updater signing key',
);
assert.equal(
  (releaseWorkflow.match(/PASTED_UPDATER_PUBLIC_KEY: \$\{\{ secrets\.PASTED_UPDATER_PUBLIC_KEY \}\}/g) ?? []).length,
  3,
  'Every desktop release must embed the matching updater public key',
);
assert.match(
  releaseWorkflow,
  /\.app\.tar\.gz[\s\S]*\.AppImage\.sig[\s\S]*-setup\.exe\.sig/,
  'The draft release must include signed updater payloads for every desktop platform',
);
assert.match(
  updaterFeedWorkflow,
  /release:\s*\n\s*types:\s*\[published\][\s\S]*updater-prerelease[\s\S]*updater-stable/,
  'Published releases must refresh prerelease and stable updater feeds',
);
assert.match(
  macosPackageJob,
  /stage:cli-sidecar:macos-universal[\s\S]*tauri -- build --target universal-apple-darwin --bundles dmg --config src-tauri\/tauri\.cli-sidecar\.conf\.json/,
  'The post-merge macOS package must exercise the bundled universal CLI path',
);
assert.match(
  fs.readFileSync('scripts/release-macos.sh', 'utf8'),
  /tauri_config="src-tauri\/tauri\.cli-sidecar\.conf\.json"[\s\S]*tauri_config="src-tauri\/tauri\.release\.conf\.json"[\s\S]*stage:cli-sidecar[\s\S]*tauri -- build --bundles dmg --config "\$tauri_config"/,
  'Local macOS rehearsals must bundle the CLI, while distributable builds also emit signed updates',
);
const macosVerifier = fs.readFileSync('scripts/verify-macos-release.sh', 'utf8');
assert.match(
  macosVerifier,
  /Contents\/MacOS\/pasted[\s\S]*-verify_arch[\s\S]*codesign --verify --strict/,
  'macOS release verification must require an architecture-matched signed bundled CLI',
);
assert.match(
  linuxReleaseScript,
  /--no-default-features[\s\S]*--features cli[\s\S]*--bin pasted[\s\S]*stage:cli-sidecar[\s\S]*tauri build[\s\S]*tauri\.cli-sidecar\.conf\.json/,
  'The local Linux release must build and bundle the headless CLI explicitly',
);
assert.match(
  macosPackageJob,
  /build-macos-universal-cli\.sh[\s\S]*name: Pasted-macOS-universal-CLI[\s\S]*tauri -- build --target universal-apple-darwin[\s\S]*name: Pasted-macOS-universal-DMG/,
  'The post-merge macOS job must preserve both artifacts while reusing compilation between them',
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
  /--no-default-features\s*\\\s*\n\s*--features cli\s*\\\s*\n\s*--bin pasted/,
  'Universal macOS CLI builds must exclude GUI dependencies',
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
assert.equal(
  (releaseWorkflow.match(/cp THIRD_PARTY_NOTICES\.txt release-assets\//g) ?? []).length,
  2,
  'macOS and Linux portable releases must stage the notice beside the CLI',
);
assert.match(
  releaseWorkflow,
  /Copy-Item THIRD_PARTY_NOTICES\.txt release-assets\//,
  'The Windows portable release must stage the notice beside the CLI',
);
assert.match(
  releaseWorkflow,
  /THIRD_PARTY_SBOM\.spdx\.json.*Pasted_\$\{RELEASE_VERSION\}_source\.spdx\.json/,
  'The release must publish the deterministic dependency-graph SBOM',
);
assert.equal(
  (releaseWorkflow.match(/anchore\/sbom-action@e22c389904149dbc22b58101806040fa8d37a610/g) ?? []).length,
  3,
  'Every release platform must generate an exact-artifact SBOM with the reviewed action revision',
);
assert.match(
  desktopBuildWorkflow,
  /actions\/dependency-review-action@a1d282b36b6f3519aa1f3fc636f609c47dddb294/,
  'Pull requests must review new dependency licenses and vulnerabilities',
);
for (const workflow of [desktopBuildWorkflow, releaseWorkflow]) {
  assert.match(
    workflow,
    /EmbarkStudios\/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25/,
    'Build and release workflows must enforce the reviewed Rust dependency policy',
  );
  assert.match(workflow, /audit-artifact-sbom\.js/, 'Packaged payloads must pass artifact SBOM policy');
}
assert.match(dependencyPolicyWorkflow, /schedule:/, 'Dependency policy must run without a source change');
assert.match(dependencyPolicyWorkflow, /npm run dependencies:check/, 'Scheduled policy must enforce mission and expiry rules');
assert.match(
  dependencyPolicyWorkflow,
  /EmbarkStudios\/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25/,
  'Scheduled policy must refresh RustSec and Rust dependency findings',
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
