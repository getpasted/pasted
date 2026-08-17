import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

function filesUnder(directory, extension) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const item = path.join(directory, entry.name);
    return entry.isDirectory() ? filesUnder(item, extension) : entry.name.endsWith(extension) ? [item] : [];
  });
}

const componentSources = [...filesUnder('src/components', '.tsx'), 'src/App.tsx'];
const manifest = JSON.parse(fs.readFileSync('src/locales/manifest.json', 'utf8'));
const arabicDefinition = manifest.locales.find(({ code }) => code === 'ar');
assert.equal(arabicDefinition?.direction, 'rtl', 'Arabic must be registered as a shipped RTL locale.');
const hebrewDefinition = manifest.locales.find(({ code }) => code === 'he');
assert.equal(hebrewDefinition?.direction, 'rtl', 'Hebrew must be registered as a shipped RTL locale.');
const physicalUtility = /\b(?:ml|mr|pl|pr)-|\btext-(?:left|right)\b|\bborder-(?:l|r)\b|\brounded-(?:l|r)(?:-|\b)|(?:^|\s)-?(?:left|right)-(?:\d|\[)[A-Za-z0-9./\[\]%-]*/g;
const utilityExceptions = new Map([
  ['src/App.tsx', new Set([' left-1/2'])], // Physical centering pairs with translateX(-50%).
]);

for (const file of componentSources) {
  const source = fs.readFileSync(file, 'utf8');
  const matches = [...source.matchAll(physicalUtility)].map(([match]) => match);
  const unexpected = matches.filter((match) => !utilityExceptions.get(file)?.has(match));
  assert.deepEqual(unexpected, [], `${file} contains direction-sensitive physical utilities: ${unexpected.join(', ')}`);
  assert.doesNotMatch(source, /<textarea(?!\s+dir="auto")/,
    `${file} textareas must determine direction from their content.`);
}

const physicalCss = /(?:^|[;{]\s*)(?:(?:left|right|margin-left|margin-right|padding-left|padding-right|border-left|border-right)\s*:|text-align\s*:\s*(?:left|right))/gm;
const cssExceptions = new Map([
  ['src/styles/theme-primitives.css', new Set(['border-left:', 'left:'])], // Checkmark stroke and centered HUD caret geometry.
]);
for (const file of filesUnder('src/styles', '.css')) {
  const source = fs.readFileSync(file, 'utf8');
  const matches = [...source.matchAll(physicalCss)].map(([match]) => match.trim().replace(/^[;{]\s*/, ''));
  const unexpected = matches.filter((match) => !cssExceptions.get(file)?.has(match));
  assert.deepEqual(unexpected, [], `${file} contains direction-sensitive physical CSS: ${unexpected.join(', ')}`);
}

const provider = fs.readFileSync('src/localization/LocalizationProvider.tsx', 'utf8');
const main = fs.readFileSync('src/main.tsx', 'utf8');
const app = fs.readFileSync('src/App.tsx', 'utf8');
const runtime = fs.readFileSync('src/localization/runtime.ts', 'utf8');
const preview = fs.readFileSync('src/components/ClipPreviewContent.tsx', 'utf8');
const overflowText = fs.readFileSync('src/components/OverflowText.tsx', 'utf8');
const columnResize = fs.readFileSync('src/hooks/useColumnResize.ts', 'utf8');
const anchoredMenu = fs.readFileSync('src/components/AnchoredMenu.tsx', 'utf8');
const settingsSwitch = fs.readFileSync('src/components/SettingsSwitch.tsx', 'utf8');
const titlebar = fs.readFileSync('src-tauri/src/titlebar.rs', 'utf8');
const binPicker = fs.readFileSync('src/components/ClipBinPicker.tsx', 'utf8');
const menuSelect = fs.readFileSync('src/components/MenuSelect.tsx', 'utf8');
const macWindowControls = fs.readFileSync('src/components/MacRtlWindowControls.tsx', 'utf8');
const desktopCapability = fs.readFileSync('src-tauri/capabilities/default.json', 'utf8');
const directionalSources = [
  'src/components/AnchoredMenu.tsx',
  'src/components/HelpView.tsx',
  'src/components/SettingsAboutPanel.tsx',
  'src/components/Sidebar.tsx',
  'src/components/WelcomeSetup.tsx',
].map((file) => fs.readFileSync(file, 'utf8')).join('\n');

assert.match(provider, /document\.documentElement\.dir = snapshot\.direction/,
  'The document direction must follow the effective locale.');
assert.match(main, /document\.documentElement\.dir = initialLocalization\.direction/,
  'Startup must apply the cached direction before the first React paint.');
assert.match(runtime, /isolateBidi/,
  'RTL message interpolation must isolate user and technical values.');
assert.match(preview, /dir="auto"[^>]*clip-text-content/,
  'Clipboard text must determine its own direction independently of the interface.');
assert.match(overflowText, /dir: props\.dir \?\? 'auto'/,
  'Overflowing user-defined labels must determine their own direction.');
assert.match(columnResize, /inlineResizeDelta\(startX, moveEvent\.clientX, direction\)/,
  'Column resizing must grow toward the active inline direction.');
assert.match(anchoredMenu, /direction === 'rtl' \? fitsLeft \|\| !fitsRight/,
  'Submenus must prefer opening toward inline-end in RTL layouts.');
assert.match(settingsSwitch, /ltr:translate-x-4 rtl:-translate-x-4/,
  'Enabled switch thumbs must move toward inline-end in both directions.');
assert.match(titlebar, /setHidden: rtl/,
  'AppKit-managed traffic lights must be hidden while stable RTL controls are shown.');
assert.match(app, /direction === 'ltr' && previousDirection !== 'rtl'/,
  'An initially LTR window must leave its native macOS traffic lights untouched.');
assert.match(titlebar, /TRAFFIC_LIGHT_Y[\s\S]*titlebar_height[\s\S]*titlebar_container, setFrame:/,
  'Returning to LTR must immediately restore Tauri’s configured titlebar container inset.');
assert.match(macWindowControls, /getCurrentWindow[\s\S]*toggleMaximize/,
  'Stable RTL window controls must retain close, minimize, and zoom behavior.');
for (const permission of ['allow-close', 'allow-minimize', 'allow-toggle-maximize']) {
  assert.ok(desktopCapability.includes(`core:window:${permission}`),
    `Stable RTL window controls require the core:window:${permission} capability.`);
}
assert.match(binPicker, /bidi-interface-align[\s\S]*flex-1 truncate/,
  'Mixed-direction Bin names must align with the surrounding menu direction.');
assert.match(menuSelect, /bidi-interface-align[\s\S]*option\.label/,
  'Mixed-direction select values must align with the surrounding menu direction.');
assert.ok((directionalSources.match(/rtl:-scale-x-100/g) ?? []).length >= 8,
  'Navigation and disclosure icons must mirror in RTL layouts.');

const arabic = JSON.parse(fs.readFileSync('src/locales/ar.json', 'utf8'));
for (const [key, message] of Object.entries(arabic)) {
  if (!message || typeof message !== 'object' || Array.isArray(message)) continue;
  assert.deepEqual(Object.keys(message).sort(), ['few', 'many', 'one', 'other', 'two', 'zero'],
    `Arabic plural message ${key} must cover every CLDR category.`);
}

const hebrew = JSON.parse(fs.readFileSync('src/locales/he.json', 'utf8'));
for (const [key, message] of Object.entries(hebrew)) {
  if (!message || typeof message !== 'object' || Array.isArray(message)) continue;
  assert.deepEqual(Object.keys(message).sort(), ['one', 'other', 'two'],
    `Hebrew plural message ${key} must cover every CLDR category.`);
}

console.log('RTL layout, mixed-content, and directional-control audit passed.');
