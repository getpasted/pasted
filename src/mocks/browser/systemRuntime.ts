import type { MockClip } from './models';
import { mockManualTransforms } from './manualTransforms';
import { getMockSavedTransforms } from './intelligenceRuntime';
import { unhandledValue } from './result';
import { invokeRetentionBrowserMock } from './retentionRuntime';

let mockLibraryLocation = {
  path: '/mock/Pasted/pasted.db',
  directory: '/mock/Pasted',
  isDefault: true,
};


export async function invokeSystemBrowserMock<T>(
  cmd: string,
  args: Record<string, unknown> | undefined,
  mockClips: MockClip[],
): Promise<T | typeof unhandledValue> {
  const retention = await invokeRetentionBrowserMock<T>(cmd);
  if (retention !== unhandledValue) return retention;
  switch (cmd) {
    case 'get_hotkey_capability_status':
      return {
        platform: 'unsupported',
        backend: 'unsupported',
        state: 'unavailable',
        is_trusted: true,
        is_dev_mode: false,
        configured_count: 0,
        registered_count: 0,
        issues: [],
      } as unknown as T;
    case 'get_installation_diagnostics':
      return {
        appVersion: '1.0.0',
        buildKind: 'Development',
        platform: 'macos',
        architecture: 'aarch64',
        bundleIdentifier: 'software.jjj.pasted',
        appPath: '/Applications/Pasted.app',
        dataPath: '/Users/example/Library/Application Support/software.jjj.pasted',
        databaseSizeBytes: 2_457_600,
        signingStatus: 'Ad hoc',
        signingIdentity: null,
        signingTeamId: null,
        notarizationStatus: 'Not expected for development builds',
        cliPath: '/Applications/Pasted.app/Contents/MacOS/pasted',
      } as unknown as T;
    case 'get_app_update_status':
      return {
        configured: false,
        currentVersion: '1.0.0',
        channel: 'stable',
      } as unknown as T;
    case 'check_for_app_update':
      return {
        currentVersion: '1.0.0',
        channel: 'stable',
        available: false,
        version: null,
        notes: null,
        pubDate: null,
      } as unknown as T;
    case 'install_app_update':
      return undefined as unknown as T;
    case 'get_third_party_licenses':
      return {
        schemaVersion: 1,
        componentCount: 2,
        components: [
          { ecosystem: 'cargo', name: 'tauri', version: '2.x', license: 'MIT OR Apache-2.0', repository: 'https://github.com/tauri-apps/tauri', noticeIds: ['development'] },
          { ecosystem: 'npm', name: 'react', version: '19.x', license: 'MIT', repository: 'https://github.com/facebook/react', noticeIds: ['development'] },
        ],
        noticeText: [
          'Pasted Third-Party Software Notices',
          '',
          'Development preview',
          '',
          'Production builds embed the complete generated component inventory and license text.',
          'Run `pasted licenses` or open this dialog in the native app to inspect that document.',
        ].join('\n'),
      } as unknown as T;
    case 'get_ocr_backfill_status':
      return {
        totalImages: 0,
        eligibleCount: 0,
        queuedCount: 0,
        runningCount: 0,
        completedCount: 0,
        noTextCount: 0,
        failedCount: 0,
      } as unknown as T;
    case 'start_ocr_backfill':
    case 'cancel_ocr_backfill':
      return undefined as T;
    case 'retry_failed_ocr':
      return 0 as unknown as T;
    case 'copy_clip_to_system':
    case 'copy_clip_by_id':
    case 'paste_text_to_frontmost':
      return null as unknown as T;
    case 'quit_app':
      return undefined as unknown as T;
    case 'get_source_icons':
      return {} as unknown as T;
    case 'get_external_import_sources':
      return [
        { id: 'alfred', label: 'Alfred', description: 'Clipboard history from Alfred Powerpack', available: true, detected: true, defaultPath: '/mock/Alfred/clipboard.alfdb', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'pastebot', label: 'Pastebot', description: 'Text history from Pastebot', available: false, detected: true, defaultPath: '/mock/Pastebot.sqlite', supportsCustomFile: true, selectionKind: 'folder' },
        { id: 'pasta', label: 'Pasta', description: 'Text history from Pasta', available: false, detected: false, defaultPath: '/mock/Pasta/pasta.sqlite', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'paste', label: 'Paste', description: 'Text history from Paste', available: false, detected: false, defaultPath: '/mock/Paste/Paste.sqlite', supportsCustomFile: true, selectionKind: 'folder' },
        { id: 'copyclip', label: 'CopyClip 2', description: 'Text history from CopyClip 2', available: false, detected: false, defaultPath: '/mock/CopyClip.data', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'maccy', label: 'Maccy', description: 'Text history from Maccy', available: false, detected: false, defaultPath: '/mock/Maccy/Storage.sqlite', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'flycut', label: 'Flycut', description: 'Text history from Flycut', available: false, detected: false, defaultPath: '/mock/Flycut.plist', supportsCustomFile: true, selectionKind: 'file' },
      ] as unknown as T;
    case 'import_external_history':
      return {
        source: String(args?.source ?? 'pastebot'),
        scannedCount: 42,
        importedCount: 38,
        duplicateCount: 3,
        skippedCount: 1,
        historyCapacityAdjustedTo: 1200,
      } as unknown as T;
    case 'get_library_location':
      return mockLibraryLocation as unknown as T;
    case 'get_storage_protection':
      return {
        status: 'protected',
        technology: 'FileVault',
        summary: 'FileVault is on',
        detail: 'The volume containing this database is encrypted.',
      } as unknown as T;
    case 'move_library':
      mockLibraryLocation = {
        path: '/mock/Custom Pasted Library/pasted.db',
        directory: '/mock/Custom Pasted Library',
        isDefault: false,
      };
      return { location: mockLibraryLocation, recoveryPath: '/mock/Pasted/pasted.db' } as unknown as T;
    case 'restore_default_library_location':
      mockLibraryLocation = {
        path: '/mock/Pasted/pasted.db',
        directory: '/mock/Pasted',
        isDefault: true,
      };
      return { location: mockLibraryLocation, recoveryPath: '/mock/Custom Pasted Library/pasted.db' } as unknown as T;
    case 'perform_titlebar_double_click':
    case 'play_system_sound':
    case 'set_dock_visibility':
    case 'set_linux_native_menu_theme':
    case 'set_overlay_cursor':
    case 'set_titlebar_direction':
    case 'toggle_hud_window':
      return undefined as unknown as T;
    case 'get_transforms':
      return [
        ...mockManualTransforms.map((manualTransform) => ({
          ...manualTransform,
          authoringKind: 'manual',
          executionCharacter: 'replayable',
          connectionId: null,
          plan: null,
        })),
        ...getMockSavedTransforms().map((transform) => ({
          ...transform,
          authoringKind: 'intent',
          executionCharacter: 'interpretive',
          steps: [],
        })),
      ] as unknown as T;
    case 'get_capture_feedback_clip': {
      const clip = mockClips.find(({ id }) => id === Number(args?.id));
      if (!clip) throw new Error('Clip was not found');
      return {
        id: clip.id,
        contentType: clip.content_type,
        previewText: clip.text_content.slice(0, 280),
        source: clip.source,
        isPinned: Boolean(clip.is_pinned),
        isProtected: Boolean(clip.is_protected),
        isTrashed: Boolean(clip.is_trashed),
      } as unknown as T;
    }
    case 'get_installed_applications':
      return ['Finder', 'Safari', 'Terminal'] as unknown as T;
    case 'install_cli_to_path':
      return '/mock/bin/pasted' as unknown as T;
    case 'open_backing_page':
    case 'open_emoji_picker':
    case 'request_accessibility_permission':
      return true as unknown as T;
    case 'paste_clip_by_id':
      return undefined as unknown as T;
    case 'resolve_logical_shortcut_key':
      return String(args?.fallback ?? '') as unknown as T;
  }
  return unhandledValue;
}
