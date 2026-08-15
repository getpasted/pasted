import assert from 'node:assert/strict';
import fs from 'node:fs';

const TOOL_COPY_FILES = [
  'src/components/ActivityLogView.tsx',
  'src/components/AddBlacklistAppModal.tsx',
  'src/components/AnalyticsView.tsx',
  'src/components/CaptureFeedbackWindow.tsx',
  'src/components/ConnectionModal.tsx',
  'src/components/ContentExtractorManagerDialog.tsx',
  'src/components/DeleteTransformationAssetDialog.tsx',
  'src/components/ExternalHistoryImport.tsx',
  'src/components/HelpView.tsx',
  'src/components/IntentTransformComposer.tsx',
  'src/components/OpenSourceLicensesDialog.tsx',
  'src/components/OperationEditorModal.tsx',
  'src/components/OperationsManager.tsx',
  'src/components/PipelineEditorModal.tsx',
  'src/components/SettingsAboutPanel.tsx',
  'src/components/SettingsBlacklistPanel.tsx',
  'src/components/SettingsDetectionPanel.tsx',
  'src/components/SettingsOcrPanel.tsx',
  'src/components/SettingsFeaturesPanel.tsx',
  'src/components/SettingsGeneralPanel.tsx',
  'src/components/SettingsHotkeysPanel.tsx',
  'src/components/SettingsModal.tsx',
  'src/components/SettingsNotificationsPanel.tsx',
  'src/components/SettingsResetPanel.tsx',
  'src/components/SettingsSyncPanel.tsx',
  'src/components/SettingsTabs.tsx',
  'src/components/TransformComposerModal.tsx',
  'src/components/TransformationsView.tsx',
  'src/utils/features.ts',
];

const ALLOWED_COPY = [
  // Literal packaged-app paths used by CLI installation instructions.
  ['src/components/HelpView.tsx', '/Applications/Pasted.app/Contents/MacOS/pasted'],
  // “Pasted” is the action verb in these Activity labels, not the product name.
  ['src/components/ActivityLogView.tsx', '<span>Queue Pasted</span>'],
  ['src/components/ActivityLogView.tsx', '<span>HUD Pasted</span>'],
  // About and installation diagnostics intentionally identify the product.
  ['src/components/SettingsAboutPanel.tsx', '`Pasted ${details.appVersion} (${details.buildKind})`'],
  ['src/components/SettingsAboutPanel.tsx', "'Not installed beside Pasted'"],
  ['src/components/SettingsAboutPanel.tsx', 'title="About Pasted"'],
  ['src/components/SettingsAboutPanel.tsx', '>Pasted</h3>'],
  // The Covenant deliberately addresses the user to emphasize ownership.
  ['src/components/SettingsAboutPanel.tsx', 'keep your clipboard yours'],
  ['src/components/SettingsAboutPanel.tsx', 'Your core library lives locally'],
  ['src/components/SettingsAboutPanel.tsx', 'hosted copy of your history'],
  ['src/components/SettingsAboutPanel.tsx', 'Your clipboard is yours'],
  // macOS requires approval under the literal application name shown by the OS.
  ['src/components/SettingsHotkeysPanel.tsx', 'Allow <strong>Pasted</strong> under <strong>System Settings'],
  // Example executable name, not first-person interface narration.
  ['src/components/ConnectionModal.tsx', '/usr/local/bin/my-planner'],
  // Destructive actions name their scope explicitly.
  ['src/components/SettingsResetPanel.tsx', "message: 'Pasted was reset to its first-launch state.'"],
  ['src/components/SettingsResetPanel.tsx', '>Reset Pasted</h3>'],
  ['src/components/SettingsResetPanel.tsx', 'Reset Pasted…'],
];

const allowedByFile = new Map();
for (const [file, snippet] of ALLOWED_COPY) {
  const snippets = allowedByFile.get(file) ?? [];
  snippets.push(snippet);
  allowedByFile.set(file, snippets);
}

const violations = [];
for (const file of TOOL_COPY_FILES) {
  const source = fs.readFileSync(file, 'utf8');
  const allowedSnippets = allowedByFile.get(file) ?? [];

  for (const snippet of allowedSnippets) {
    assert.ok(source.includes(snippet), `UI copy allowlist entry is stale in ${file}: ${snippet}`);
  }

  source.split('\n').forEach((line, index) => {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('//') || trimmed.startsWith('*')) return;
    let auditedLine = line;
    for (const snippet of allowedSnippets) {
      auditedLine = auditedLine.split(snippet).join('');
    }
    const hasProductName = /\bPasted\b/.test(auditedLine);
    const hasPronoun = /\b(?:i|me|my|mine|we|our|ours|us|you|your|yours)\b/.test(auditedLine.toLowerCase());
    const hasOutsideNarratorPhrase = /\bthe app\b/i.test(auditedLine);
    if (!hasProductName && !hasPronoun && !hasOutsideNarratorPhrase) return;
    violations.push(`${file}:${index + 1}: ${trimmed}`);
  });
}

