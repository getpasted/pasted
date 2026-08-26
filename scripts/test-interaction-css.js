/**
 * Regression contracts for the cursor and selection rules that prevent WebKit
 * from resolving competing cursors while a pointer crosses nested UI nodes.
 */
import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const accessibility = read('src/styles/accessibility.css');
const sidebar = read('src/styles/clips-sidebar.css');
const theme = read('src/styles/theme-primitives.css');
const foundation = read('src/styles/foundation.css');
const utilities = read('src/styles/utilities.css');
const manualTransformEditor = [
  'src/components/ManualTransformEditorModal.tsx',
  'src/components/ManualTransformStepEditor.tsx',
].map(read).join('\n');
const reorderHook = read('src/hooks/useStableVerticalReorder.ts');
const sidebarComponent = [
  'src/components/Sidebar.tsx',
  'src/components/CollapsedSidebar.tsx',
  'src/components/SidebarBinsSection.tsx',
  'src/components/SidebarSearchFooter.tsx',
  'src/hooks/useSidebarHoverState.ts',
].map(read).join('\n');
const app = [read('src/App.tsx'), read('src/components/AppShellView.tsx')].join('\n');
const appDialog = read('src/components/AppDialog.tsx');
const appShell = read('src/hooks/useAppShell.ts');
const settingsPanelHeader = read('src/components/SettingsPanelHeader.tsx');
const connectedMenuAction = read('src/components/ConnectedMenuAction.tsx');
const previewNotes = read('src/styles/preview-notes.css');
const allStyleSources = fs.readdirSync('src/styles')
  .filter((file) => file.endsWith('.css'))
  .map((file) => read(`src/styles/${file}`))
  .join('\n');
const allComponentSources = fs.readdirSync('src/components')
  .filter((file) => file.endsWith('.tsx'))
  .map((file) => read(`src/components/${file}`))
  .join('\n');
const settingsSearchHistory = read('src/components/SettingsSearchHistoryPanel.tsx');
const settingsAbout = read('src/components/SettingsAboutPanel.tsx');
const settingsOcr = read('src/components/SettingsOcrPanel.tsx');
const settingsLoadingState = read('src/components/SettingsLoadingState.tsx');
const settingsNavigationCard = read('src/components/SettingsNavigationCard.tsx');
const libraryTransitionDialog = read('src/components/LibraryTransitionDialog.tsx');
const settingsGeneralRetentionSections = read('src/components/SettingsGeneralRetentionSections.tsx');
const clipListHeader = read('src/components/ClipListHeader.tsx');
const activityLogView = read('src/components/ActivityLogView.tsx');
const clipPreviewContent = read('src/components/ClipPreviewContent.tsx');
const clipPreviewTransformControls = read('src/components/ClipPreviewTransformControls.tsx');
const clipTransformBar = read('src/components/ClipTransformBar.tsx');
const helpCliInstallCard = read('src/components/HelpCliInstallCard.tsx');
const sequentialQueueBar = read('src/components/SequentialQueueBar.tsx');
const solidProductActions = [
  read('src/components/SettingsAboutPanel.tsx'),
  read('src/components/WelcomeSetup.tsx'),
  read('src/components/SettingsResetPanel.tsx'),
  read('src/components/FactoryResetDialog.tsx'),
].join('\n');

const ruleBody = (css, selector) => {
  const start = css.indexOf(selector);
  assert.notEqual(start, -1, `Missing selector: ${selector}`);
  const open = css.indexOf('{', start);
  const close = css.indexOf('}', open);
  assert.notEqual(open, -1, `Missing rule body: ${selector}`);
  assert.notEqual(close, -1, `Unclosed rule body: ${selector}`);
  return css.slice(open + 1, close);
};

