import assert from 'node:assert/strict';
import fs from 'node:fs';

const TOOL_COPY_FILES = [
  'src/components/ActivityLogView.tsx',
  'src/components/ActivityEventBadge.tsx',
  'src/components/AddBlacklistAppModal.tsx',
  'src/components/AnalyticsView.tsx',
  'src/components/CaptureFeedbackWindow.tsx',
  'src/components/CaptureFeedbackCard.tsx',
  'src/components/ConnectionModal.tsx',
  'src/components/ContentExtractorManagerDialog.tsx',
  'src/components/ExtractorAuthoringHistoryDialog.tsx',
  'src/components/ExtractorRecipeEditor.tsx',
  'src/components/ExtractorRegistryPanel.tsx',
  'src/components/contentExtractorModel.ts',
  'src/hooks/useContentExtractorManager.ts',
  'src/components/ContentTypeManagerDialog.tsx',
  'src/components/DeleteTransformationAssetDialog.tsx',
  'src/components/ExternalHistoryImport.tsx',
  'src/components/HelpView.tsx',
  'src/components/HelpCliTopic.tsx',
  'src/components/helpCliCatalog.ts',
  'src/components/IntentTransformComposer.tsx',
  'src/components/OpenSourceLicensesDialog.tsx',
  'src/components/OperationEditorModal.tsx',
  'src/components/OperationsManager.tsx',
  'src/components/ManualTransformEditorModal.tsx',
  'src/components/ManualTransformStepEditor.tsx',
  'src/components/SettingsAboutPanel.tsx',
  'src/components/SettingsBlacklistPanel.tsx',
  'src/components/SettingsAnalysisPanel.tsx',
  'src/components/AnalysisLifecycleSequence.tsx',
  'src/components/ClassifierManagerDialog.tsx',
  'src/components/classifierModel.ts',
  'src/hooks/useAnalysisMaintenance.ts',
  'src/hooks/useClassifierManager.ts',
  'src/components/SettingsOcrPanel.tsx',
  'src/components/SettingsFeaturesPanel.tsx',
  'src/components/SettingsGeneralPanel.tsx',
  'src/components/SettingsGeneralAppearanceSection.tsx',
  'src/components/SettingsGeneralLayoutSection.tsx',
  'src/components/SettingsGeneralRetentionSections.tsx',
  'src/components/SettingsHotkeysPanel.tsx',
  'src/components/SettingsModal.tsx',
  'src/components/SettingsNotificationsPanel.tsx',
  'src/components/SettingsResetPanel.tsx',
  'src/components/SettingsSecurityPanel.tsx',
  'src/components/SettingsSyncPanel.tsx',
  'src/components/SettingsSyncLibrarySection.tsx',
  'src/components/SettingsSyncExportSection.tsx',
  'src/components/SettingsSyncImportSection.tsx',
  'src/components/SettingsTabs.tsx',
  'src/components/TransformComposerModal.tsx',
  'src/components/TransformationsView.tsx',
  'src/utils/features.ts',
];

