import { useEffect, useState } from 'react';
import { CheckCircle2, ChevronRight, Copy, Database, HardDrive, Info, Scale, ShieldCheck, TerminalSquare } from 'lucide-react';
import type { InstallationDiagnostics } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { OpenSourceLicensesDialog } from './OpenSourceLicensesDialog';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';

function fileSize(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${unit}`;
}

function installationSummary(details: InstallationDiagnostics) {
  return [
    `Pasted ${details.appVersion} (${details.buildKind})`,
    `Platform: ${details.platform} (${details.architecture})`,
    `Bundle identifier: ${details.bundleIdentifier}`,
    `Application: ${details.appPath}`,
    `Data: ${details.dataPath}`,
    `Database: ${details.databaseSizeBytes} bytes`,
    `Code signing: ${details.signingStatus}`,
    ...(details.signingIdentity ? [`Signing identity: ${details.signingIdentity}`] : []),
    ...(details.signingTeamId ? [`Signing team: ${details.signingTeamId}`] : []),
    `Notarization: ${details.notarizationStatus}`,
    `CLI: ${details.cliPath ?? 'Not installed beside Pasted'}`,
  ].join('\n');
}

export function SettingsAboutPanel() {
  const [installation, setInstallation] = useState<InstallationDiagnostics | null>(null);
  const [error, setError] = useState('');
  const [copied, setCopied] = useState(false);
  const [licensesOpen, setLicensesOpen] = useState(false);

  useEffect(() => {
    invoke<InstallationDiagnostics>('get_installation_diagnostics')
      .then((details) => setInstallation(details))
      .catch((reason) => setError(String(reason)));
  }, []);

  const copyDetails = async () => {
    if (!installation) return;
    await navigator.clipboard.writeText(installationSummary(installation));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1_500);
  };

  return (
    <div className="space-y-5">
      <SettingsPanelHeader
        icon={Info}
        title="About Pasted"
        description="A fast, focused clipboard workspace that keeps you in control."
      />

      <section className="theme-surface flex flex-col items-center rounded-2xl border px-6 py-8 text-center">
        <img src="/app_icon.png" alt="" className="h-20 w-20" draggable={false} />
        <h3 className="theme-title mt-3 text-xl font-bold">Pasted</h3>
        <p className="theme-text-muted mt-1 text-xs">Copy once. Find it, organize it, and shape it whenever you need it.</p>
        <span className="theme-badge mt-4 rounded-full border px-3 py-1 font-mono text-[10px] font-semibold">
          {installation ? `Version ${installation.appVersion} · ${installation.buildKind}` : 'Loading version…'}
        </span>
      </section>

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<HardDrive className="h-4 w-4" />}
          title="This Installation"
          description="Details for troubleshooting and verification."
          actions={<ActionButton disabled={!installation} onClick={() => void copyDetails()} className="shrink-0 disabled:opacity-40">
            {copied ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? 'Copied' : 'Copy Details'}
          </ActionButton>}
        />

        {installation ? (
          <div className="grid gap-2 sm:grid-cols-2">
            {[
              { icon: HardDrive, label: 'Version', value: `${installation.appVersion} · ${installation.buildKind}` },
              { icon: ShieldCheck, label: 'Verification', value: `${installation.signingStatus} · ${installation.notarizationStatus}` },
              { icon: Database, label: 'Database', value: fileSize(installation.databaseSizeBytes) },
              { icon: TerminalSquare, label: 'Command Line', value: installation.cliPath ? 'Installed' : 'Not installed beside Pasted' },
            ].map(({ icon: Icon, label, value }) => (
              <div key={label} className="theme-card-idle flex min-w-0 items-center gap-2.5 border px-3 py-2.5">
                <Icon className="theme-text-muted h-4 w-4 shrink-0" />
                <div className="min-w-0">
                  <div className="theme-text-muted text-[9px] font-semibold uppercase tracking-wider">{label}</div>
                  <div className="theme-text-main mt-0.5 truncate text-xs font-semibold" title={value}>{value}</div>
                </div>
              </div>
            ))}
            {[
              ['Application', installation.appPath],
              ['Data', installation.dataPath],
            ].map(([label, value]) => (
              <div key={label} className="theme-card-idle min-w-0 border px-3 py-2.5 sm:col-span-2">
                <div className="theme-text-muted text-[9px] font-semibold uppercase tracking-wider">{label}</div>
                <div className="theme-text-main mt-1 select-text break-all font-mono text-[10px] leading-relaxed">{value}</div>
              </div>
            ))}
            <div className="theme-text-muted px-1 text-[10px] sm:col-span-2">
              {installation.bundleIdentifier} · {installation.platform} {installation.architecture}
              {installation.signingTeamId ? ` · Team ${installation.signingTeamId}` : ''}
            </div>
          </div>
        ) : (
          <div className="theme-text-muted p-3 text-center text-xs">Inspecting this installation…</div>
        )}
        {error && <div className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{error}</div>}
      </section>

      <section className="theme-surface rounded-2xl border p-3">
        <button
          type="button"
          onClick={() => setLicensesOpen(true)}
          className="theme-card-idle flex w-full items-center gap-3 border px-3 py-3 text-left"
        >
          <SettingsAccentTile>
            <Scale className="h-4 w-4" />
          </SettingsAccentTile>
          <span className="min-w-0 flex-1">
            <span className="theme-title block text-sm font-bold">Open Source Licenses</span>
            <span className="theme-text-muted mt-0.5 block text-xs">
              Licenses and acknowledgements for software included with Pasted.
            </span>
          </span>
          <ChevronRight className="theme-text-muted h-4 w-4 shrink-0" />
        </button>
      </section>

      <OpenSourceLicensesDialog isOpen={licensesOpen} onClose={() => setLicensesOpen(false)} />
    </div>
  );
}