const openingTags = (source, tagName) => {
  const tags = [];
  let start = source.indexOf(`<${tagName}`);
  while (start !== -1) {
    let braces = 0;
    let quote = '';
    let escaped = false;
    let end = start + tagName.length + 1;
    for (; end < source.length; end += 1) {
      const char = source[end];
      if (quote) {
        if (escaped) escaped = false;
        else if (char === '\\') escaped = true;
        else if (char === quote) quote = '';
        continue;
      }
      if (char === '"' || char === "'" || char === '`') quote = char;
      else if (char === '{') braces += 1;
      else if (char === '}') braces -= 1;
      else if (char === '>' && braces === 0) break;
    }
    tags.push(source.slice(start, end + 1).replace(/\s+/g, ' '));
    start = source.indexOf(`<${tagName}`, end + 1);
  }
  return tags;
};

// Unlayered interaction rules own the normal cursor without !important, while
// their selectors explicitly opt out during reordering.
assert.match(accessibility, /html:not\(\.is-stable-reordering\) button:not\(:disabled\)/);
assert.match(accessibility, /html:not\(\.is-stable-reordering\) input:not\(\[type="checkbox"\]\)/);
assert.match(accessibility, /html:not\(\.is-stable-reordering\) \.clip-text-content/);
assert.doesNotMatch(accessibility, /!important/);

// Manual Transform steps expose explicit keyboard-accessible ordering controls instead
// of making the entire editor card a pointer-only drag target.
assert.match(manualTransformEditor, /aria-label=\{translate\('component\.pipelineEditorModal\.moveStepUp'\)\}/);
assert.match(manualTransformEditor, /aria-label=\{translate\('component\.pipelineEditorModal\.moveStepDown'\)\}/);
assert.match(manualTransformEditor, /handleMoveStep\(index, -1\)/);
assert.match(manualTransformEditor, /handleMoveStep\(index, 1\)/);
assert.doesNotMatch(manualTransformEditor, /data-stable-reorder-id|onReorderPointerDown|cursor-grab|GripVertical/);

// Settings action clusters wrap before they can collapse headings or
// descriptions at narrow widths and larger user-selected text sizes.
assert.match(settingsPanelHeader, /settings-section-header flex flex-wrap/);
assert.match(settingsPanelHeader, /min-w-\[min\(16rem,100%\)\] flex-1/);
assert.match(settingsPanelHeader, /max-w-full flex-wrap/);

// Connected menu/action controls use shared square mating edges, and selected
// menu items use an inset ring so neither treatment changes layout geometry.
assert.match(connectedMenuAction, /connected-menu-action/);
assert.match(ruleBody(foundation, '.connected-menu-action > button:first-child {'), /border-start-end-radius:\s*0;/);
assert.match(ruleBody(foundation, '.connected-menu-action > button:last-child {'), /border-start-start-radius:\s*0;/);
assert.match(ruleBody(theme, '.theme-menu-item.is-selected {'), /box-shadow:\s*inset 0 0 0 1px/);

// Resize mode disables the whole app except the captured divider.
assert.match(ruleBody(sidebar, '.is-resizing-columns {'), /cursor:\s*col-resize;/);
assert.match(ruleBody(sidebar, '.is-resizing-columns * {'), /pointer-events:\s*none;/);
const resizerBody = ruleBody(sidebar, '.is-resizing-columns .column-resizer,');
assert.match(resizerBody, /pointer-events:\s*auto;/);
assert.match(resizerBody, /cursor:\s*col-resize;/);
assert.match(app, /isResizingSidebar \|\| isResizingList \? 'is-resizing-columns'/);

// Clip navigation drop targets retain the same semantic colors as their icons.
for (const [action, token] of [
  ['queue', '--queue-accent'],
  ['pin', '--status-success'],
  ['protect', '--accent-primary'],
  ['conceal', '--status-warning'],
  ['trash', '--status-danger'],
]) {
  assert.match(ruleBody(sidebar, `.sidebar-action-drop-${action} {`),
    new RegExp(`--sidebar-action-drop-color:\\s*var\\(${token}\\);`));
}

