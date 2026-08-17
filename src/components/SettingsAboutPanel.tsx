import { useEffect, useState } from 'react';
import { Bot, CheckCircle2, ChevronRight, Copy, Database, ExternalLink, HardDrive, HeartHandshake, Info, RadioTower, Scale, ShieldCheck, TerminalSquare } from 'lucide-react';
import type { InstallationDiagnostics } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { OpenSourceLicensesDialog } from './OpenSourceLicensesDialog';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';
import { CopycatHeadMark } from './CopycatMark';

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
  const [backingError, setBackingError] = useState('');

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

  const openBackingPage = async () => {
    setBackingError('');
    try {
      await invoke('open_backing_page');
    } catch (reason) {
      setBackingError(String(reason));
    }
  };

  return (
    <div className="space-y-5">
      <SettingsPanelHeader
        icon={Info}
        title="About Pasted"
        description="The cat captures clips. We don’t capture copycats."
      />

      <section className="theme-surface relative flex flex-col items-center overflow-hidden rounded-2xl border px-6 py-8 text-center">
        <div className="copycat-about-mark" aria-hidden="true"><CopycatHeadMark /></div>
        <h3 className="theme-title mt-3 text-xl font-bold">Pasted</h3>
        <p className="theme-title mt-1 max-w-md text-sm font-bold">
          Works for copycats. Not for corporations.
        </p>
        <p className="theme-text-muted mt-2 max-w-lg text-xs leading-relaxed">
          Copycats are people, scripts, automations, and agents. They share one private workspace with each other. Nobody else gets a copy—and certainly not us.
        </p>
        <span className="theme-badge mt-4 rounded-full border px-3 py-1 font-mono text-[10px] font-semibold">
          {installation ? `Version ${installation.appVersion} · ${installation.buildKind}` : 'Loading version…'}
        </span>
      </section>

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<HeartHandshake className="h-4 w-4" />}
          title="The Copycat Covenant"
          description="Product constraints, not marketing preferences."
        />
        <div className="grid gap-2 sm:grid-cols-2">
          {[
            { icon: HardDrive, title: 'No cloud account', body: 'Pasted works without an identity, a sync account, or a hosted copy of your clipboard history. The core workspace lives where you do.' },
            { icon: RadioTower, title: 'No telemetry', body: 'We do not measure engagement, inspect clipboard activity, or teach a dashboard how copycats behave. Your work is not our dataset.' },
            { icon: HeartHandshake, title: 'No subscription', body: 'Pasted will not rent your own clipboard back to you. If it earns a place in your workflow, support is an endorsement—not an unlock.' },
            { icon: Bot, title: 'Every copycat welcome', body: 'Humans use the app. Scripts use the CLI. Automations and agents use the tools you explicitly give them. Everyone shares the same local library.' },
          ].map(({ icon: Icon, title, body }) => (
            <article key={title} className="theme-card-idle border p-3.5">
              <Icon className="mb-2 h-4 w-4 text-[var(--accent-primary)]" />
              <h4 className="theme-title text-xs font-bold">{title}</h4>
              <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">{body}</p>
            </article>
          ))}
        </div>
        <p className="theme-text-muted theme-divider border-t pt-3 text-[10px] leading-relaxed">
          Outside intelligence is optional and explicit. Clip content leaves the device only when a copycat runs a connected intelligence-assisted action.
        </p>
        <div className="theme-card-idle flex flex-col gap-3 border p-4 sm:flex-row sm:items-center">
          <div className="min-w-0 flex-1">
            <p className="theme-title text-sm font-bold">
              If Pasted earns a permanent place in your workflow, put $9.99 behind its future.
            </p>
            <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">
              Nothing to unlock. No license key. No ET phone home. Just useful software—and one more reason to keep making it.
            </p>
          </div>
          <ActionButton variant="primary" className="shrink-0" onClick={() => void openBackingPage()}>
            Back Pasted — $9.99 <ExternalLink className="h-3.5 w-3.5" />
          </ActionButton>
        </div>
        {backingError && <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{backingError}</div>}
      </section>

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<HardDrive className="h-4 w-4" />}
          title="This installation"
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
            <span className="theme-title block text-sm font-bold">Open Source Licenses…</span>
            <span className="theme-text-muted mt-0.5 block text-xs">
              Licenses and acknowledgements for bundled software.
            </span>
          </span>
          <ChevronRight className="theme-text-muted h-4 w-4 shrink-0" />
        </button>
      </section>

      <OpenSourceLicensesDialog isOpen={licensesOpen} onClose={() => setLicensesOpen(false)} />
    </div>
  );
}
