import assert from 'node:assert/strict';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, extname, join, normalize, relative } from 'node:path';

const repositoryRoot = process.cwd();
const documentationRoots = [
  'README.md',
  'CHANGELOG.md',
  'CONTRIBUTING.md',
  'SECURITY.md',
  'SUPPORT.md',
  'docs',
];

function markdownFiles(path) {
  const absolute = join(repositoryRoot, path);
  if (!existsSync(absolute)) return [];
  if (!statSync(absolute).isDirectory()) return extname(path) === '.md' ? [path] : [];
  return readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const child = join(path, entry.name);
    return entry.isDirectory() ? markdownFiles(child) : extname(entry.name) === '.md' ? [child] : [];
  });
}

const files = documentationRoots.flatMap(markdownFiles);
const brokenLinks = [];

for (const file of files) {
  const content = readFileSync(join(repositoryRoot, file), 'utf8');
  for (const match of content.matchAll(/!?\[[^\]]*\]\(([^)]+)\)/g)) {
    let target = match[1].trim().replace(/^<|>$/g, '').split(/\s+["']/)[0];
    if (!target || /^(?:#|https?:|mailto:)/i.test(target)) continue;
    target = decodeURIComponent(target.split('#')[0]);
    if (!target) continue;

    let resolved = normalize(join(dirname(file), target));
    if (file.startsWith('docs/wiki/') && !extname(resolved)) resolved += '.md';
    if (!existsSync(join(repositoryRoot, resolved))) {
      brokenLinks.push(`${file} -> ${target}`);
    }
  }
}

assert.deepEqual(brokenLinks, [], `Documentation contains broken relative links:\n${brokenLinks.join('\n')}`);

const productDocs = [
  'README.md',
  ...files.filter((file) => file.startsWith('docs/wiki/')),
  'src/components/HelpView.tsx',
].map((file) => [file, readFileSync(join(repositoryRoot, file), 'utf8')]);

for (const [file, content] of productDocs) {
  for (const stale of [
    'Analytics & Insights',
    'Content Analysis, Classification, and Types',
    'Content Detection',
    'Detectors',
    'Enrichers',
    'Detection-and-Types',
    'pasted detector',
    'pasted enricher',
    'custom Type',
    'Manage Types',
    'Restore Defaults',
  ]) {
    assert.ok(!content.includes(stale), `${file} must not use stale product wording: ${stale}`);
  }
}

const cliReference = readFileSync(join(repositoryRoot, 'docs/wiki/CLI-Reference.md'), 'utf8');
assert.match(
  cliReference,
  /registry list \[--kind capture\|inspector\|extractor\|classifier\|suggestion\|operation\|transform\]/,
  'CLI Reference must list every registry kind, including Capture',
);

for (const file of [
  'README.md',
  'docs/ANALYSIS_ARCHITECTURE.md',
  'docs/ANALYSIS_ACCEPTANCE.md',
  'docs/wiki/Classification-and-Types.md',
  'docs/wiki/Settings-and-Features.md',
]) {
  const content = readFileSync(join(repositoryRoot, file), 'utf8');
  for (const term of ['Clip Type', 'Content Type']) {
    assert.ok(content.includes(term), `${file} must explain ${term}`);
  }
}

const taxonomyGuide = readFileSync(join(repositoryRoot, 'docs/wiki/Classification-and-Types.md'), 'utf8');
assert.ok(taxonomyGuide.includes('File Format'), 'Content Analysis guide must distinguish File Format');

const smartBinContract = readFileSync(join(repositoryRoot, 'docs/wiki/Smart-Bin-Rule-Contract.md'), 'utf8');
for (const target of ['clip_type', 'content_type', 'file_format', 'source']) {
  assert.ok(smartBinContract.includes(`\`${target}\``), `Smart Bin contract must document ${target}`);
}
assert.ok(smartBinContract.includes('"version": 1'), 'Smart Bin contract must document its format version');

console.log(`Documentation audit passed (${files.length} Markdown files checked).`);