// Stable reordering owns cursor and selection until its hook removes the class.
const reorderBody = ruleBody(sidebar, 'html.is-stable-reordering,');
assert.match(reorderBody, /cursor:\s*grabbing;/);
assert.match(reorderBody, /user-select:\s*none;/);
assert.match(reorderHook, /classList\.add\('is-stable-reordering'\)/);
assert.match(reorderHook, /classList\.remove\('is-stable-reordering'\)/);

// Bin reordering restores the JS-managed hover immediately after settling;
// clip dragging keeps its separate post-drag suppression behavior.
assert.match(sidebarComponent, /wasBinReorderingRef\.current && !isBinReorderActive/);
assert.match(sidebarComponent, /elementFromPoint\(pointer\.x, pointer\.y\)/);
assert.match(sidebarComponent, /wasClipDraggingRef\.current && !isClipDragging/);

// The pinned shelf exists while its leave animation settles, but its invisible
// stack must never block the first clip row.
assert.match(ruleBody(sidebar, '.pinned-clip-shelf-stack {'), /pointer-events:\s*none;/);
assert.match(ruleBody(sidebar, '.pinned-clip-shelf.is-visible .pinned-clip-shelf-stack {'), /pointer-events:\s*auto;/);

// Main navigation typography must remain root-relative so the General text-size
// preference scales labels along with the rest of the application.
assert.match(sidebarComponent, /sidebar-scroll-container[^\"]*text-\[0\.8125rem\]/);
assert.doesNotMatch(sidebarComponent, /sidebar-scroll-container[^\"]*text-\[13px\]/);

// Scrollable menus, panels, and wells reveal a non-layout-shifting thumb only
// while the shared document listener marks them as actively scrolling.
assert.match(appShell, /TRANSIENT_SCROLL_SURFACE_SELECTOR/);
assert.match(appShell, /event\.composedPath\(\)\.find/);
assert.match(appShell, /addEventListener\('wheel', handleSurfaceWheel/);
assert.match(utilities, /\.surface-scroll-region/);
assert.match(utilities, /\.theme-panel/);
assert.match(utilities, /\.theme-surface/);
assert.match(utilities, /\.is-scrolling::\-webkit-scrollbar-thumb/);

// Semantic action families own one consistent keyboard focus treatment.
assert.match(theme, /\.theme-primary-button,[\s\S]*\.clip-preview-action[\s\S]*:focus-visible/);
assert.match(theme, /outline:\s*2px solid var\(--focus-ring\)/);

// Destructive toolbar actions use the standard-height outlined danger variant
// instead of assembling shorter one-off status buttons.
assert.match(settingsSearchHistory, /<ActionButton variant="danger" onClick=\{requestClear\}/);
assert.match(clipListHeader, /<ActionButton[\s\S]{0,80}variant="danger"[\s\S]{0,120}onClick=\{requestEmptyTrash\}/);
assert.match(settingsGeneralRetentionSections, /<ActionButton variant="danger" onClick=\{\(event\) => onClearHistory\?\.\(event\.altKey\)\}/);
assert.match(clipListHeader, /translate\('app\.emptyTrashEllipsis'\)/);
assert.match(clipListHeader, /<ConfirmationDialog request=\{emptyTrashRequest\}/);
assert.doesNotMatch(clipListHeader, /\bTrash2\b/);

// Static status surfaces communicate state; interactive controls must use a
// semantic button family that owns hover, focus, disabled, and active states.
assert.doesNotMatch(
  allComponentSources,
  /<button\b[^>]*className="[^"]*\btheme-status-(?:info|success|warning|danger)\b[^"]*"[^>]*>/s,
);

// Every raw button opts into an interaction family with a reviewed hover
// treatment. This prevents visually static one-off buttons from bypassing the
// shared action primitives while preserving purpose-built controls and menus.
const interactiveButtonFamily = /(?:app-dialog-(?:button|close)|theme-(?:icon|primary|secondary)-button|theme-menu-item|theme-(?:input|inline)-action|theme-interactive-card|floating-action-button|batch-action-button|sidebar-(?:row-action|control-muted|nav-row|add-btn|item-(?:active|hovered|idle))|list-toolbar-button|smart-action-button|transform-workspace-(?:action|tab)|playground-run-status-action|clip-preview-action|clip-format-button|clip-revision-(?:count|history-(?:close|load-more|item))|preview-name-row|active-filter-reset|appearance-mode-button|help-topic-button|settings-(?:tab|feature-preset|switch)|queue-action-(?:primary|secondary)|menu-select-trigger|theme-toggle|theme-checkbox|mac-window-control|note-(?:save|cancel)-button|connection-loading-action|connected-menu-action|hotkey-recorder-(?:trigger|clear)|info-popover-trigger|pinned-clip-shelf-card|ocr-status-card|app-toast-close|welcome-setup-skip|hover:)/;
for (const button of openingTags(allComponentSources, 'button')) {
  assert.match(button, interactiveButtonFamily, `Button lacks a reviewed interaction family: ${button}`);
}
const roleButtons = openingTags(allComponentSources, 'div').filter((tag) => /\brole="button"/.test(tag));
assert.equal(roleButtons.length, (allComponentSources.match(/\brole="button"/g) || []).length);
for (const roleButton of roleButtons) {
  assert.match(roleButton, interactiveButtonFamily, `role="button" control lacks a reviewed interaction family: ${roleButton}`);
  assert.match(roleButton, /\btabIndex=/, `role="button" control is not keyboard focusable: ${roleButton}`);
  assert.match(roleButton, /\bonKeyDown=/, `role="button" control lacks keyboard activation: ${roleButton}`);
}
assert.match(theme, /\.theme-interactive-card:hover:not\(:disabled\)/);
assert.match(theme, /\.theme-input-action:hover:not\(:disabled\)/);
assert.match(theme, /\.theme-inline-action:hover:not\(:disabled\)/);
assert.match(theme, /\.settings-switch:hover:not\(:disabled\):not\(\.is-on\)/);

// Solid semantic fills are reserved for the product-wide backing and reset
// actions. Ordinary actions remain outlined, including neutral and warning.
assert.equal((solidProductActions.match(/variant="solid-primary"/g) || []).length, 2);
assert.equal((solidProductActions.match(/variant="solid-danger"/g) || []).length, 2);
assert.match(ruleBody(theme, '.app-dialog-button.is-secondary {'), /background-color:\s*transparent;/);
assert.match(ruleBody(theme, '.app-dialog-button.is-primary {'), /background-color:\s*transparent;/);
assert.match(ruleBody(theme, '.app-dialog-button.is-warning {'), /background-color:\s*transparent;/);
assert.match(ruleBody(theme, '.app-dialog-button.is-danger {'), /background-color:\s*transparent;/);
assert.match(ruleBody(theme, '.theme-secondary-button {'), /background-color:\s*transparent;/);
assert.match(ruleBody(theme, '.app-dialog-button.is-solid-primary {'), /background-color:\s*var\(--accent-primary\);/);
assert.match(ruleBody(theme, '.app-dialog-button.is-solid-danger {'), /background-color:\s*var\(--status-danger\);/);

// Hover may change semantic color, border, or opacity, but controls must not
// rise or grow under the pointer. Structural reveal/layout motion is separate.
assert.doesNotMatch(
  allStyleSources,
  /:hover[^\{]*\{[^\}]*transform:\s*[^;]*(?:translateY\(\s*-[1-9]|scale\(\s*1\.)/s,
);
assert.doesNotMatch(allComponentSources, /\bhover:(?:-translate-y|scale)-/);
assert.doesNotMatch(allComponentSources, /\bactive:(?:-translate-y|scale)-/);
assert.doesNotMatch(theme, /\.app-dialog-button:active[^\{]*\{[^\}]*transform:/s);

// Keyboard focus is a global semantic guarantee. Component families may add
// emphasis, but cannot erase the shared ring without replacing it.
assert.match(accessibility, /:where\(button, \[role="button"\], a\[href\]\):focus-visible/);
assert.match(ruleBody(accessibility, ':where(button, [role="button"], a[href]):focus-visible {'), /outline:\s*2px solid var\(--focus-ring\);/);
for (const source of [sidebar, theme, previewNotes]) {
  assert.doesNotMatch(source, /(?:button|\[role="button"\]|-action|-trigger|-item|-card)[^\{]*:focus-visible[^\{]*\{[^\}]*outline:\s*none/s);
}

// Depth is assigned by semantic role so it follows every theme and never
// changes layout geometry. Tailwind shadow utilities are not product tokens.
for (const token of [
  '--elevation-control',
  '--elevation-raised',
  '--elevation-floating',
  '--elevation-modal',
  '--elevation-inset',
  '--elevation-object',
]) assert.match(foundation, new RegExp(`${token}:`));
for (const role of ['control', 'raised', 'floating', 'modal', 'inset', 'object']) {
  assert.match(ruleBody(utilities, `.elevation-${role} {`), new RegExp(`box-shadow:\\s*var\\(--elevation-${role}\\);`));
}
assert.match(ruleBody(theme, '.theme-panel,'), /box-shadow:\s*var\(--elevation-raised\);/);
assert.match(ruleBody(theme, '.floating-action-strip {'), /box-shadow:\s*var\(--elevation-floating\);/);
assert.match(appDialog, /className=\{`app-dialog-panel elevation-modal/);
assert.doesNotMatch(
  allComponentSources,
  /(?:^|\s)shadow(?:-(?:sm|md|lg|xl|2xl|inner|none))?(?=\s|["'`}])/m,
);

// Selected tabs and segmented choices communicate state with a quiet tint and
// border instead of borrowing the solid product-action treatment or elevation.
for (const selector of [
  '.settings-tab.is-active {',
  '.settings-feature-preset.is-active {',
  '.appearance-mode-button.is-active {',
]) {
  const selectedControl = ruleBody(theme, selector);
  assert.match(selectedControl, /background-color:\s*color-mix\(/);
  assert.match(selectedControl, /border-color:\s*color-mix\(/);
  assert.doesNotMatch(selectedControl, /box-shadow:/);
}
const smartActionButton = ruleBody(theme, '.smart-action-button {');
assert.match(smartActionButton, /height:\s*2rem;/);
assert.match(smartActionButton, /min-height:\s*2rem;/);
assert.match(smartActionButton, /font-size:\s*0\.75rem;/);
assert.match(previewNotes, /\.note-save-button \{\s*background-color:\s*transparent;/);
assert.match(previewNotes, /\.note-save-button \{[\s\S]*?border:\s*1px solid color-mix\(/);
assert.match(ruleBody(previewNotes, '.note-cancel-button {'), /background-color:\s*transparent;/);
assert.match(ruleBody(previewNotes, '.note-cancel-button {'), /border:\s*1px solid var\(--border-input\);/);
assert.doesNotMatch(allComponentSources, /note-save-button[^"\n]*\bshadow\b/);

// Settings use shared, theme-safe affordances for asynchronous states and
// cards that navigate to another settings destination.
assert.match(settingsLoadingState, /role="status"/);
assert.match(settingsLoadingState, /aria-busy="true"/);
assert.match(settingsLoadingState, /aria-live="polite"/);
for (const panel of [settingsAbout, settingsOcr, settingsSearchHistory]) {
  assert.match(panel, /<SettingsLoadingState\b/);
}
assert.match(settingsNavigationCard, /theme-card-idle theme-interactive-card/);
assert.match(settingsNavigationCard, /rtl:-scale-x-100/);
assert.match(settingsAbout, /<SettingsNavigationCard\b/);
assert.doesNotMatch(settingsAbout, /theme-card-idle theme-interactive-card/);

// Search history remains bounded even when retention is Unlimited: navigation
// replaces one fixed-size page rather than appending an unbounded list.
assert.match(settingsSearchHistory, /const PAGE_SIZE = 50;/);
assert.match(settingsSearchHistory, /setEntries\(page\.items\);/);
assert.doesNotMatch(settingsSearchHistory, /setEntries\([^\n]*\.\.\.current/);
assert.match(settingsSearchHistory, /offset \+ entries\.length >= totalCount/);
assert.match(settingsSearchHistory, /settingsSearchHistoryPanel\.showingRange/);
assert.match(settingsSearchHistory, /settingsSearchHistoryPanel\.previousPage/);
assert.match(settingsSearchHistory, /settingsSearchHistoryPanel\.nextPage/);

// Product action controls share the 32px geometry and flat outlined treatment;
// domain accents may strengthen a border, but do not introduce solid fills.
const transformAction = ruleBody(theme, '.transform-workspace-action {');
assert.match(transformAction, /height:\s*2rem;/);
assert.match(transformAction, /background-color:\s*transparent;/);
assert.doesNotMatch(ruleBody(theme, '.transform-workspace-action.manual-transforms {'), /background-color:/);
assert.match(clipPreviewTransformControls, /className="transform-workspace-action manual-transforms"/);
assert.match(clipTransformBar, /className="transform-workspace-action manual-transforms"/);

const queueActions = ruleBody(theme, '.queue-action-secondary,');
assert.match(queueActions, /height:\s*2rem;/);
assert.match(queueActions, /background-color:\s*transparent;/);
assert.match(sequentialQueueBar, /className="queue-action-secondary"/);
assert.match(sequentialQueueBar, /className="queue-action-primary"/);

const noteActions = ruleBody(previewNotes, '.note-cancel-button,');
assert.match(noteActions, /height:\s*2rem;/);
assert.match(noteActions, /min-height:\s*2rem;/);

assert.match(helpCliInstallCard, /<ActionButton variant="primary"/);
assert.doesNotMatch(helpCliInstallCard, /shadow/);
assert.equal((clipPreviewContent.match(/theme-primary-button theme-focusable ui-control-radius grid h-8 w-8/g) || []).length, 2);
assert.doesNotMatch(clipPreviewContent, /theme-primary-button[^"\n]*\bshadow\b/);
assert.match(activityLogView, /<ActionButton\s+variant="danger"[\s\S]{0,160}setIsClearConfirmOpen/);

// Retired one-off transform fills must not return after action normalization.
assert.doesNotMatch(`${foundation}\n${theme}`, /--(?:manual-transform|operation)-action|\.preview-filter-reset/);

// Reduced-motion rules must continue to defeat component-level animation.
const reducedMotionStart = theme.indexOf('@media (prefers-reduced-motion: reduce)');
assert.notEqual(reducedMotionStart, -1, 'Missing reduced-motion media query');
const reducedMotion = theme.slice(reducedMotionStart, theme.indexOf('\n}', theme.indexOf('\n}', reducedMotionStart) + 2) + 2);
assert.match(reducedMotion, /animation-duration:\s*0\.01ms !important;/);
assert.match(reducedMotion, /animation-iteration-count:\s*1 !important;/);
assert.match(reducedMotion, /transition-duration:\s*0\.01ms !important;/);
assert.match(reducedMotion, /scroll-behavior:\s*auto !important;/);
assert.match(libraryTransitionDialog, /matchMedia\('\(prefers-reduced-motion: reduce\)'\)/);

// Every remaining override must belong to this exact accessibility allowlist.
const importantLines = [
  ['accessibility', accessibility],
  ['sidebar', sidebar],
  ['theme', theme],
  ['foundation', foundation],
  ['preview', previewNotes],
].flatMap(([name, source]) => source
  .split('\n')
  .filter((line) => line.includes('!important'))
  .map((line) => `${name}:${line.trim()}`));

assert.deepEqual(importantLines, [
  'theme:scroll-behavior: auto !important;',
  'theme:animation-duration: 0.01ms !important;',
  'theme:animation-iteration-count: 1 !important;',
  'theme:transition-duration: 0.01ms !important;',
]);

console.log('Interaction CSS contracts passed.');
