import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const OUTPUT_PATH = 'THIRD_PARTY_LICENSES.json';
const OUTPUT_TEXT_PATH = 'THIRD_PARTY_NOTICES.txt';
const CARGO_ABOUT_VERSION = '0.9.1';
const checkOnly = process.argv.includes('--check');

function sha256(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function fileHash(filePath) {
  return sha256(fs.readFileSync(filePath));
}

function repositoryUrl(packageJson) {
  const repository = packageJson.repository;
  const value = typeof repository === 'string' ? repository : repository?.url;
  return (value ?? packageJson.homepage ?? '')
    .replace(/^git\+/, '')
    .replace(/\.git$/, '');
}

function cargoAboutCommand() {
  const configured = process.env.CARGO_ABOUT;
  if (configured) return { command: configured, prefix: [] };
  if (spawnSync('cargo-about', ['--version'], { stdio: 'ignore' }).status === 0) {
    return { command: 'cargo-about', prefix: [] };
  }
  if (spawnSync('cargo', ['about', '--version'], { stdio: 'ignore' }).status === 0) {
    return { command: 'cargo', prefix: ['about'] };
  }
  throw new Error(
    `cargo-about ${CARGO_ABOUT_VERSION} is required. Install it with: `
    + `cargo install --locked --features cli --version ${CARGO_ABOUT_VERSION} cargo-about`,
  );
}

function generateRustInventory() {
  const tool = cargoAboutCommand();
  const version = spawnSync(tool.command, [...tool.prefix, '--version'], { encoding: 'utf8' });
  assert.equal(version.status, 0, version.stderr);
  assert.match(version.stdout, new RegExp(`cargo-about ${CARGO_ABOUT_VERSION.replaceAll('.', '\\.')}`));

  const result = spawnSync(tool.command, [
    ...tool.prefix,
    'generate',
    '--config', 'about.toml',
    '--manifest-path', 'src-tauri/Cargo.toml',
    '--locked',
    '--fail',
    '--format', 'json',
  ], { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  return JSON.parse(result.stdout);
}

function addNotice(notices, text, label, spdx) {
  const normalized = text.trim().replaceAll('\r\n', '\n') + '\n';
  const id = sha256(normalized).slice(0, 16);
  const existing = notices.get(id);
  if (existing) {
    assert.equal(existing.text, normalized, `Notice hash collision for ${label}`);
    if (!existing.labels.includes(label)) existing.labels.push(label);
    if (spdx && !existing.spdx.includes(spdx)) existing.spdx.push(spdx);
  } else {
    notices.set(id, { id, labels: [label], spdx: spdx ? [spdx] : [], text: normalized });
  }
  return id;
}

function buildInventory(rawCargo) {
  const policy = JSON.parse(fs.readFileSync('dependency-policy.json', 'utf8'));
  const notices = new Map();
  const rustNoticeIds = new Map();

  for (const license of rawCargo.licenses) {
    const noticeId = addNotice(notices, license.text, license.name, license.id);
    for (const usage of license.used_by) {
      const key = `${usage.crate.name}@${usage.crate.version}`;
      const ids = rustNoticeIds.get(key) ?? new Set();
      ids.add(noticeId);
      rustNoticeIds.set(key, ids);
    }
  }

  const components = rawCargo.crates
    .filter(({ package: crate }) => crate.name !== 'pasted')
    .map(({ package: crate, license }) => {
      const key = `${crate.name}@${crate.version}`;
      const noticeIds = new Set(rustNoticeIds.get(key) ?? []);
      const crateDirectory = path.dirname(crate.manifest_path);
      const supplementalFiles = fs.readdirSync(crateDirectory, { withFileTypes: true })
        .filter((entry) => entry.isFile() && /^(notice|copyright)([._-].*)?$/i.test(entry.name))
        .map((entry) => entry.name)
        .sort((left, right) => left.localeCompare(right));
      for (const fileName of supplementalFiles) {
        noticeIds.add(addNotice(
          notices,
          fs.readFileSync(path.join(crateDirectory, fileName), 'utf8'),
          `${crate.name} — ${fileName}`,
          '',
        ));
      }
      assert.ok(noticeIds.size > 0, `No license notice resolved for Cargo component ${key}`);
      return {
        ecosystem: 'cargo',
        name: crate.name,
        version: crate.version,
        license,
        repository: crate.repository ?? '',
        noticeIds: [...noticeIds].sort(),
      };
    });

  const packageLock = JSON.parse(fs.readFileSync('package-lock.json', 'utf8'));
  for (const [packagePath, locked] of Object.entries(packageLock.packages ?? {})) {
    if (!packagePath || locked.dev) continue;
    const manifestPath = path.join(packagePath, 'package.json');
    const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    const licenseFiles = fs.readdirSync(packagePath, { withFileTypes: true })
      .filter((entry) => entry.isFile() && /^(licen[cs]e|copying|notice|copyright)([._-].*)?$/i.test(entry.name))
      .map((entry) => entry.name)
      .sort((left, right) => left.localeCompare(right));
    assert.ok(licenseFiles.length > 0, `No license file found for npm component ${manifest.name}`);
    const noticeIds = licenseFiles.map((fileName) => addNotice(
      notices,
      fs.readFileSync(path.join(packagePath, fileName), 'utf8'),
      `${manifest.name} — ${fileName}`,
      locked.license ?? manifest.license ?? '',
    ));
    components.push({
      ecosystem: 'npm',
      name: manifest.name,
      version: locked.version,
      license: locked.license ?? manifest.license ?? 'UNKNOWN',
      repository: repositoryUrl(manifest),
      noticeIds: [...new Set(noticeIds)].sort(),
    });
  }

  for (const packaging of policy.packagingComponents) {
    const noticeSource = components.find(({ name }) => name === packaging.noticeSourceComponent);
    assert.ok(noticeSource, `No notice source found for packaging component ${packaging.name}`);
    assert.ok(
      noticeSource.license.includes(packaging.license),
      `Notice source ${noticeSource.name} does not cover ${packaging.license}`,
    );
    components.push({
      ecosystem: 'packaging',
      name: packaging.name,
      version: packaging.version,
      license: packaging.license,
      repository: packaging.repository,
      noticeIds: noticeSource.noticeIds,
    });
  }

  components.sort((left, right) => (
    left.ecosystem.localeCompare(right.ecosystem)
    || left.name.localeCompare(right.name)
    || left.version.localeCompare(right.version)
  ));
  const sortedNotices = [...notices.values()]
    .map((notice) => ({
      ...notice,
      labels: notice.labels.sort((left, right) => left.localeCompare(right)),
      spdx: notice.spdx.sort((left, right) => left.localeCompare(right)),
    }))
    .sort((left, right) => left.id.localeCompare(right.id));

  const usersByNotice = new Map(sortedNotices.map((notice) => [notice.id, []]));
  for (const component of components) {
    for (const noticeId of component.noticeIds) {
      usersByNotice.get(noticeId).push(`${component.name} ${component.version} (${component.ecosystem})`);
    }
  }

  const inventoryLines = components.map((component) => {
    const repository = component.repository ? ` — ${component.repository}` : '';
    return `- ${component.name} ${component.version} [${component.ecosystem}; ${component.license}]${repository}`;
  });
  const noticeSections = sortedNotices.map((notice) => [
    '================================================================================',
    notice.labels.join(' / '),
    notice.spdx.length > 0 ? `License identifier: ${notice.spdx.join(', ')}` : '',
    `Used by: ${usersByNotice.get(notice.id).join(', ')}`,
    '--------------------------------------------------------------------------------',
    notice.text.trim(),
  ].filter(Boolean).join('\n'));

  const noticeText = [
    'Pasted Third-Party Software Notices',
    '',
    'Pasted is built with open-source software. The following notices are provided',
    'to preserve the licenses and attributions of components distributed with it.',
    '',
    'Source availability',
    '',
    'Source for Rust crates, including MPL-2.0-covered components, is available from',
    'the repository links below and from the matching version archives on crates.io.',
    'Pasted does not modify those dependency source files.',
    '',
    'Platform note',
    '',
    'Operating-system frameworks are supplied by the operating system. Linux AppImage',
    'runtime notices are embedded by the AppImage runtime and must remain intact.',
    'Windows installer-tool components and their notices are included in this inventory.',
    '',
    `Component inventory (${components.length})`,
    '',
    ...inventoryLines,
    '',
    `License and attribution notices (${sortedNotices.length})`,
    '',
    ...noticeSections,
    '',
  ].join('\n');

  return { components, notices: sortedNotices, noticeText };
}

const sourceHashes = {
  aboutConfig: fileHash('about.toml'),
  cargoLock: fileHash('src-tauri/Cargo.lock'),
  dependencyPolicy: fileHash('dependency-policy.json'),
  generatorScript: fileHash('scripts/generate-third-party-licenses.js'),
  packageLock: fileHash('package-lock.json'),
};

if (checkOnly) {
  const generated = JSON.parse(fs.readFileSync(OUTPUT_PATH, 'utf8'));
  assert.equal(generated.schemaVersion, 1, 'Unsupported third-party license schema');
  assert.equal(generated.generator.cargoAbout, CARGO_ABOUT_VERSION, 'cargo-about version changed');
  assert.deepEqual(generated.sourceHashes, sourceHashes, 'Third-party notices are stale; run npm run licenses:generate');
  assert.equal(generated.componentCount, generated.components.length, 'Component count is inconsistent');
  assert.ok(generated.noticeText.includes(`Component inventory (${generated.componentCount})`));
  assert.equal(
    fs.readFileSync(OUTPUT_TEXT_PATH, 'utf8'),
    generated.noticeText,
    'Plain-text third-party notices are stale; run npm run licenses:generate',
  );
  console.log(`Third-party notice audit passed for ${generated.componentCount} components.`);
  process.exit(0);
}

const inventory = buildInventory(generateRustInventory());
const generated = {
  schemaVersion: 1,
  generator: { cargoAbout: CARGO_ABOUT_VERSION },
  sourceHashes,
  componentCount: inventory.components.length,
  components: inventory.components,
  notices: inventory.notices,
  noticeText: inventory.noticeText,
};
fs.writeFileSync(OUTPUT_PATH, `${JSON.stringify(generated, null, 2)}\n`);
fs.writeFileSync(OUTPUT_TEXT_PATH, generated.noticeText);
console.log(`Generated ${OUTPUT_PATH} and ${OUTPUT_TEXT_PATH} for ${generated.componentCount} components.`);
