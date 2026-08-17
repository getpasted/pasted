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
const pipelineEditor = read('src/components/PipelineEditorModal.tsx');
const reorderHook = read('src/hooks/useStableVerticalReorder.ts');
const sidebarComponent = read('src/components/Sidebar.tsx');
const app = read('src/App.tsx');
const settingsPanelHeader = read('src/components/SettingsPanelHeader.tsx');
const connectedMenuAction = read('src/components/ConnectedMenuAction.tsx');

const ruleBody = (css, selector) => {
  const start = css.indexOf(selector);
  assert.notEqual(start, -1, `Missing selector: ${selector}`);
  const open = css.indexOf('{', start);
  const close = css.indexOf('}', open);
  assert.notEqual(open, -1, `Missing rule body: ${selector}`);
  assert.notEqual(close, -1, `Unclosed rule body: ${selector}`);
  return css.slice(open + 1, close);
};

// Unlayered interaction rules own the normal cursor without !important, while
// their selectors explicitly opt out during reordering.
assert.match(accessibility, /html:not\(\.is-stable-reordering\) button:not\(:disabled\)/);
assert.match(accessibility, /html:not\(\.is-stable-reordering\) input:not\(\[type="checkbox"\]\)/);
assert.match(accessibility, /html:not\(\.is-stable-reordering\) \.clip-text-content/);
assert.doesNotMatch(accessibility, /!important/);

// Pipeline steps expose explicit keyboard-accessible ordering controls instead
// of making the entire editor card a pointer-only drag target.
assert.match(pipelineEditor, /aria-label=\{translate\('component\.pipelineEditorModal\.moveStepUp'\)\}/);
assert.match(pipelineEditor, /aria-label=\{translate\('component\.pipelineEditorModal\.moveStepDown'\)\}/);
assert.match(pipelineEditor, /handleMoveStep\(idx, -1\)/);
assert.match(pipelineEditor, /handleMoveStep\(idx, 1\)/);
assert.doesNotMatch(pipelineEditor, /data-stable-reorder-id|onReorderPointerDown|cursor-grab|GripVertical/);

// Settings action clusters wrap before they can collapse headings or
// descriptions at narrow widths and larger user-selected text sizes.
assert.match(settingsPanelHeader, /settings-section-header flex flex-wrap/);
assert.match(settingsPanelHeader, /min-w-\[min\(16rem,100%\)\] flex-1/);
assert.match(settingsPanelHeader, /max-w-full flex-wrap/);

// Connected menu/action controls use shared square mating edges, and selected
// menu items use an inset ring so neither treatment changes layout geometry.
assert.match(connectedMenuAction, /connected-menu-action/);
assert.match(ruleBody(foundation, '.connected-menu-action > button:first-child {'), /border-top-right-radius:\s*0;/);
assert.match(ruleBody(foundation, '.connected-menu-action > button:last-child {'), /border-top-left-radius:\s*0;/);
assert.match(ruleBody(theme, '.theme-menu-item.is-selected {'), /box-shadow:\s*inset 0 0 0 1px/);

// Resize mode disables the whole app except the captured divider.
assert.match(ruleBody(sidebar, '.is-resizing-columns {'), /cursor:\s*col-resize;/);
assert.match(ruleBody(sidebar, '.is-resizing-columns * {'), /pointer-events:\s*none;/);
const resizerBody = ruleBody(sidebar, '.is-resizing-columns .column-resizer,');
assert.match(resizerBody, /pointer-events:\s*auto;/);
assert.match(resizerBody, /cursor:\s*col-resize;/);
assert.match(app, /isResizingSidebar \|\| isResizingList \? 'is-resizing-columns'/);

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
assert.match(app, /TRANSIENT_SCROLL_SURFACE_SELECTOR/);
assert.match(app, /event\.composedPath\(\)\.find/);
assert.match(app, /addEventListener\('wheel', handleSurfaceWheel/);
assert.match(utilities, /\.surface-scroll-region/);
assert.match(utilities, /\.theme-panel/);
assert.match(utilities, /\.theme-surface/);
assert.match(utilities, /\.is-scrolling::\-webkit-scrollbar-thumb/);

// Semantic action families own one consistent keyboard focus treatment.
assert.match(theme, /\.theme-primary-button,[\s\S]*\.clip-preview-action[\s\S]*:focus-visible/);
assert.match(theme, /outline:\s*2px solid var\(--focus-ring\)/);

// Reduced-motion rules must continue to defeat component-level animation.
const reducedMotionStart = theme.indexOf('@media (prefers-reduced-motion: reduce)');
assert.notEqual(reducedMotionStart, -1, 'Missing reduced-motion media query');
const reducedMotion = theme.slice(reducedMotionStart, theme.indexOf('\n}', theme.indexOf('\n}', reducedMotionStart) + 2) + 2);
assert.match(reducedMotion, /animation-duration:\s*0\.01ms !important;/);
assert.match(reducedMotion, /transition-duration:\s*0\.01ms !important;/);

// Every remaining override must belong to this exact accessibility allowlist.
const importantLines = [
  ['accessibility', accessibility],
  ['sidebar', sidebar],
  ['theme', theme],
  ['foundation', foundation],
  ['preview', read('src/styles/preview-notes.css')],
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
