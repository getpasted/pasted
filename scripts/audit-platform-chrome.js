import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const baseConfig = readJson('src-tauri/tauri.conf.json');
const macConfig = readJson('src-tauri/tauri.macos.conf.json');
const mainSource = fs.readFileSync('src/main.tsx', 'utf8');
const sidebarSource = [
  'src/components/Sidebar.tsx',
  'src/components/SidebarClipSection.tsx',
  'src/components/CollapsedSidebar.tsx',
].map((path) => fs.readFileSync(path, 'utf8')).join('\n');
const chromeCss = fs.readFileSync('src/styles/layout-chrome.css', 'utf8');
const cargoManifest = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const rustMainSource = fs.readFileSync('src-tauri/src/main.rs', 'utf8');
const settingsSource = fs.readFileSync('src/hooks/useAppSettings.ts', 'utf8');
const linuxThemeSource = fs.readFileSync('src-tauri/src/linux_native_theme.rs', 'utf8');
const windowDragSource = fs.readFileSync('src/utils/windowDrag.ts', 'utf8');
const titlebarSource = fs.readFileSync('src-tauri/src/titlebar.rs', 'utf8');
const rustLibSource = fs.readFileSync('src-tauri/src/lib.rs', 'utf8');
const appWindowsSource = fs.readFileSync('src-tauri/src/app_windows.rs', 'utf8');
const rtlWindowControlsSource = fs.readFileSync('src/components/MacRtlWindowControls.tsx', 'utf8');
const appLockScreenSource = fs.readFileSync('src/components/AppLockScreen.tsx', 'utf8');
const hudWindowSource = fs.readFileSync('src-tauri/src/hud_window.rs', 'utf8');
const hudCommandSource = fs.readFileSync('src-tauri/src/commands/hud.rs', 'utf8');
const viteSource = fs.readFileSync('vite.config.ts', 'utf8');
const hudEntrySource = fs.readFileSync('src/hud-main.tsx', 'utf8');
const feedbackEntrySource = fs.readFileSync('src/capture-feedback-main.tsx', 'utf8');
const auxiliarySettingsSource = fs.readFileSync('src/hooks/useAuxiliaryAppSettings.ts', 'utf8');
const auxiliaryReadySource = fs.readFileSync('src/hooks/useAuxiliaryWindowReady.ts', 'utf8');
const hudHtml = fs.readFileSync('hud.html', 'utf8');
const feedbackHtml = fs.readFileSync('capture-feedback.html', 'utf8');

const windowByLabel = (config, label) => config.app.windows.find((window) => window.label === label);
const baseMain = windowByLabel(baseConfig, 'main');
const macMain = windowByLabel(macConfig, 'main');
const captureFeedback = windowByLabel(baseConfig, 'capture-feedback');
const baseHud = windowByLabel(baseConfig, 'hud');
const macHud = windowByLabel(macConfig, 'hud');

