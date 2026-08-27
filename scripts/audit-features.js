import assert from 'node:assert/strict';
import fs from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const read = (path) => fs.readFileSync(path, 'utf8');
const frontendRegistry = read('src/utils/features.ts');
const settingsType = [
  'src/appSettingsTypes.ts',
  'src/appSettingsTypes/functionality.ts',
].map(read).join('\n');
const settingsHook = read('src/hooks/useAppSettings.ts');
const settingsModel = read('src/appSettingsModel.ts');
const settingsContract = JSON.parse(read('shared/settings-contract.json'));
const appTheme = read('src/utils/appTheme.ts');
const nativePolicy = read('src-tauri/src/features.rs');
const nativeRoot = read('src-tauri/src/lib.rs');
const nativeCommands = readRustModuleTree('src-tauri/src/commands.rs', 'src-tauri/src/commands');
const settingsModal = read('src/components/SettingsModal.tsx');
const settingsFeaturesPanel = read('src/components/SettingsFeaturesPanel.tsx');
const sidebar = [
  'src/components/Sidebar.tsx',
  'src/components/CollapsedSidebar.tsx',
  'src/components/SidebarBinsSection.tsx',
  'src/components/SidebarClipSection.tsx',
  'src/components/SidebarFacetSections.tsx',
  'src/components/SidebarSearchFooter.tsx',
  'src/components/SidebarToolsSection.tsx',
  'src/components/sidebarNavigationModel.tsx',
  'src/hooks/useSidebarFacets.ts',
  'src/hooks/useSidebarHoverState.ts',
].map(read).join('\n');
const nativeMenu = read('src-tauri/src/app_menu.rs');
const captureFeedbackWindow = read('src/components/CaptureFeedbackWindow.tsx');
const clipboardMonitor = readRustModuleTree(
  'src-tauri/src/clipboard_monitor.rs',
  'src-tauri/src/clipboard_ingestion',
);
const hotkeyManager = readRustModuleTree(
  'src-tauri/src/hotkey_manager.rs',
  'src-tauri/src/hotkey_manager',
);
const manualTransformService = read('src-tauri/src/manual_transform_service.rs');
const clipPreview = [
  'src/components/ClipPreview.tsx',
  'src/components/ClipPreviewHeader.tsx',
  'src/components/ClipPreviewNotesPanel.tsx',
  'src/components/ClipPreviewOrganization.tsx',
  'src/components/ClipPreviewTransformControls.tsx',
  'src/components/ClipPreviewWorkspace.tsx',
  'src/hooks/useClipPreviewAnalysis.ts',
  'src/hooks/useClipPreviewNotes.ts',
  'src/hooks/useClipPreviewRevisions.ts',
  'src/hooks/useClipPreviewTransforms.ts',
].map(read).join('\n');
const clipNameDialog = read('src/components/ClipNameDialog.tsx');
const appDialogLayout = read('src/components/AppDialogLayout.tsx');
const clipPreviewBanner = read('src/components/ClipPreviewBanner.tsx');
const quickHud = read('src/components/QuickHudWindow.tsx');
const hudEntry = read('src/hud-main.tsx');
const settingsHotkeys = read('src/components/SettingsHotkeysPanel.tsx');
const cli = readRustModuleTree('src-tauri/src/bin/pasted.rs', 'src-tauri/src/cli');
const appUpdates = read('src-tauri/src/app_updates.rs');
const frontendDefinitions = frontendRegistry.match(/export const FEATURE_DEFINITIONS[\s\S]*?\n\] as const;/)?.[0] ?? '';

const frontendKeys = [...frontendRegistry.matchAll(/settingKey:\s*'(enable[A-Za-z]+)'/g)]
  .map((match) => match[1]);
const nativeKeys = [...nativePolicy.matchAll(/=>\s*"(enable[A-Za-z]+)"/g)]
  .map((match) => match[1]);

assert.equal(frontendKeys.length, 27, 'The frontend feature registry must include every supported capability');
const frontendGroups = [...frontendRegistry.matchAll(/group:\s*'([A-Za-z]+)'/g)]
  .map((match) => match[1]);
