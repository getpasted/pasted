import fs from 'node:fs';

const sharedMenu = fs.readFileSync('src/components/AnchoredMenu.tsx', 'utf8');
const nativeMenu = fs.readFileSync('src-tauri/src/app_menu.rs', 'utf8');
const englishCatalog = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));
const floatingMenus = [
  'src/components/BinModal.tsx',
  'src/components/BinContextMenu.tsx',
  'src/components/ClipWorkflowMenu.tsx',
  'src/components/ContextMenu.tsx',
  'src/components/MenuSelect.tsx',
];

const failures = [];

for (const file of floatingMenus) {
  const source = fs.readFileSync(file, 'utf8');
  if (!source.includes('AnchoredMenu')) failures.push(`${file} does not use AnchoredMenu`);
  if (source.includes("createPortal") || source.includes('theme-menu fixed')) {
    failures.push(`${file} owns floating-menu portal or fixed-position behavior`);
  }
}

if (!sharedMenu.includes("window.addEventListener('pointerdown', closeOutside, true)")) {
  failures.push('AnchoredMenu outside dismissal must run before draggable surfaces stop pointer events');
}
if (!sharedMenu.includes("window.addEventListener('scroll', positionMenu, true)")) {
  failures.push('AnchoredMenu must reposition for nested scrolling containers');
}
if (!sharedMenu.includes('calc(100% - 1px)')) {
  failures.push('MenuSubmenu must overlap its parent edge to avoid a dead hover gap');
}
if (!sharedMenu.includes('position.anchorKey === anchorKey')) {
  failures.push('AnchoredMenu must hide stale coordinates until the current anchor is positioned');
}
if (!sharedMenu.includes('needsHiddenPositioningPass') || !sharedMenu.includes('requestAnimationFrame')) {
  failures.push('AnchoredMenu must reveal only after a hidden positioning frame');
}
if (!sharedMenu.includes('data-anchored-menu-measurement') || !sharedMenu.includes('key={`visible:${anchorKey}`}')) {
  failures.push('AnchoredMenu must separate its offscreen measurement node from the visible WebKit layer');
}
if ((sharedMenu.match(/surface-scroll-region/g) || []).length < 3) {
  failures.push('AnchoredMenu and its measurement/submenu surfaces must share transient-scroll geometry');
}
if (!sharedMenu.includes('onWheelCapture') || !sharedMenu.includes('onScroll')) {
  failures.push('AnchoredMenu must reveal its transient scrollbar directly during wheel and scroll input');
}
if ((sharedMenu.match(/normalizeMenuDividers\(children, MenuDivider\)/g) || []).length < 2) {
  failures.push('Anchored menus and submenus must normalize conditional dividers');
}
if (!fs.readFileSync('src/components/ContextMenu.tsx', 'utf8').includes('MenuSubmenu')) {
  failures.push('Clip context submenus do not use the shared hover-corridor implementation');
}
if (!/"file\.quit",\s*t\("native\.file\.quit"\),\s*true,\s*Some\("CmdOrCtrl\+Q"\)/.test(nativeMenu)
  || englishCatalog['native.file.quit'] !== 'Quit Pasted') {
  failures.push('The macOS application menu must use the product name for Quit Pasted');
}
if (nativeMenu.includes('.quit()')) {
  failures.push('The generated macOS quit item must not expose the development binary name');
}

if (failures.length > 0) {
  console.error(`Menu-system audit failed:\n- ${failures.join('\n- ')}`);
  process.exit(1);
}

console.log(`Menu-system audit passed (${floatingMenus.length} floating menu surfaces).`);
