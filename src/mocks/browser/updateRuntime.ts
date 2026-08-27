import { unhandledValue } from './result';
import type { InstallationDiagnostics } from '../../types';

const developmentInstallationDiagnostics = {
  appVersion: '1.0.0',
  buildKind: 'development',
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
} satisfies InstallationDiagnostics;

export function invokeUpdateBrowserMock<T>(cmd: string): T | typeof unhandledValue {
  switch (cmd) {
    case 'get_installation_diagnostics':
      return developmentInstallationDiagnostics as unknown as T;
    case 'get_app_update_status':
      return {
        configured: false,
        enabled: true,
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
    default:
      return unhandledValue;
  }
}
