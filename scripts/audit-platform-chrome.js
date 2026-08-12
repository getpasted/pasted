import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const baseConfig = readJson('src-tauri/tauri.conf.json');
const macConfig = readJson('src-tauri/tauri.macos.conf.json');
const mainSource = fs.readFileSync('src/main.tsx', 'utf8');
const sidebarSource = fs.readFileSync('src/components/Sidebar.tsx', 'utf8');
const chromeCss = fs.readFileSync('src/styles/layout-chrome.css', 'utf8');
const cargoManifest = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const rustMainSource = fs.readFileSync('src-tauri/src/main.rs', 'utf8');
const settingsSource = fs.readFileSync('src/hooks/useAppSettings.ts', 'utf8');
const linuxThemeSource = fs.readFileSync('src-tauri/src/linux_native_theme.rs', 'utf8');
const windowDragSource = fs.readFileSync('src/utils/windowDrag.ts', 'utf8');
const titlebarSource = fs.readFileSync('src-tauri/src/titlebar.rs', 'utf8');
const rustLibSource = fs.readFileSync('src-tauri/src/lib.rs', 'utf8');

const windowByLabel = (config, label) => config.app.windows.find((window) => window.label === label);
const baseMain = windowByLabel(baseConfig, 'main');
const macMain = windowByLabel(macConfig, 'main');
const captureFeedback = windowByLabel(baseConfig, 'capture-feedback');

assert.ok(baseMain, 'Base configuration must define the main window');
assert.ok(captureFeedback, 'Base configuration must define the capture feedback window');
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
assert.match(chromeCss, /html:not\(\[data-platform="macos"\]\) \.platform-framed-only/);
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
  /arboard = \{ version = "3\.4", features = \["wayland-data-control"\] \}/,
  'Linux clipboard history needs Wayland data-control instead of an XWayland fallback',
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
assert.match(titlebarSource, /AppleActionOnDoubleClick/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::Minimize/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::None/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::Fill/);
assert.match(titlebarSource, /run_on_main_thread/);
assert.match(titlebarSource, /STANDARD_ZOOM_WIDTH: f64 = 1040\.0/);
assert.match(titlebarSource, /STANDARD_ZOOM_HEIGHT: f64 = 640\.0/);
assert.match(titlebarSource, /setFrame: next display: 1i8 animate: 0i8/);
assert.match(titlebarSource, /TitlebarDoubleClickAction::Fill => visible/);
assert.match(rustLibSource, /commands::perform_titlebar_double_click/);

console.log('Platform window-chrome audit passed.');