const ALLOWED_COPY = [
  // Literal packaged-app paths used by CLI installation instructions.
  ['src/components/helpCliCatalog.ts', '/Applications/Pasted.app/Contents/MacOS/pasted'],
  // About and installation diagnostics intentionally identify the product.
  ['src/components/SettingsAboutPanel.tsx', '`Pasted ${details.appVersion} (${buildKind})`'],
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
    auditedLine = auditedLine.replace(/\bclassName\s*=\s*(?:"[^"]*"|'[^']*')/g, '');
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

const settingsImportExport = [
  'src/components/SettingsSyncPanel.tsx',
  'src/components/SettingsSyncLibrarySection.tsx',
  'src/components/SettingsSyncExportSection.tsx',
  'src/components/SettingsSyncImportSection.tsx',
].map((file) => fs.readFileSync(file, 'utf8')).join('\n');
const settingsTabs = fs.readFileSync('src/components/SettingsTabs.tsx', 'utf8');
const settingsHotkeys = [
  'src/components/SettingsHotkeysPanel.tsx',
  'src/hotkeySettingsModel.ts',
].map((file) => fs.readFileSync(file, 'utf8')).join('\n');
const settingsFeatures = fs.readFileSync('src/components/SettingsFeaturesPanel.tsx', 'utf8');
const helpView = [
  'src/components/HelpView.tsx',
  'src/components/helpCliCatalog.ts',
].map((file) => fs.readFileSync(file, 'utf8')).join('\n');
const app = [
  fs.readFileSync('src/App.tsx', 'utf8'),
  fs.readFileSync('src/components/AppShellView.tsx', 'utf8'),
  fs.readFileSync('src/components/AppDestinationView.tsx', 'utf8'),
].join('\n');
const appNavigation = fs.readFileSync('src/utils/appNavigation.ts', 'utf8');
const nativeMenu = fs.readFileSync('src-tauri/src/app_menu.rs', 'utf8');
const englishCatalog = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const localeManifest = JSON.parse(fs.readFileSync('src/locales/manifest.json', 'utf8'));
const intentionalProductNameKeys = new Set([
  // “Pasted” is the past-tense action result here, not the product name.
  'component.activityLogView.hudPasted',
  'component.activityLogView.queuePasted',
  'component.clipPreview.pasted',
  'component.transformationOutputActions.pasted',
  // Destructive actions name their concrete scope.
  'component.factoryResetDialog.resetPasted',
  'component.factoryResetDialog.resetPasted2',
  'component.settingsResetPanel.resetPasted',
  'component.settingsResetPanel.resetPasted2',
  // Product identity, literal operating-system labels, and named technical formats.
  'component.settingsAboutPanel.aboutPasted',
  'component.settingsAboutPanel.pasted',
  'component.settingsHotkeysPanel.pasted',
  'component.contentExtractorManagerDialog.pastedJson',
  'native.app.about',
  'native.file.quit',
  'native.tray.quit',
]);
for (const { code, catalog } of localeManifest.locales) {
  const messages = JSON.parse(fs.readFileSync(`src/locales/${catalog}`, 'utf8'));
  const redundantProductNames = Object.entries(messages)
    .filter(([key, value]) => typeof value === 'string' && /\bPasted\b/.test(value) && !intentionalProductNameKeys.has(key))
    .map(([key, value]) => `${code}:${key}: ${value}`);
  assert.deepEqual(redundantProductNames, [],
    `Localized in-app copy must not narrate from outside the product:\n${redundantProductNames.join('\n')}`);
}
const firstPersonCatalogCopy = Object.entries(englishCatalog)
  .filter(([, value]) => typeof value === 'string' && /\b(?:we|our|ours|us)\b/i.test(value))
  .map(([key, value]) => `${key}: ${value}`);
assert.deepEqual(firstPersonCatalogCopy, [],
  `English in-app copy must use neutral phrasing instead of first-person pronouns:\n${firstPersonCatalogCopy.join('\n')}`);
const localizedValueIsUsed = (source, expected) => Object.entries(englishCatalog)
  .some(([key, value]) => value === expected && source.includes(`'${key}'`));
const canonicalAnalysisCopyFiles = [
  'src/components/ContentExtractorManagerDialog.tsx',
  'src/components/ExtractorAuthoringHistoryDialog.tsx',
  'src/components/ExtractorRecipeEditor.tsx',
  'src/components/ExtractorRegistryPanel.tsx',
  'src/components/contentExtractorModel.ts',
  'src/hooks/useContentExtractorManager.ts',
  'src/components/ContentTypeManagerDialog.tsx',
  'src/components/SettingsAnalysisPanel.tsx',
  'src/components/AnalysisLifecycleSequence.tsx',
  'src/components/ClassifierManagerDialog.tsx',
  'src/components/classifierModel.ts',
  'src/hooks/useAnalysisMaintenance.ts',
  'src/hooks/useClassifierManager.ts',
  'src/components/SettingsSyncPanel.tsx',
  'src/components/SettingsSyncLibrarySection.tsx',
  'src/components/SettingsSyncExportSection.tsx',
  'src/components/SettingsSyncImportSection.tsx',
];
for (const file of canonicalAnalysisCopyFiles) {
  const source = fs.readFileSync(file, 'utf8');
  for (const stale of [
    'Restore Defaults',
    'Restore Shipped Defaults',
    'Built-in Types',
    'Custom Types',
    'Shipped Types',
  ]) {
    assert.ok(!source.includes(stale), `${file} must not use stale Analysis wording: ${stale}`);
  }
}
assert.match(settingsHotkeys, /<SettingsPanelResetNote[^>]*onReset=\{requestReset\}/,
  'Hotkeys must use the shared resettable Settings note well');
assert.match(settingsHotkeys, /hotkeyResetChanges[\s\S]*<SettingsResetChanges/,
  'Hotkeys Reset must preview only effective shortcut changes');
assert.doesNotMatch(settingsHotkeys, /<SettingsPanelHeader[\s\S]{0,300}actions=/,
  'Hotkeys must keep Reset in the shared footer instead of the panel header');
const hotkeySectionKeys = [
  'component.settingsHotkeysPanel.actions',
  'component.settingsHotkeysPanel.customBinHotkeys',
  'component.settingsHotkeysPanel.clipHotkeysCount',
  'component.settingsHotkeysPanel.savedTransformHotkeys',
  'component.settingsHotkeysPanel.pasteClipsByPosition',
];
const hotkeySectionPositions = hotkeySectionKeys.map((key) => settingsHotkeys.indexOf(`translate('${key}'`));
assert.ok(hotkeySectionPositions.every((position) => position >= 0), 'Hotkeys must render every configurable section');
assert.deepEqual([...hotkeySectionPositions].sort((left, right) => left - right), hotkeySectionPositions,
  'Hotkey sections must remain ordered Actions, Bins, Clips, Transforms, then HUD clip positions');
const mainWindowAction = settingsHotkeys.indexOf("key: 'openMainWindowHotkey'");
const lockAppAction = settingsHotkeys.indexOf("key: 'lockAppHotkey'");
const queueAction = settingsHotkeys.indexOf("key: 'seqToggleHotkey'");
assert.ok(mainWindowAction >= 0 && mainWindowAction < lockAppAction && lockAppAction < queueAction,
  'Toggle Main Window and Lock App must lead the Actions group');
assert.match(settingsFeatures, /translate\('component\.settingsFeaturesPanel\.chooseWhichFeaturesAreAvailable'\)/,
  'Functionality must keep its header description concise');
assert.match(settingsFeatures, /<SettingsPanelNote>[\s\S]*translate\('component\.settingsFeaturesPanel\.simpleEnablesEssentialClipboardToolsFullEnablesEveryFeatureDisablingAFeature'\)[\s\S]*<\/SettingsPanelNote>/,
  'Functionality must move preset and preservation guidance into the shared Settings note well');
assert.equal(englishCatalog['feature.cli.description'], 'Use {command} to automate clipboard workflows.',
  'The Command-Line Interface card must keep the literal command in a localizable placeholder');
assert.match(settingsFeatures, /<LocalizedInlineCode message=\{translate\(keys\.description, \{ command: 'pasted' \}\)\} code="pasted" \/>/,
  'The Command-Line Interface card must render the pasted executable as inline code');
const localizedInlineCode = fs.readFileSync('src/components/LocalizedInlineCode.tsx', 'utf8');
assert.match(localizedInlineCode, /<code dir="ltr" className="[^"]*theme-code-surface[^"]*border[^"]*font-mono[^"]*">\{code\}<\/code>/,
  'Localized inline code must remain an explicitly styled, monospaced, LTR code badge');
for (const [menuId, topic, catalogKey, label] of [
  ['help.getting_started', 'getting-started', 'native.help.gettingStarted', 'Getting Started'],
  ['help.shortcuts', 'shortcuts-hud', 'native.help.shortcuts', 'Hotkeys and HUD'],
  ['help.privacy', 'privacy-capture', 'native.help.privacy', 'Privacy and Capture'],
  ['help.deletion', 'deletion-recovery', 'native.help.deletion', 'Deletion and Recovery'],
  ['help.analysis', 'analysis', 'native.help.analysis', 'Content Analysis'],
  ['help.transformations', 'transformations', 'native.help.transformations', 'Transformations'],
  ['help.cli', 'cli', 'native.help.cli', 'CLI Commands'],
]) {
  assert.ok(nativeMenu.includes(`"${menuId}" => MenuDispatch::Navigate("help:${topic}")`),
    `Native Help menu must route ${label} to its matching Help topic`);
  assert.ok(nativeMenu.includes(`.text("${menuId}", t("${catalogKey}"))`),
    `Native Help menu must use the Help topic label ${label}`);
  assert.equal(englishCatalog[catalogKey], label,
    `The English native catalog must match the canonical Help label ${label}`);
  const helpCatalogKey = Object.entries(englishCatalog).find(([, value]) => value === label)?.[0];
  assert.ok(helpCatalogKey && helpView.includes(`id: '${topic}', get label() { return translate('${helpCatalogKey}'); }`),
    `Help must register the localized ${label} topic with its canonical ID`);
  assert.ok(appNavigation.includes(`'${topic}'`), `App navigation must accept the ${label} Help topic`);
}
for (const [file, labels] of Object.entries({
  'src/components/SettingsBlacklistPanel.tsx': ['Add app…', 'Hotkeys', 'checked hotkeys'],
  'src/components/IntelligenceConnectionsPanel.tsx': ['Add connection…'],
  'src/components/SettingsWelcomePanel.tsx': ['Open Copycat Welcome…'],
  'src/components/SettingsAboutPanel.tsx': ['Open Source Licenses…'],
  'src/components/SettingsAnalysisPanel.tsx': ['Rescan Clips…'],
  'src/components/AnalysisLifecycleSequence.tsx': ['Manage {title}…'],
  'src/components/ClassifierManagerDialog.tsx': ['Delete…', 'Manage Content Types', 'Manage…', 'Reset…'],
  'src/components/ContentExtractorManagerDialog.tsx': ['Reset…'],
  'src/components/ExtractorRecipeEditor.tsx': ['Choose…'],
  'src/components/ExtractorRegistryPanel.tsx': ['Delete…'],
  'src/components/ContentTypeManagerDialog.tsx': ['Manage Content Type Groups', 'Manage…'],
  'src/components/SettingsGeneralRetentionSections.tsx': ['Delete All Clips…', 'Trash All Clips…'],
  'src/components/SettingsSyncLibrarySection.tsx': ['Move…'],
  'src/components/SettingsSyncExportSection.tsx': ['Export…'],
  'src/components/SettingsSyncImportSection.tsx': ['Import…', 'Recover…'],
  'src/components/SettingsResetPanel.tsx': ['Reset Pasted…'],
  'src/components/FactoryResetDialog.tsx': ['Create Full Backup…'],
})) {
  const source = fs.readFileSync(file, 'utf8');
  for (const label of labels) {
    const matchingKeys = Object.entries(englishCatalog)
      .filter(([, value]) => typeof value === 'string' && (value === label || value.includes(label)))
      .map(([key]) => key);
    assert.ok(matchingKeys.some((key) => source.includes(`'${key}'`)),
      `${file} must show localized “${label}” before opening follow-up UI`);
  }
}
const settingsDestinations = [
  ['general', 'General'],
  ['security', 'Security'],
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
  const entry = `{ id: '${id}', get label()`;
  const index = settingsTabs.indexOf(entry);
  assert.ok(index > previousSettingsTab, `Settings destination ${label} must retain its requested order`);
  assert.ok(settingsTabs.includes(`translate('component.settingsTabs.${id === 'app-exclusions' ? 'appExclusions' : id}')`),
    `Settings destination ${label} must use its catalog entry`);
  previousSettingsTab = index;
}
assert.doesNotMatch(settingsTabs, /id: '(?:features|connections|blacklist)'/,
  'Settings destination IDs must match current product terminology');
for (const file of [
  'src/components/SettingsGeneralPanel.tsx',
  'src/components/SettingsFeaturesPanel.tsx',
  'src/components/SettingsHotkeysPanel.tsx',
  'src/components/SettingsNotificationsPanel.tsx',
  'src/components/SettingsBlacklistPanel.tsx',
  'src/components/SettingsSyncPanel.tsx',
  'src/components/SettingsAnalysisPanel.tsx',
  'src/components/IntelligenceConnectionsPanel.tsx',
  'src/components/SettingsAboutPanel.tsx',
]) {
  const source = fs.readFileSync(file, 'utf8');
  assert.match(source, /<div className="space-y-5(?:\s|\")/,
    `${file} must use the shared 20px Settings panel rhythm`);
}
assert.match(settingsHotkeys, /theme-divider flex items-center justify-between gap-3 border-b p-2\.5 last:border-b-0/,
  'Hotkey rows must render as divided rows instead of individual wells');
assert.ok((settingsHotkeys.match(/theme-surface overflow-hidden rounded-xl border/g) ?? []).length >= 3,
  'Hotkey categories must group rows into shared wells');
const destinationCopyFiles = [
  'src/components/HelpView.tsx',
  ...fs.readdirSync('docs', { recursive: true })
    .filter((name) => typeof name === 'string' && name.endsWith('.md'))
    .map((name) => `docs/${name}`),
];
for (const file of destinationCopyFiles) {
  const source = fs.readFileSync(file, 'utf8');
  assert.doesNotMatch(
    source,
    /Settings → (?:Connections|Blacklist|Detection|Features)\b/,
    `${file} must use current Settings destination names`,
  );
}
for (const title of ['Database Location', 'Export', 'Import']) {
  assert.ok(localizedValueIsUsed(settingsImportExport, title),
    `Storage titles must retain title case: ${title}`,
  );
}
for (const dataLabel of ['Clips', 'Organization', 'Activity', 'Settings and Application Data', 'Revisions and Automation History', 'Interface and Window State']) {
  assert.ok(localizedValueIsUsed(settingsImportExport, dataLabel),
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
    file: 'src/components/sidebarNavigationModel.tsx',
    source: fs.readFileSync('src/components/sidebarNavigationModel.tsx', 'utf8').match(/const allToolNavItems[\s\S]*?\n  \];/)?.[0] ?? '',
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
  assert.ok(localizedValueIsUsed(transformationTabs, title),
    `The Transformations ${title} tab must retain its localized title when labels collapse to icons`);
}
assert.equal((transformationTabs.match(/role="tab"/g) ?? []).length, 3,
  'Each Transformations workspace must remain a labeled tab');

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
const localizedSettingsColonViolations = Object.entries(englishCatalog)
  .filter(([key, value]) => key.startsWith('component.settings') && typeof value === 'string' && value.endsWith(':'));
assert.deepEqual(localizedSettingsColonViolations, [],
  'Localized structural Settings labels must not end with colons');
assert.equal(englishCatalog['component.settingsSecurityPanel.unlockUsingMethod'], 'Unlock using {method}',
  'Settings labels must preserve sentence-case capitalization');

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
