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
import { translate } from '../localization/runtime';

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
    translate('component.settingsAboutPanel.diagnosticPlatform', { platform: details.platform, architecture: details.architecture }),
    translate('component.settingsAboutPanel.diagnosticBundleIdentifier', { value: details.bundleIdentifier }),
    translate('component.settingsAboutPanel.diagnosticApplication', { value: details.appPath }),
    translate('component.settingsAboutPanel.diagnosticData', { value: details.dataPath }),
    translate('component.settingsAboutPanel.diagnosticDatabaseBytes', { value: details.databaseSizeBytes }),
    translate('component.settingsAboutPanel.diagnosticCodeSigning', { value: details.signingStatus }),
    ...(details.signingIdentity ? [translate('component.settingsAboutPanel.diagnosticSigningIdentity', { value: details.signingIdentity })] : []),
    ...(details.signingTeamId ? [translate('component.settingsAboutPanel.diagnosticSigningTeam', { value: details.signingTeamId })] : []),
    translate('component.settingsAboutPanel.diagnosticNotarization', { value: details.notarizationStatus }),
    translate('component.settingsAboutPanel.diagnosticCli', { value: details.cliPath ?? translate('component.settingsAboutPanel.notInstalledBesidePasted') }),
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
        title={translate('component.settingsAboutPanel.aboutPasted')}
        description={translate('component.settingsAboutPanel.theCatCapturesClipsWeDonTCaptureCopycats')}
      />

      <section className="theme-surface relative flex flex-col items-center overflow-hidden rounded-2xl border px-6 py-8 text-center">
        <div className="copycat-about-mark" aria-hidden="true"><CopycatHeadMark /></div>
        <h3 className="theme-title mt-3 text-xl font-bold">{translate('component.settingsAboutPanel.pasted')}</h3>
        <p className="theme-title mt-1 max-w-md text-sm font-bold">
          {translate('component.settingsAboutPanel.worksForCopycatsNotForCorporations')}
        </p>
        <p className="theme-text-muted mt-2 max-w-lg text-xs leading-relaxed">
          {translate('component.settingsAboutPanel.copycatsArePeopleScriptsAutomationsAndAgentsTheyShareOnePrivateWorkspace')}
        </p>
        <span className="theme-badge mt-4 rounded-full border px-3 py-1 font-mono text-[10px] font-semibold">
          {installation ? translate('component.settingsAboutPanel.versionAppversionBuildkind', { appVersion: installation.appVersion, buildKind: installation.buildKind }) : translate('component.settingsAboutPanel.loadingVersion')}
        </span>
      </section>

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<HeartHandshake className="h-4 w-4" />}
          title={translate('component.settingsAboutPanel.theCopycatCovenant')}
          description={translate('component.settingsAboutPanel.productConstraintsNotMarketingPreferences')}
        />
        <div className="grid gap-2 sm:grid-cols-2">
          {[
            { icon: HardDrive, get title() { return translate('component.settingsAboutPanel.noCloudAccount'); }, get body() { return translate('component.settingsAboutPanel.pastedWorksWithoutAnIdentityASyncAccountOrAHostedCopy'); } },
            { icon: RadioTower, get title() { return translate('component.settingsAboutPanel.noTelemetry'); }, get body() { return translate('component.settingsAboutPanel.weDoNotMeasureEngagementInspectClipboardActivityOrTeachADashboard'); } },
            { icon: HeartHandshake, get title() { return translate('component.settingsAboutPanel.noSubscription'); }, get body() { return translate('component.settingsAboutPanel.pastedWillNotRentYourOwnClipboardBackToYouIfIt'); } },
            { icon: Bot, get title() { return translate('component.settingsAboutPanel.everyCopycatWelcome'); }, get body() { return translate('component.settingsAboutPanel.humansUseTheAppScriptsUseTheCliAutomationsAndAgentsUse'); } },
          ].map(({ icon: Icon, title, body }) => (
            <article key={title} className="theme-card-idle border p-3.5">
              <Icon className="mb-2 h-4 w-4 text-[var(--accent-primary)]" />
              <h4 className="theme-title text-xs font-bold">{title}</h4>
              <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">{body}</p>
            </article>
          ))}
        </div>
        <p className="theme-text-muted theme-divider border-t pt-3 text-[10px] leading-relaxed">
          {translate('component.settingsAboutPanel.outsideIntelligenceIsOptionalAndExplicitClipContentLeavesTheDeviceOnly')}
        </p>
        <div className="theme-card-idle flex flex-col gap-3 border p-4 sm:flex-row sm:items-center">
          <div className="min-w-0 flex-1">
            <p className="theme-title text-sm font-bold">
              {translate('component.settingsAboutPanel.ifPastedEarnsAPermanentPlaceInYourWorkflowPut999')}
            </p>
            <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">
              {translate('component.settingsAboutPanel.nothingToUnlockNoLicenseKeyNoEtPhoneHomeJustUseful')}
            </p>
          </div>
          <ActionButton variant="primary" className="shrink-0" onClick={() => void openBackingPage()}>
            {translate('component.settingsAboutPanel.backPasted999')} <ExternalLink className="h-3.5 w-3.5" />
          </ActionButton>
        </div>
        {backingError && <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{backingError}</div>}
      </section>

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<HardDrive className="h-4 w-4" />}
          title={translate('component.settingsAboutPanel.thisInstallation')}
          description={translate('component.settingsAboutPanel.detailsForTroubleshootingAndVerification')}
          actions={<ActionButton disabled={!installation} onClick={() => void copyDetails()} className="shrink-0 disabled:opacity-40">
            {copied ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? translate('action.copied') : translate('component.settingsAboutPanel.copyDetails')}
          </ActionButton>}
        />

        {installation ? (
          <div className="grid gap-2 sm:grid-cols-2">
            {[
              { icon: HardDrive, get label() { return translate('component.settingsAboutPanel.version'); }, value: translate('component.settingsAboutPanel.appversionBuildkind', { appVersion: installation.appVersion, buildKind: installation.buildKind }) },
              { icon: ShieldCheck, get label() { return translate('component.settingsAboutPanel.verification'); }, value: translate('component.settingsAboutPanel.signingstatusNotarizationstatus', { signingStatus: installation.signingStatus, notarizationStatus: installation.notarizationStatus }) },
              { icon: Database, get label() { return translate('component.settingsAboutPanel.database'); }, value: fileSize(installation.databaseSizeBytes) },
              { icon: TerminalSquare, get label() { return translate('component.settingsAboutPanel.commandLine'); }, value: installation.cliPath ? translate('component.settingsAboutPanel.installed') : translate('component.settingsAboutPanel.notInstalledBesidePasted') },
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
              [translate('component.settingsAboutPanel.application'), installation.appPath],
              [translate('component.settingsAboutPanel.data'), installation.dataPath],
            ].map(([label, value]) => (
              <div key={label} className="theme-card-idle min-w-0 border px-3 py-2.5 sm:col-span-2">
                <div className="theme-text-muted text-[9px] font-semibold uppercase tracking-wider">{label}</div>
                <div className="theme-text-main mt-1 select-text break-all font-mono text-[10px] leading-relaxed">{value}</div>
              </div>
            ))}
            <div className="theme-text-muted px-1 text-[10px] sm:col-span-2">
              {installation.bundleIdentifier} · {installation.platform} {installation.architecture}
              {installation.signingTeamId ? translate('component.settingsAboutPanel.teamId', { id: installation.signingTeamId }) : ''}
            </div>
          </div>
        ) : (
          <div className="theme-text-muted p-3 text-center text-xs">{translate('component.settingsAboutPanel.inspectingThisInstallation')}</div>
        )}
        {error && <div className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{error}</div>}
      </section>

      <section className="theme-surface rounded-2xl border p-3">
        <button
          type="button"
          onClick={() => setLicensesOpen(true)}
          className="theme-card-idle flex w-full items-center gap-3 border px-3 py-3 text-start"
        >
          <SettingsAccentTile>
            <Scale className="h-4 w-4" />
          </SettingsAccentTile>
          <span className="min-w-0 flex-1">
            <span className="theme-title block text-sm font-bold">{translate('component.settingsAboutPanel.openSourceLicenses')}</span>
            <span className="theme-text-muted mt-0.5 block text-xs">
              {translate('component.settingsAboutPanel.licensesAndAcknowledgementsForBundledSoftware')}
            </span>
          </span>
          <ChevronRight className="theme-text-muted h-4 w-4 shrink-0 rtl:-scale-x-100" />
        </button>
      </section>

      <OpenSourceLicensesDialog isOpen={licensesOpen} onClose={() => setLicensesOpen(false)} />
    </div>
  );
}