assert.deepEqual(
  violations,
  [],
  `Tools UI copy must use neutral, in-app phrasing. Add only necessary, documented exceptions:\n${violations.join('\n')}`,
);

const settingsImportExport = fs.readFileSync('src/components/SettingsSyncPanel.tsx', 'utf8');
const settingsTabs = fs.readFileSync('src/components/SettingsTabs.tsx', 'utf8');
const settingsDestinations = [
  ['general', 'General'],
  ['functionality', 'Functionality'],
  ['hotkeys', 'Hotkeys'],
  ['notifications', 'Notifications'],
  ['app-exclusions', 'App Exclusions'],
  ['storage', 'Storage'],
  ['analysis', 'Analysis'],
  ['intelligence', 'Intelligence'],
  ['about', 'About'],
];
let previousSettingsTab = -1;
for (const [id, label] of settingsDestinations) {
  const entry = `{ id: '${id}', label: '${label}'`;
  const index = settingsTabs.indexOf(entry);
  assert.ok(index > previousSettingsTab, `Settings destination ${label} must retain its requested order`);
  previousSettingsTab = index;
}
assert.doesNotMatch(settingsTabs, /id: '(?:features|connections|blacklist)'/,
  'Settings destination IDs must match current product terminology');
for (const title of ['Database Location', 'Export', 'Import']) {
  assert.ok(
    settingsImportExport.includes(`>${title}<`) || settingsImportExport.includes(`title="${title}"`),
    `Storage titles must retain title case: ${title}`,
  );
}
for (const dataLabel of ['Clips', 'Organization', 'Activity', 'Settings and Application Data', 'Revisions and Automation History', 'Interface and Window State']) {
  assert.ok(
    settingsImportExport.includes(`label: '${dataLabel}'`),
    `Storage must retain the ${dataLabel} data selection`,
  );
}
assert.match(
  settingsImportExport,
  /\['json', 'csv', 'backup'\]/,
  'Storage must show JSON, CSV, and Backup in one format control',
);
assert.doesNotMatch(
  settingsImportExport,
  /title="Backup and Transfer"/,
  'Storage must not add a redundant wrapper title around backup, recovery, and portable merge actions',
);
assert.doesNotMatch(
  settingsImportExport,
  /(?:Export JSON|Import JSON)…/,
  'Direct Import and Export actions must not use decorative ellipses',
);
assert.doesNotMatch(
  settingsImportExport,
  /<ActionButton[^>]*variant="primary"[^>]*>[\s\S]{0,120}Export JSON/,
  'History and Organization actions must have equal visual weight',
);

const INLINE_METADATA_LABELS = new Set([
  '<span>Items:</span>',
  '<span>Types:</span>',
  '<span>Size:</span>',
  '<span>Available:</span>',
  '<span>Chars:</span>',
  '<span>Words:</span>',
  '<span>Lines:</span>',
  '<span>Revisions:</span>',
  '<span>Captured:</span>',
]);
const structuralColonViolations = [];
const componentFiles = fs.readdirSync('src/components')
  .filter((name) => name.endsWith('.tsx'))
  .map((name) => `src/components/${name}`);

const compoundNavigationViolations = [];
const navigationSources = [
  {
    file: 'src/components/HelpView.tsx',
    source: fs.readFileSync('src/components/HelpView.tsx', 'utf8').match(/const HELP_TOPICS[\s\S]*?\n\];/)?.[0] ?? '',
    pattern: /label: '([^']+)'/g,
  },
  {
    file: 'src/components/SettingsTabs.tsx',
    source: fs.readFileSync('src/components/SettingsTabs.tsx', 'utf8').match(/const TABS[\s\S]*?\n\] as const;/)?.[0] ?? '',
    pattern: /label: '([^']+)'/g,
  },
  {
    file: 'src/components/Sidebar.tsx',
    source: fs.readFileSync('src/components/Sidebar.tsx', 'utf8').match(/const allToolNavItems[\s\S]*?\n  \];/)?.[0] ?? '',
    pattern: /label: '([^']+)'/g,
  },
  {
    file: 'src-tauri/src/app_menu.rs',
    source: fs.readFileSync('src-tauri/src/app_menu.rs', 'utf8'),
    pattern: /\.text\("[^"]+", "([^"]+)"\)/g,
  },
];

const transformationTabs = fs.readFileSync('src/components/TransformWorkspaceHeader.tsx', 'utf8');
for (const title of ['Library', 'Operations', 'Playground']) {
  assert.match(
    transformationTabs,
    new RegExp(`role="tab"[\\s\\S]{0,80}title="${title}"`),
    `The Transformations ${title} tab must retain its title when labels collapse to icons`,
  );
}

