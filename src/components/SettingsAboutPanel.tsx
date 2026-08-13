import { useEffect, useState } from 'react';
import { Bot, CheckCircle2, ChevronRight, Copy, Database, HardDrive, HeartHandshake, Info, RadioTower, Scale, ShieldCheck, TerminalSquare } from 'lucide-react';
import type { InstallationDiagnostics } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { OpenSourceLicensesDialog } from './OpenSourceLicensesDialog';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';
import { CopycatMark } from './CopycatMark';

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
        description="The private, local clipboard workspace for copycats."
      />

      <section className="theme-surface relative flex flex-col items-center overflow-hidden rounded-2xl border px-6 py-8 text-center">
        <div className="copycat-about-mark" aria-hidden="true"><CopycatMark /></div>
        <h3 className="theme-title mt-3 text-xl font-bold">Pasted</h3>
        <p className="theme-text-muted mt-1 max-w-md text-xs leading-relaxed">
          One local workspace for everything humans, scripts, automations, and agents copy along the way.
        </p>
        <span className="theme-badge mt-4 rounded-full border px-3 py-1 font-mono text-[10px] font-semibold">
          {installation ? `Version ${installation.appVersion} · ${installation.buildKind}` : 'Loading version…'}
        </span>
      </section>

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<HeartHandshake className="h-4 w-4" />}
          title="The Copycat Covenant"
          description="The constraints Pasted chooses so your clipboard remains yours."
        />
        <div className="grid gap-2 sm:grid-cols-2">
          {[
            { icon: HardDrive, title: 'No cloud account', body: 'Your core library lives locally. Pasted works without an identity, sync account, or hosted copy of your history.' },
            { icon: RadioTower, title: 'No telemetry', body: 'Pasted does not measure engagement or report how humans, scripts, or agents use the workspace.' },
            { icon: HeartHandshake, title: 'No subscription', body: 'Pasted will not rent your clipboard back to you. Financial support is an endorsement, never an unlock.' },
            { icon: Bot, title: 'Every copycat welcome', body: 'The GUI and CLI share one library, so people and the tools they direct work from the same local context.' },
          ].map(({ icon: Icon, title, body }) => (
            <article key={title} className="theme-card-idle border p-3.5">
              <Icon className="mb-2 h-4 w-4 text-[var(--accent-primary)]" />
              <h4 className="theme-title text-xs font-bold">{title}</h4>
              <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">{body}</p>
            </article>
          ))}
        </div>
        <p className="theme-text-muted theme-divider border-t pt-3 text-[10px] leading-relaxed">
          Outside intelligence is optional and explicit. Pasted sends clip content only when a copycat runs a connected intelligence-assisted action.
        </p>
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
