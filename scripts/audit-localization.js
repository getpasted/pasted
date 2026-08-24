import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const LOCALES_DIRECTORY = 'src/locales';
// This is an explicit debt ceiling, not a target. Move strings into the catalog
// and ratchet the number down as existing surfaces are localized.
const HARD_CODED_COPY_BUDGET = 0;

const readJson = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));
const manifest = readJson(path.join(LOCALES_DIRECTORY, 'manifest.json'));

assert.equal(manifest.schemaVersion, 1, 'The locale manifest schema version must be 1.');
assert.ok(Array.isArray(manifest.locales) && manifest.locales.length > 0,
  'The locale manifest must register at least one locale.');

const localeCodes = manifest.locales.map(({ code }) => code);
assert.equal(new Set(localeCodes).size, localeCodes.length, 'Locale codes must be unique.');
assert.ok(localeCodes.includes(manifest.defaultLocale), 'The default locale must be registered.');

const placeholderNames = (message) => new Set(
  [...message.matchAll(/\{([A-Za-z][A-Za-z0-9_]*)\}/g)].map((match) => match[1]),
);
const unionPlaceholders = (message) => {
  const values = typeof message === 'string' ? [message] : Object.values(message);
  return new Set(values.flatMap((value) => [...placeholderNames(value)]));
};
const sorted = (values) => [...values].sort();

const catalogs = new Map();
for (const locale of manifest.locales) {
  assert.match(locale.code, /^[a-z]{2,3}(?:-[A-Z][A-Za-z0-9]{1,7})*$/,
    `Locale code ${locale.code} must use a stable BCP 47-style form.`);
  assert.ok(locale.name && locale.nativeName, `Locale ${locale.code} must have English and native names.`);
  assert.ok(['ltr', 'rtl'].includes(locale.direction), `Locale ${locale.code} must declare ltr or rtl direction.`);
  assert.equal(locale.catalog, `${locale.code}.json`, `Locale ${locale.code} must use its canonical catalog filename.`);
  const catalogPath = path.join(LOCALES_DIRECTORY, locale.catalog);
  assert.ok(fs.existsSync(catalogPath), `Missing catalog ${catalogPath}.`);
  const catalog = readJson(catalogPath);
  for (const [key, message] of Object.entries(catalog)) {
    assert.match(key, /^[a-z][A-Za-z0-9]*(?:\.[a-zA-Z][A-Za-z0-9]*)+$/,
      `Localization key ${key} must be a stable dotted identifier.`);
    if (typeof message === 'string') {
      assert.ok(message.length > 0, `${locale.code}:${key} must not be empty.`);
      continue;
    }
    assert.ok(message && typeof message === 'object' && !Array.isArray(message),
      `${locale.code}:${key} must be a string or plural map.`);
    assert.equal(typeof message.other, 'string', `${locale.code}:${key} plural maps require “other”.`);
    for (const [category, variant] of Object.entries(message)) {
      assert.ok(['zero', 'one', 'two', 'few', 'many', 'other'].includes(category),
        `${locale.code}:${key} has unsupported plural category ${category}.`);
      assert.ok(typeof variant === 'string' && variant.length > 0,
        `${locale.code}:${key}.${category} must be a non-empty string.`);
    }
  }
  catalogs.set(locale.code, catalog);
}

const baseCatalog = catalogs.get(manifest.defaultLocale);
const baseKeys = Object.keys(baseCatalog).sort();
for (const [locale, catalog] of catalogs) {
  assert.deepEqual(Object.keys(catalog).sort(), baseKeys,
    `${locale} must contain exactly the canonical ${manifest.defaultLocale} catalog keys.`);
  for (const key of baseKeys) {
    assert.deepEqual(sorted(unionPlaceholders(catalog[key])), sorted(unionPlaceholders(baseCatalog[key])),
      `${locale}:${key} must preserve the canonical interpolation placeholders.`);
  }
  if (locale !== manifest.defaultLocale) {
    const sourceMessages = baseKeys.filter((key) => /[A-Za-z]{2}/.test(JSON.stringify(baseCatalog[key])));
    const unchanged = sourceMessages.filter((key) => JSON.stringify(catalog[key]) === JSON.stringify(baseCatalog[key]));
    assert.ok(unchanged.length <= Math.max(100, Math.floor(sourceMessages.length * 0.08)),
      `${locale} leaves ${unchanged.length} of ${sourceMessages.length} translatable messages unchanged.`);
    assert.doesNotMatch(JSON.stringify(catalog), /Your goal is to accurately convey|Produce only the .* translation/,
      `${locale} contains translation-prompt leakage.`);
    assert.doesNotMatch(JSON.stringify(catalog), /ZXQPH\d+QXZ/,
      `${locale} contains an unrestored draft-generation placeholder token.`);
  }
}

const rustLocalization = fs.readFileSync('src-tauri/src/localization.rs', 'utf8');
for (const { code, catalog } of manifest.locales) {
  assert.ok(rustLocalization.includes(`("${code}", english)`)
    || rustLocalization.includes(`("${code}", ${code.replaceAll('-', '_').toLowerCase()})`),
  `Rust must register ${catalog} so native menus share the GUI catalog.`);
  assert.ok(rustLocalization.includes(`src/locales/${catalog}`),
    `Rust must embed the shared ${catalog} catalog.`);
}

