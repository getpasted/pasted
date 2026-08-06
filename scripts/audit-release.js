import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const packageJson = readJson('package.json');
const packageLock = readJson('package-lock.json');
const tauriConfig = readJson('src-tauri/tauri.conf.json');
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const rootLockPackage = packageLock.packages?.[''];

assert.equal(packageJson.name, 'pasted', 'Frontend package must use the Pasted product name');
assert.equal(packageLock.name, packageJson.name, 'Package lock name must match package.json');
assert.equal(rootLockPackage?.name, packageJson.name, 'Locked root package name must match package.json');
assert.equal(packageLock.version, packageJson.version, 'Package lock version must match package.json');
assert.equal(rootLockPackage?.version, packageJson.version, 'Locked root package version must match package.json');
assert.equal(tauriConfig.productName, 'Pasted', 'Native product name must remain Pasted');
assert.equal(tauriConfig.version, packageJson.version, 'Tauri and frontend versions must match');
assert.equal(cargoVersion, packageJson.version, 'Rust crate and frontend versions must match');
assert.match(
  tauriConfig.identifier,
  /^[a-zA-Z][a-zA-Z0-9-]*(?:\.[a-zA-Z0-9-]+){2,}$/,
  'Bundle identifier must be a stable reverse-domain identifier',
);
assert.equal(tauriConfig.bundle?.active, true, 'Release bundling must remain enabled');
assert.ok(tauriConfig.bundle?.icon?.length > 0, 'Release bundles must include app icons');

console.log(`Release metadata audit passed for Pasted ${packageJson.version}.`);
