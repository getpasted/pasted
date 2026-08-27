import fs from 'node:fs';
import path from 'node:path';

function option(name, required = true) {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (required && !value) throw new Error(`Missing ${name}`);
  return value;
}

const version = option('--version').replace(/^v/, '');
const tag = option('--tag');
const assetRoot = path.resolve(option('--asset-root'));
const output = path.resolve(option('--output'));
const notesFile = option('--notes-file', false);
const publishedAt = option('--published-at', false) ?? new Date().toISOString();

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`Invalid release version: ${version}`);
}

const files = fs.readdirSync(assetRoot, { recursive: true })
  .map((entry) => path.join(assetRoot, String(entry)))
  .filter((entry) => fs.statSync(entry).isFile());

function one(description, predicate) {
  const matches = files.filter(predicate);
  if (matches.length !== 1) {
    throw new Error(`Expected one ${description}; found ${matches.map((entry) => path.basename(entry)).join(', ') || 'none'}`);
  }
  return matches[0];
}

function signedAsset(description, predicate) {
  const artifact = one(description, (entry) => predicate(path.basename(entry)));
  const signature = `${artifact}.sig`;
  if (!fs.existsSync(signature)) throw new Error(`Missing signature for ${path.basename(artifact)}`);
  const signatureContents = fs.readFileSync(signature, 'utf8').trim();
  if (!signatureContents) throw new Error(`Empty signature for ${path.basename(artifact)}`);
  const filename = path.basename(artifact);
  return {
    url: `https://github.com/getpasted/pasted/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(filename)}`,
    signature: signatureContents,
  };
}

const macos = signedAsset('macOS updater archive', (name) => name.endsWith('.app.tar.gz'));
const linux = signedAsset('Linux updater AppImage', (name) => name.endsWith('.AppImage'));
const windows = signedAsset('Windows updater installer', (name) => name.endsWith('-setup.exe'));

const manifest = {
  version,
  notes: notesFile ? fs.readFileSync(notesFile, 'utf8').trim() : '',
  pub_date: publishedAt,
  platforms: {
    'darwin-aarch64': macos,
    'darwin-x86_64': macos,
    'linux-x86_64': linux,
    'windows-x86_64': windows,
  },
};

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Rendered signed updater manifest for Pasted ${version}`);