assert.equal(frontendGroups.length, frontendKeys.length, 'Every feature must belong to a Functionality group');
assert.deepEqual(
  [...new Set(frontendGroups)].sort(),
  ['app', 'discovery', 'library', 'workflow'],
  'Functionality must keep the expected feature groups',
);
const expectedFeatureLayout = {
  library: ['bins', 'naming', 'notes', 'pinning', 'protection', 'concealment', 'trash', 'revisions'],
  discovery: ['clipTypes', 'types', 'contentClassification', 'fileFormats', 'ocr', 'transcriptions', 'sources', 'search', 'analytics'],
  workflow: ['queue', 'transformations', 'hud', 'hotkeys'],
  app: ['notifications', 'appLock', 'activityLog', 'cli', 'help', 'updates'],
};
for (const [group, expectedIds] of Object.entries(expectedFeatureLayout)) {
  const actualIds = [...frontendDefinitions.matchAll(new RegExp(
    `id:\\s*'([A-Za-z]+)'[\\s\\S]{0,160}?group:\\s*'${group}'`,
    'g',
  ))].map((match) => match[1]);
  assert.deepEqual(actualIds, expectedIds, `${group} Functionality cards must retain their intentional pairings`);
}
assert.match(
  settingsFeaturesPanel,
  /FEATURE_GROUPS\.map\(\(group\)/,
  'Settings → Functionality must render features in their logical groups',
);
assert.match(
  settingsFeaturesPanel,
  /useLocalization\(\)/,
  'Settings → Functionality must react to language changes',
);
for (const featureId of [...frontendDefinitions.matchAll(/id:\s*'([A-Za-z]+)'/g)].map((match) => match[1])) {
  assert.match(
    settingsFeaturesPanel,
    new RegExp(`feature\\.${featureId}\\.label`),
    `${featureId} must have a localized Functionality label`,
  );
  assert.match(
    settingsFeaturesPanel,
    new RegExp(`feature\\.${featureId}\\.description`),
    `${featureId} must have a localized Functionality description`,
  );
}
const toolOrder = ['transformations', 'analytics', 'activity', 'help', 'settings'];
let previousToolIndex = -1;
for (const tab of toolOrder) {
  const index = sidebar.indexOf(`{ tab: '${tab}'`, previousToolIndex + 1);
  assert.ok(index > previousToolIndex, `Tools must keep ${tab} in the intended navigation order`);
  previousToolIndex = index;
}
assert.ok(
  nativeMenu.indexOf('item(&transforms_menu)') < nativeMenu.indexOf('text("view.analytics", t("native.tools.insights"))')
    && nativeMenu.indexOf('text("view.analytics", t("native.tools.insights"))') < nativeMenu.indexOf('text("view.activity", t("native.tools.activity"))'),
  'The native Tools menu must mirror Transformations, Insights, and Activity order',
);
assert.deepEqual(
  [...new Set(nativeKeys)].sort(),
  [...new Set(frontendKeys)].sort(),
  'Frontend and native feature setting keys must stay in sync',
);

for (const key of frontendKeys) {
  assert.match(settingsType, new RegExp(`\\b${key}\\??:\\s*boolean`), `${key} must be typed in AppSettings`);
  assert.equal(settingsContract.settings.find((setting) => setting.key === key)?.default, true,
    `${key} must default on for existing installations`);
  assert.match(settingsModel, new RegExp(`\\b${key}:\\s*settingDefault\\('${key}'\\)`),
    `${key} must read its default from the shared settings contract`);
  assert.match(settingsModel, new RegExp(`(?:['\"]${key}['\"]|saved\\.${key})`), `${key} must hydrate from persisted settings`);
}

assert.match(nativeRoot, /pub mod features;/, 'The native policy must be shared with the CLI crate');
assert.doesNotMatch(
  read('src/components/SettingsGeneralPanel.tsx'),
  /onUpdateSettings\(\{\s*enable(?:Trash|ActivityLog):/,
  'Feature switches belong only on Settings → Functionality',
);

assert.match(
  nativeCommands,
  /"app-setting-changed"/,
  'Native settings writes must notify every open window',
);
assert.match(
  settingsHook,
  /listen<AppSettingChangedEvent>\(APP_EVENTS\.appSettingChanged/,
  'Each window must synchronize settings changed elsewhere',
);
assert.match(
  settingsModal,
  /settings\.enableNotifications && activeTab === 'notifications'/,
  'The Notifications feature must own its Settings surface',
);
assert.match(sidebar, /id: 'clipTypes'/,
  'Clip Types must retain their sidebar collection surface');
assert.match(sidebar, /features\[section\.id\] && section\.items\.length > 0/,
  'Every sidebar facet collection must honor its matching feature gate');
assert.match(
  sidebar,
  /features\.search && \(?[\s\S]{0,80}<SidebarSearchFooter/,
  'Clip Search must own the sidebar search surface',
);
assert.match(
  cli,
  /fn run_search[\s\S]{0,1800}Feature::Search/,
  'The explicit CLI search command must honor the Clip Search feature gate',
);
assert.match(
  cli,
  /matches!\(command, "update" \| "updates"\)[\s\S]{0,200}Feature::Updates/,
  'CLI update checks must honor the Software Updates feature gate before contacting GitHub',
);
assert.match(
  appUpdates,
  /check_for_app_update[\s\S]{0,500}features::require\(&db, Feature::Updates\)/,
  'GUI update checks must honor the Software Updates feature gate before contacting GitHub',
);
assert.match(
  nativeMenu,
  /feature_enabled\(Feature::Search\)[\s\S]{0,100}clips_builder = clips_builder\.item\(&search\)/,
  'Clip Search must own the native Search menu item and shortcut',
);
assert.match(
  quickHud,
  /features\.search && <div className="relative flex-1">/,
  'Clip Search must own the Quick HUD search field',
);
assert.match(
  hudEntry,
  /<FeatureProvider features=\{features\}><QuickHudWindow/,
  'The dedicated HUD entry point must retain the shared feature policy',
);
assert.match(
  quickHud,
  /hudPasteShortcutIndex\(e\)/,
  'HUD positional paste must use the tested primary-modifier shortcut contract',
);
assert.match(
  quickHud,
  /<ClipImageThumbnail[\s\S]{0,160}clipId=\{clip\.id\}/,
  'Image search results in the HUD must load their safe thumbnail independently',
);
assert.match(
  settingsModal,
  /searchEnabled=\{settings\.enableSearch\}/,
  'Clip Search must own the Analysis Index manager surface',
);
assert.match(
  captureFeedbackWindow,
  /currentSettings\.enableNotifications/,
  'The capture feedback window must honor the shared Notifications feature gate',
);
assert.match(
  clipboardMonitor,
  /Feature::Notifications/,
  'Clipboard capture must suppress notification events at the native policy boundary',
);
assert.match(
  hotkeyManager,
  /Feature::Hotkeys[\s\S]{0,500}state:\s*"disabled"/,
  'The native hotkey boundary must unregister and report disabled state when Hotkeys is off',
);
assert.match(
  hotkeyManager,
  /PasteClipById\(clip_id\) =>[\s\S]{0,900}PasteOrigin::ClipHotkey/,
  'Direct clip hotkeys must not depend on the HUD feature gate',
);
assert.match(hotkeyManager, /get_all_settings\(\)/,
  'Hotkey rebuilds must load settings in one database snapshot');
assert.match(hotkeyManager, /get_bin_hotkeys\(\)/,
  'Hotkey rebuilds must not load full Bin records');
assert.match(hotkeyManager, /manual_transform_service::hotkeys\(&db\)/,
  'Hotkey rebuilds must enter through the Manual Transform application service');
assert.match(manualTransformService, /db\.get_pipeline_hotkeys\(\)/,
  'The Manual Transform service must isolate the historical hotkey storage API');
assert.doesNotMatch(
  nativeCommands.match(/pub\(crate\) fn execute_clipboard_pipeline[\s\S]*?\n\}/)?.[0] ?? '',
  /thread::spawn/,
  'Transform hotkeys must retain clipboard serialization until the synthetic paste fires',
);
assert.doesNotMatch(settingsHotkeys, /setInterval\(/,
  'Hotkey Settings status must remain event-driven instead of polling');
assert.match(
  settingsModal,
  /settings\.enableHotkeys && activeTab === 'hotkeys'/,
  'The Hotkeys feature must own its Settings surface',
);
assert.match(
  clipPreview,
  /features\.protection && features\.hotkeys/,
  'Clip hotkey assignment must honor both Protection and Hotkeys',
);
assert.match(
  clipNameDialog,
  /<form[\s\S]{0,300}onSubmit=[\s\S]{0,500}onSave\(clip/,
  'The clip Name dialog must submit from the form so Enter saves',
);
assert.match(
  clipNameDialog,
  /AppDialogButton type="submit" variant="primary"/,
  'The clip Name dialog Save action must remain primary',
);
assert.doesNotMatch(
  appDialogLayout,
  /\bSave\b[^\n]*from 'lucide-react'|<Save\b/,
  'Shared labeled Save buttons must not add a disk icon to every dialog flow',
);
assert.match(
  clipPreviewBanner,
  /floating-action-strip[\s\S]*?<X \/>[\s\S]*?<Save \/>/,
  'The icon-only preview actions must retain their neighboring Cancel and Save icons',
);
assert.match(
  cli,
  /"hotkey" => \{[\s\S]{0,160}Feature::Hotkeys/,
  'CLI hotkey mutations must honor the Hotkeys feature gate',
);
assert.match(
  appTheme,
  /root\.dataset\.theme = resolvedTheme/,
  'Synchronized appearance settings must update semantic theme tokens',
);

console.log(`Feature capability audit passed for ${frontendKeys.length} shared gates.`);