for (const { file, source, pattern } of navigationSources) {
  assert.ok(source, `Navigation copy source could not be located in ${file}`);
  for (const match of source.matchAll(pattern)) {
    const label = match[1];
    if (!label.includes(' and ')) continue;
    const invalidSegment = label.split(' and ').find((segment) => {
      const words = segment.split(/\s+/);
      const minorWords = new Set(['a', 'an', 'the', 'at', 'by', 'for', 'in', 'of', 'on', 'or', 'per', 'to', 'via', 'vs']);
      return words.some((word, index) => {
        const lowercaseWord = word.toLowerCase();
        const mayBeLowercase = index > 0 && index < words.length - 1 && minorWords.has(lowercaseWord);
        return mayBeLowercase ? word !== lowercaseWord : !/^[A-Z0-9]/.test(word);
      });
    });
    if (invalidSegment) compoundNavigationViolations.push(`${file}: ${label}`);
  }
}

assert.deepEqual(
  compoundNavigationViolations,
  [],
  `Compound navigation items must use title case with a lowercase “and”:\n${compoundNavigationViolations.join('\n')}`,
);

const structuralLabelPattern = /<(label|span|p|button|h[1-6])\b[^>]*>[^<{\n]*:<\/\1>/g;

for (const file of componentFiles) {
  const source = fs.readFileSync(file, 'utf8');
  source.split('\n').forEach((line, index) => {
    for (const match of line.matchAll(structuralLabelPattern)) {
      if (!INLINE_METADATA_LABELS.has(match[0])) {
        structuralColonViolations.push(`${file}:${index + 1}: ${match[0]}`);
      }
    }
  });
}

assert.deepEqual(
  structuralColonViolations,
  [],
  `Structural UI labels must not end with colons; reserve them for inline key–value metadata:\n${structuralColonViolations.join('\n')}`,
);

const descriptionPunctuationViolations = [];
const literalDescriptionPattern = /description=(?:"([^"]+)"|'([^']+)')/g;
for (const file of componentFiles) {
  const source = fs.readFileSync(file, 'utf8');
  source.split('\n').forEach((line, index) => {
    for (const match of line.matchAll(literalDescriptionPattern)) {
      const description = match[1] ?? match[2];
      if (!/[.!?…]$/.test(description)) {
        descriptionPunctuationViolations.push(`${file}:${index + 1}: ${description}`);
      }
    }
  });
}

assert.deepEqual(
  descriptionPunctuationViolations,
  [],
  `Literal UI descriptions must be complete, punctuated sentences:\n${descriptionPunctuationViolations.join('\n')}`,
);

const rawAmpersandViolations = [];
const attributedAmpersandPattern = /(?:label|title|placeholder|description)\s*(?::|=)\s*["'][^"']*\s&\s[^"']*["']/;
const jsxAmpersandPattern = />[^<{\n]*\s&\s[^<{\n]*</;
for (const file of componentFiles) {
  const source = fs.readFileSync(file, 'utf8');
  source.split('\n').forEach((line, index) => {
    if (attributedAmpersandPattern.test(line) || jsxAmpersandPattern.test(line)) {
      rawAmpersandViolations.push(`${file}:${index + 1}: ${line.trim()}`);
    }
  });
}

assert.deepEqual(
  rawAmpersandViolations,
  [],
  `Interface copy must use “and” instead of raw ampersands:\n${rawAmpersandViolations.join('\n')}`,
);

const timestampViolations = [];
for (const file of componentFiles) {
  const source = fs.readFileSync(file, 'utf8');
  source.split('\n').forEach((line, index) => {
    if (/\.toLocale(?:Date|Time)String\(/.test(line)) {
      timestampViolations.push(`${file}:${index + 1}: format dates through src/utils/date.ts`);
    }
  });
  for (const match of source.matchAll(/<time\b([\s\S]*?)>([\s\S]*?)<\/time>/g)) {
    const [, attributes, contents] = match;
    if (!attributes.includes('dateTime=') || !attributes.includes('title={formatFullDateTime(') || !contents.includes('formatRelativeTime(')) {
      const line = source.slice(0, match.index).split('\n').length;
      timestampViolations.push(`${file}:${line}: timestamps need relative text, a machine-readable dateTime, and a full-detail title`);
    }
  }
}

const dateUtilities = fs.readFileSync('src/utils/date.ts', 'utf8');
assert.match(dateUtilities, /second: '2-digit'/, 'Full timestamp tooltips must retain seconds');
assert.match(dateUtilities, /timeZoneName: 'short'/, 'Full timestamp tooltips must retain the local timezone');
assert.deepEqual(
  timestampViolations,
  [],
  `Visible timestamps must use compact relative time with full details in the tooltip:\n${timestampViolations.join('\n')}`,
);

console.log(`UI copy voice and label audit passed for ${TOOL_COPY_FILES.length} Tools surfaces and ${componentFiles.length} components.`);
