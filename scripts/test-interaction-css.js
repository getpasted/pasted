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
const pipelineEditor = read('src/components/PipelineEditorModal.tsx');
const reorderHook = read('src/hooks/useStableVerticalReorder.ts');
const sidebarComponent = read('src/components/Sidebar.tsx');
const app = read('src/App.tsx');

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
assert.match(accessibility, /html:not\(\.is-stable-reordering\) button:not\(:disabled\):not\(\.step-drag-handle\)/);
assert.match(accessibility, /html:not\(\.is-stable-reordering\) input:not\(\[type="checkbox"\]\)/);
assert.match(accessibility, /html:not\(\.is-stable-reordering\) \.clip-text-content/);
assert.doesNotMatch(accessibility, /!important/);

// The step handle no longer fights the global button selector.
assert.match(ruleBody(accessibility, 'button.step-drag-handle {'), /cursor:\s*grab;/);
assert.match(ruleBody(accessibility, 'button.step-drag-handle:active {'), /cursor:\s*grabbing;/);
assert.match(pipelineEditor, /className="[^"]*step-drag-handle[^"]*"/);

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
  ['foundation', read('src/styles/foundation.css')],
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