const nativeSources = [
  fs.readFileSync('src-tauri/src/app_menu.rs', 'utf8'),
  fs.readFileSync('src-tauri/src/app_tray.rs', 'utf8'),
].join('\n');
const referencedNativeKeys = new Set(
  [...nativeSources.matchAll(/t\("(native\.[A-Za-z0-9.]+)"\)/g)].map((match) => match[1]),
);
const catalogNativeKeys = baseKeys.filter((key) => key.startsWith('native.'));
assert.deepEqual(sorted(referencedNativeKeys), catalogNativeKeys,
  'Every native menu message must be cataloged and every native catalog key must be used.');
assert.doesNotMatch(nativeSources, /\.text\("[^"]+",\s*"[A-Za-z]/,
  'Native menu items must use shared catalog keys instead of embedded English labels.');

const componentFiles = fs.readdirSync('src/components')
  .filter((file) => file.endsWith('.tsx'))
  .map((file) => path.join('src/components', file));
let hardCodedCopyCount = 0;
const hardCodedCopyByFile = [];
for (const file of componentFiles) {
  const source = fs.readFileSync(file, 'utf8');
  const jsxTextCount = [...source.matchAll(/(?<!=)>\s*([A-Z][^<>{}]*)\s*</g)]
    .filter((match) => match[1].trim().length > 1)
    .length;
  const propCount = [...source.matchAll(/\b(?:aria-label|ariaLabel|closeLabel|description|emptyMessage|groupLabel|label|message|placeholder|searchPlaceholder|subtitle|title)=["']([^"']*[A-Za-z][^"']*)["']/g)]
    .filter((match) => !/^\/path\/|\\[bdDsSwW]|\{\d/.test(match[1]))
    .length;
  const fileCount = jsxTextCount + propCount;
  hardCodedCopyCount += fileCount;
  if (fileCount > 0) hardCodedCopyByFile.push({ file, fileCount, jsxTextCount, propCount });
}
if (process.env.LOCALIZATION_VERBOSE === '1') {
  for (const { file, fileCount, jsxTextCount, propCount } of hardCodedCopyByFile.sort((left, right) => right.fileCount - left.fileCount)) {
    console.log(`${String(fileCount).padStart(3)} ${file} (JSX: ${jsxTextCount}, props: ${propCount})`);
  }
}
if (process.env.LOCALIZATION_VERBOSE === '2') {
  for (const file of componentFiles) {
    const source = fs.readFileSync(file, 'utf8');
    const matches = [
      ...[...source.matchAll(/(?<!=)>\s*([A-Z][^<>{}]*)\s*</g)]
        .filter((match) => match[1].trim().length > 1),
      ...[...source.matchAll(/\b(?:aria-label|ariaLabel|closeLabel|description|emptyMessage|groupLabel|label|message|placeholder|searchPlaceholder|subtitle|title)=["']([^"']*[A-Za-z][^"']*)["']/g)]
        .filter((match) => !/^\/path\/|\\[bdDsSwW]|\{\d/.test(match[1])),
    ].sort((left, right) => left.index - right.index);
    if (matches.length === 0) continue;
    console.log(`\n${file}`);
    for (const match of matches) {
      const line = source.slice(0, match.index).split('\n').length;
      console.log(`  ${line}: ${match[1].trim()}`);
    }
  }
}
assert.ok(hardCodedCopyCount <= HARD_CODED_COPY_BUDGET,
  `Hardcoded interface-copy debt is ${hardCodedCopyCount}; keep it at or below ${HARD_CODED_COPY_BUDGET} and ratchet the budget downward as strings move into catalogs.`);

const sidebarSource = [
  'src/components/Sidebar.tsx',
  'src/components/SidebarSearchFooter.tsx',
].map((path) => fs.readFileSync(path, 'utf8')).join('\n');
const activitySource = fs.readFileSync('src/components/ActivityLogView.tsx', 'utf8');
const clipCardSource = fs.readFileSync('src/components/ClipCard.tsx', 'utf8');
const settingsBlacklistSource = fs.readFileSync('src/components/SettingsBlacklistPanel.tsx', 'utf8');
assert.doesNotMatch(sidebarSource, /prefix:\s*'[^']+'\s*,\s*desc:\s*'[A-Za-z]/,
  'Sidebar search-helper descriptions must be localized.');
assert.doesNotMatch(activitySource, /group:\s*'(?:Application|Capture|History|Organization)'/,
  'Activity filter group headings must be localized.');
assert.match(clipCardSource, /useLocalization\(\)/,
  'Memoized clip cards must subscribe to locale changes.');
assert.doesNotMatch(settingsBlacklistSource, /['"](?:Nothing|Everything|All content|Hotkeys)['"]/,
  'App-exclusion summaries must use localized catalog messages.');
assert.match(fs.readFileSync('src/localization/presentation.ts', 'utf8'), /localizedBuiltinName/,
  'Registry-backed built-in metadata must have a locale-aware presentation layer.');
assert.match(fs.readFileSync('src/localization/runtime.ts', 'utf8'), /sortedLocales[\s\S]*\.sort\([\s\S]*nativeName/,
  'The language picker must present locale names in alphabetical order.');

console.log(`Localization catalogs and native menu coverage passed. Hardcoded copy debt: ${hardCodedCopyCount}.`);