assert.ok(baseMain, 'Base configuration must define the main window');
assert.ok(captureFeedback, 'Base configuration must define the capture feedback window');
assert.ok(baseHud && macHud, 'Every platform configuration must define the HUD window');
assert.equal(baseHud.url, 'hud.html', 'The HUD must load its dedicated entry point');
assert.equal(macHud.url, 'hud.html', 'The macOS HUD must load its dedicated entry point');
assert.equal(captureFeedback.url, 'capture-feedback.html', 'Capture feedback must load its dedicated entry point');
assert.equal(baseHud.height, 448, 'The HUD must retain a snug layout with all nine rows visible');
assert.equal(macHud.height, baseHud.height, 'HUD height must remain synchronized across platform configurations');
assert.match(hudWindowSource, /HUD_HEIGHT: f64 = 448\.0/, 'Native HUD positioning must use the configured height');
assert.match(hudCommandSource, /hud_window::HUD_HEIGHT/, 'Every HUD positioning path must share the native height constant');
for (const entry of ['index.html', 'hud.html', 'capture-feedback.html']) {
  assert.match(viteSource, new RegExp(entry.replace('.', '\\.')),
    `Vite must retain the ${entry} build entry`);
}
for (const [name, html] of [['HUD', hudHtml], ['Capture feedback', feedbackHtml]]) {
  assert.match(html, /#root \{ visibility: hidden; \}/,
    `${name} must remain hidden before React finishes initialization`);
  assert.match(html, /html\[data-window-ready="true"\] #root \{ visibility: visible; \}/,
    `${name} must reveal atomically without a flash of uninitialized content`);
}
for (const entrySource of [hudEntrySource, feedbackEntrySource]) {
  assert.match(entrySource, /useAuxiliaryAppSettings\(\)/,
    'Auxiliary entries must not run main-window settings side effects');
  assert.match(entrySource, /useAuxiliaryWindowReady\(ready\)/,
    'Auxiliary entries must explicitly coordinate their first paint');
  assert.doesNotMatch(entrySource, /useAppController|useAppSettings|from ['"]\.\/App['"]/,
    'Auxiliary entries must remain independent from the main application graph');
}
assert.doesNotMatch(auxiliarySettingsSource, /plugin-autostart|enforce_.*retention|set_dock_visibility/,
  'Auxiliary settings hydration must remain read-only');
assert.match(auxiliarySettingsSource, /useLayoutEffect/,
  'Auxiliary theme, text size, and locale must settle before first paint');
assert.match(auxiliaryReadySource, /requestAnimationFrame/,
  'Auxiliary content must reveal on a complete animation frame');
assert.equal(baseMain.visible, false, 'The main window must remain hidden until its startup surface is ready');
assert.equal(captureFeedback.focus, false, 'Capture feedback must never steal focus');
assert.equal(
  baseConfig.app.macOSPrivateApi,
  true,
  'The base config must retain the Cargo feature during cross-platform Tauri builds',
);
assert.match(
  cargoManifest,
  /tauri\s*=\s*\{[^\n]*features\s*=\s*\[[^\]]*"macos-private-api"/,
  'Cargo must retain macos-private-api even after a Linux release build',
);
assert.equal(baseMain.decorations, true, 'Windows and Linux must retain native window decorations');
assert.equal(baseMain.transparent, false, 'Native framed windows need an opaque native window background');
assert.equal('titleBarStyle' in baseMain, false, 'macOS title-bar style must not leak into the base configuration');
assert.equal('trafficLightPosition' in baseMain, false, 'macOS traffic-light placement must not leak into the base configuration');
assert.equal('hiddenTitle' in baseMain, false, 'Native framed platforms must retain their window title');

assert.ok(macMain, 'macOS configuration must define the main window override');
assert.equal(macMain.visible, false, 'The macOS window must not expose vibrancy before the splash is ready');
assert.equal(macConfig.app.macOSPrivateApi, true, 'The macOS overlay requires the private API opt-in');
assert.equal(macMain.decorations, true, 'macOS must retain native traffic-light behavior');
assert.equal(macMain.titleBarStyle, 'Overlay', 'macOS must retain the full-size overlay titlebar');
assert.equal(macMain.hiddenTitle, true, 'Pasted draws its own macOS title presentation');
assert.equal(macMain.transparent, true, 'macOS must retain its vibrancy-compatible transparent window');
assert.deepEqual(macMain.trafficLightPosition, { x: 20, y: 30 }, 'macOS traffic-light placement changed unexpectedly');

assert.deepEqual(
  baseConfig.app.windows.map(({ label }) => label),
  macConfig.app.windows.map(({ label }) => label),
  'Platform configuration must preserve the complete Tauri window set',
);
assert.ok(
  mainSource.indexOf('applyDesktopPlatform();') < mainSource.indexOf('ReactDOM.createRoot'),
  'Platform safe areas must be applied before the first React render',
);
assert.match(chromeCss, /html\[data-platform="macos"\] \.platform-macos-only/);
assert.match(chromeCss, /html\[data-platform="linux"\] \.platform-linux-only/);
assert.match(chromeCss, /html:not\(\[data-platform="macos"\]\) \.platform-framed-only/);
const macWindowControlRule = chromeCss.match(/\.mac-window-control \{([\s\S]*?)\n\}/)?.[1] ?? '';
const activeMacWindowControlRule = chromeCss.match(/\.mac-window-control:active \{([\s\S]*?)\n\}/)?.[1] ?? '';
assert.match(macWindowControlRule, /flex:\s*0 0 14px;/, 'RTL traffic lights must retain a fixed layout footprint');
assert.match(macWindowControlRule, /transition:[^;]*transform/, 'RTL traffic-light presses should animate');
assert.match(
  activeMacWindowControlRule,
  /transform:\s*scale\(1\.23\);/,
  'RTL traffic-light presses should grow visually without moving adjacent controls',
);
assert.match(
  activeMacWindowControlRule,
  /filter:\s*brightness\(1\.08\) saturate\(1\.06\);/,
  'RTL traffic-light presses should brighten without changing their theme colors',
);
assert.doesNotMatch(
  rtlWindowControlsSource,
  /onFocusChanged|is-window-inactive/,
  'RTL traffic-light focus styling should not wait for asynchronous React state',
);
assert.match(
  chromeCss,
  /html\[data-window-inactive\] \.mac-window-control \{[\s\S]*?--mac-window-control-fill:\s*var\(--mac-traffic-inactive\);/,
  'Inactive RTL traffic lights should use the neutral macOS fill',
);
assert.match(appWindowsSource, /WindowEvent::Focused\(true\)/);
assert.match(appWindowsSource, /removeAttribute\('data-window-inactive'\)/);
assert.match(titlebarSource, /NSWindowDidBecomeKeyNotification/);
assert.doesNotMatch(titlebarSource, /NSWindowDidResignKeyNotification/);
assert.match(appWindowsSource, /titlebar::install_focus_observers\(&main_window\)/);
assert.match(mainSource, /markWindowActive[\s\S]*removeAttribute\('data-window-inactive'\)/);
assert.match(mainSource, /addEventListener\('pointerdown', markWindowActive/);
assert.match(mainSource, /addEventListener\('blur',[\s\S]*setAttribute\('data-window-inactive', ''\)/);
assert.match(sidebarSource, /sidebar-titlebar-leading/);
assert.match(
  sidebarSource,
  /platform-macos-only h-\[60px\]/,
  'The expanded sidebar titlebar allowance must remain macOS-only',
);
assert.match(
  sidebarSource,
  /platform-framed-only sidebar-control-muted h-7 w-7/,
  'Native framed platforms need an inline sidebar collapse control',
);
assert.doesNotMatch(sidebarSource, /\bpl-20\b/, 'Do not restore a universal macOS traffic-light inset');
assert.match(rustMainSource, /configure_appimage_wayland_compatibility/);
assert.match(rustMainSource, /\/usr\/lib\/libwayland-client\.so\.0/);
assert.match(rustMainSource, /std::env::var_os\("APPIMAGE"\)/);
assert.match(settingsSource, /root\.dataset\.platform === 'linux'/);
assert.match(settingsSource, /getCurrentWindow\(\)\.setTheme\(nativeTheme\)/);
assert.match(settingsSource, /set_linux_native_menu_theme/);
assert.match(linuxThemeSource, /gtk::StyleContext::add_provider_for_screen/);
assert.match(linuxThemeSource, /menubar > menuitem:hover/);
assert.match(
  cargoManifest,
  /arboard = \{ version = "3\.4", features = \["wayland-data-control"\], optional = true \}/,
  'Linux clipboard history needs an optional Wayland data-control dependency instead of an XWayland fallback',
);
assert.match(
  cargoManifest,
  /gui = \[[\s\S]*"dep:arboard"[\s\S]*\]/,
  'The GUI feature must retain ownership of clipboard integration',
);
assert.match(windowDragSource, /isInteractiveTitlebarTarget\(event\.target\)/);
assert.match(windowDragSource, /document\.documentElement\.dataset\.platform !== 'macos'/);
assert.match(windowDragSource, /handleWindowDragDoubleClick/);
assert.doesNotMatch(windowDragSource, /event\.detail/);
assert.match(windowDragSource, /perform_titlebar_double_click/);
assert.match(windowDragSource, /\.titlebar-no-drag/);
assert.match(
  sidebarSource,
  /onDoubleClick=\{handleWindowDragDoubleClick\}/,
  'Custom title bars must wait for a confirmed double-click before resizing the window',
);
assert.match(
  appLockScreenSource,
  /onMouseDown=\{startWindowDrag\}/,
  'The app window must remain movable while the lock screen covers its normal title bars',
);
assert.match(
  appLockScreenSource,
  /onDoubleClick=\{handleWindowDragDoubleClick\}/,
  'The locked window must preserve the configured macOS title-bar double-click action',
);
assert.match(titlebarSource, /AppleActionOnDoubleClick/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::Minimize/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::None/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::Fill/);
assert.match(titlebarSource, /run_on_main_thread/);
assert.match(titlebarSource, /STANDARD_ZOOM_WIDTH: f64 = 1040\.0/);
assert.match(titlebarSource, /STANDARD_ZOOM_HEIGHT: f64 = 640\.0/);
assert.match(titlebarSource, /setFrame: next display: 1i8 animate: 0i8/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::Fill => visible/);
assert.match(rustLibSource, /commands::platform::perform_titlebar_double_click/);
assert.match(
  appWindowsSource,
  /NSVisualEffectState::Active/,
  'macOS vibrancy must not retint the app when the native active-state calculation changes',
);
assert.doesNotMatch(appWindowsSource, /NSVisualEffectState::FollowsWindowActiveState/);
assert.match(appWindowsSource, /MAIN_PAGE_LOADED/);
assert.match(appWindowsSource, /STARTUP_SETUP_READY/);
assert.match(rustLibSource, /PageLoadEvent::Finished/);
assert.match(
  appWindowsSource,
  /getElementById\('startup-splash'\).*getAnimations\(\{ subtree: true \}\)/,
  'The first visible startup frame must restart the splash animation instead of exposing native chrome',
);

console.log('Platform window-chrome audit passed.');
