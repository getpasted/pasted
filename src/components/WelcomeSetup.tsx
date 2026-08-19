import { useEffect, useMemo, useState } from 'react';
import { ArrowLeft, ArrowRight, Bot, Check, ClipboardCheck, Command, Database, ExternalLink, HardDrive, HeartHandshake, ListOrdered, LockKeyhole, Monitor, RadioTower, ShieldCheck, TerminalSquare, Workflow } from 'lucide-react';
import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { ExternalHistoryImport, type ExternalImportReport } from './ExternalHistoryImport';
import { CopycatHeadMark } from './CopycatMark';
import { ActionButton } from './AppDialogLayout';
import { translate } from '../localization/runtime';

const ONBOARDING_VERSION = 1;

type SetupStep = 'welcome' | 'migration' | 'privacy' | 'hotkey' | 'ready';

const STEPS: SetupStep[] = ['welcome', 'migration', 'privacy', 'hotkey', 'ready'];

interface HotkeyCapabilityStatus {
  platform: string;
  state: string;
  is_trusted: boolean;
  configured_count: number;
  registered_count: number;
}

interface WelcomeSetupProps {
  isOpen: boolean;
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
  onImported: () => void | Promise<void>;
}

export function WelcomeSetup({ isOpen, settings, onUpdateSettings, onImported }: WelcomeSetupProps) {
  const steps = useMemo(
    () => settings.enableHotkeys ? STEPS : STEPS.filter((candidate) => candidate !== 'hotkey'),
    [settings.enableHotkeys],
  );
  const [step, setStep] = useState<SetupStep>('welcome');
  const [permission, setPermission] = useState<HotkeyCapabilityStatus | null>(null);
  const [importedCount, setImportedCount] = useState(0);
  const [backingError, setBackingError] = useState('');
  const stepIndex = steps.indexOf(step);

  useEffect(() => {
    if (!isOpen) return;
    setStep('welcome');
    setImportedCount(0);
    setBackingError('');
  }, [isOpen]);

  useEffect(() => {
    if (!settings.enableHotkeys && step === 'hotkey') setStep('ready');
  }, [settings.enableHotkeys, step]);

  useEffect(() => {
    if (!isOpen || step !== 'hotkey') return undefined;
    let cancelled = false;
    const refresh = () => invoke<HotkeyCapabilityStatus>('get_hotkey_capability_status')
      .then((status) => {
        if (!cancelled) setPermission(status);
      })
      .catch(console.error);
    void refresh();
    const timer = window.setInterval(() => void refresh(), 1500);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [isOpen, step]);

  const hotkeyLabel = useMemo(() => {
    const value = settings.hudHotkey || 'Alt+Shift+V';
    return value
      .replace('Alt', navigator.platform.includes('Mac') ? '⌥' : 'Alt')
      .replace('Shift', '⇧')
      .replace('Super', navigator.platform.includes('Mac') ? '⌘' : 'Win')
      .replace(/\+/g, ' ');
  }, [settings.hudHotkey]);

  const complete = () => {
    onUpdateSettings({ onboardingVersion: ONBOARDING_VERSION });
  };

  const advance = () => {
    if (step === 'ready') {
      complete();
      return;
    }
    setStep(steps[Math.min(steps.length - 1, stepIndex + 1)]);
  };

  const goBack = () => setStep(steps[Math.max(0, stepIndex - 1)]);
  const requestPermission = async () => {
    try {
      await invoke('request_accessibility_permission');
    } catch (error) {
      console.error('Could not open permission settings:', error);
    }
  };
  const openBackingPage = async () => {
    setBackingError('');
    try {
      await invoke('open_backing_page');
    } catch (error) {
      setBackingError(String(error));
    }
  };
  const handleImported = async (report: ExternalImportReport) => {
    setImportedCount((current) => current + report.importedCount);
    await onImported();
  };

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={complete}
      labelledBy="pasted-welcome-title"
      overlayClassName="welcome-setup-overlay p-5"
      panelClassName="welcome-setup-panel theme-panel w-full max-w-3xl overflow-hidden rounded-2xl border shadow-2xl"
    >
      <div className="welcome-setup-progress" aria-label={translate('component.welcomeSetup.setupStepValueOfLength', { value: stepIndex + 1, length: steps.length })}>
        {steps.map((candidate, index) => (
          <span key={candidate} className={index <= stepIndex ? 'is-complete' : ''} />
        ))}
      </div>

      <div className="welcome-setup-body">
        {step === 'welcome' && (
          <div className="welcome-setup-hero">
            <div className="welcome-setup-mark is-copycat is-mirrored" aria-hidden="true">
              <CopycatHeadMark />
            </div>
            <p className="welcome-setup-kicker">{translate('component.welcomeSetup.thePrivateLocalClipboardWorkspace')}</p>
            <h1 id="pasted-welcome-title">{translate('component.welcomeSetup.welcomeCopycat')}</h1>
            <p>
              {translate('component.welcomeSetup.pastedGivesYouYourScriptsAndYourAgentsOneSharedPlaceTo')}
            </p>
            <ul className="welcome-setup-highlights" aria-label={translate('component.welcomeSetup.pastedHighlights')}>
              <li><HardDrive /> <span>{translate('component.welcomeSetup.noCloudAccount')}</span></li>
              <li><RadioTower /> <span>{translate('component.welcomeSetup.noOffDeviceTelemetry')}</span></li>
              <li><HeartHandshake /> <span>{translate('component.welcomeSetup.noSubscription')}</span></li>
            </ul>
          </div>
        )}

        {step === 'migration' && (
          <div className="welcome-setup-step">
            <div className="welcome-setup-heading">
              <span className="welcome-setup-heading-icon"><ClipboardCheck /></span>
              <div>
                <p className="welcome-setup-kicker">{translate('component.welcomeSetup.bringYourHistory')}</p>
                <h1 id="pasted-welcome-title">{translate('component.welcomeSetup.startWhereYouLeftOff')}</h1>
                <p>{translate('component.welcomeSetup.supportedTextHistoryFromAnotherClipboardManagerCanBeMergedIntoHistory')}</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <ExternalHistoryImport compact onImported={handleImported} />
            </div>
          </div>
        )}

        {step === 'privacy' && (
          <div className="welcome-setup-step">
            <div className="welcome-setup-heading">
              <span className="welcome-setup-heading-icon"><ShieldCheck /></span>
              <div>
                <p className="welcome-setup-kicker">{translate('component.welcomeSetup.theCopycatCovenant')}</p>
                <h1 id="pasted-welcome-title">{translate('component.welcomeSetup.yourClipboardIsNoneOfOurBusiness')}</h1>
                <p>{translate('component.welcomeSetup.pastedWorksForYouWithoutTurningYourWorkIntoOurDataThese')}</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <div className="welcome-setup-facts">
                <article>
                  <HardDrive />
                  <div><strong>{translate('component.welcomeSetup.localByDefault')}</strong><span>{translate('component.welcomeSetup.yourClipboardLibraryStaysOnThisComputerUnlessYouExplicitlyExportMove')}</span></div>
                </article>
                <article>
                  <RadioTower />
                  <div><strong>{translate('component.welcomeSetup.noOffDeviceTelemetry')}</strong><span>{translate('component.welcomeSetup.usageInsightsStayOnThisComputerPastedDoesNotReportClipboardActivity')}</span></div>
                </article>
                <article>
                  <HeartHandshake />
                  <div><strong>{translate('component.welcomeSetup.noSubscription')}</strong><span>{translate('component.welcomeSetup.pastedWillNotRentYourOwnClipboardBackToYouOrMake')}</span></div>
                </article>
                <article>
                  <LockKeyhole />
                  <div><strong>{translate('component.welcomeSetup.connectionsAreAChoice')}</strong><span>{translate('component.welcomeSetup.outsideIntelligenceRunsOnlyThroughAConnectionYouExplicitlyEnableAndUse')}</span></div>
                </article>
              </div>
            </div>
          </div>
        )}

        {settings.enableHotkeys && step === 'hotkey' && (
          <div className="welcome-setup-step welcome-setup-hotkey-step">
            <div className="welcome-setup-heading">
              <span className="welcome-setup-heading-icon"><Command /></span>
              <div>
                <p className="welcome-setup-kicker">{translate('component.welcomeSetup.everyCopycatWelcome')}</p>
                <h1 id="pasted-welcome-title">{translate('component.welcomeSetup.oneWorkspaceMoreThanOneWayIn')}</h1>
                <p>{translate('component.welcomeSetup.useTheInterfaceDirectlyTheCliFromAScriptOrTheShared')}</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <div className="welcome-setup-hotkey-card">
                <span className="welcome-setup-keycap">{hotkeyLabel}</span>
                <div>
                  <strong>{translate('component.welcomeSetup.hud')}</strong>
                  <span>{permission?.state === 'ready' && permission.is_trusted ? translate('component.welcomeSetup.hotkeyAccessIsReady') : translate('component.welcomeSetup.youCanChangeThisHotkeyLaterInSettingsHotkeys')}</span>
                </div>
                {permission?.platform === 'macos' && !permission.is_trusted && (
                  <ActionButton onClick={() => void requestPermission()}>
                    {translate('component.welcomeSetup.allowAccessibility')}
                  </ActionButton>
                )}
                {permission && (permission.platform !== 'macos' || permission.is_trusted) && (
                  <span className="welcome-setup-ready-badge"><Check /> {translate('component.welcomeSetup.ready')}</span>
                )}
              </div>
              <div className="welcome-setup-feature-grid" aria-label={translate('component.welcomeSetup.morePastedFeatures')}>
                <article>
                  <ListOrdered />
                  <div>
                    <strong>{translate('component.welcomeSetup.pasteInSequence')}</strong>
                    <span>{translate('component.welcomeSetup.buildAQueueThenPasteEachItemIntoFormsOrRepetitiveWorkflows')}</span>
                  </div>
                </article>
                <article>
                  <TerminalSquare />
                  <div>
                    <strong>{translate('component.welcomeSetup.scriptTheSameWorkspace')}</strong>
                    <span>{translate('component.welcomeSetup.theBundledCliSearchesOrganizesTransformsAndReturnsStructuredOutputFromThe')}</span>
                  </div>
                </article>
                <article>
                  <Bot />
                  <div>
                    <strong>{translate('component.welcomeSetup.bringYourOwnIntelligence')}</strong>
                    <span>{translate('component.welcomeSetup.agentsAndOptionalProvidersCanHelpOnlyWhenYouConnectThemAnd')}</span>
                  </div>
                </article>
              </div>
              <div className="welcome-setup-shared-library">
                <div className="welcome-setup-library-hub">
                  <Database />
                  <div>
                    <strong>{translate('component.welcomeSetup.oneLocalLibraryUnderneathItAll')}</strong>
                    <span>{translate('component.welcomeSetup.historyBinsNotesAndWorkflowsStayConsistentFromEveryEntryPoint')}</span>
                  </div>
                </div>
                <div className="welcome-setup-library-routes" aria-label={translate('component.welcomeSetup.sharedLibraryEntryPoints')}>
                  <span><Monitor /> {translate('component.welcomeSetup.interface')}</span>
                  <span><TerminalSquare /> {translate('component.welcomeSetup.cli')}</span>
                  <span><Workflow /> {translate('component.welcomeSetup.automations')}</span>
                  <span><Bot /> {translate('component.welcomeSetup.agents')}</span>
                </div>
              </div>
            </div>
          </div>
        )}

        {step === 'ready' && (
          <div className="welcome-setup-hero">
            <div className="welcome-setup-mark is-copycat is-ready" aria-hidden="true"><CopycatHeadMark /></div>
            <p className="welcome-setup-kicker">{translate('component.welcomeSetup.copycatStatusReady')}</p>
            <h1 id="pasted-welcome-title">{translate('component.welcomeSetup.goCopyIrresponsibly')}</h1>
            <p>
              {importedCount > 0
                ? translate('component.welcomeSetup.countClipsCameWithYouHumanAndMachineCopycatsCanFindThem', { count: importedCount })
                : translate('component.welcomeSetup.copySomethingNewPastedWillRememberItWithoutSendingItAnywhere')}
            </p>
            <div className="welcome-setup-backing">
              <HeartHandshake />
              <div>
                <strong>{translate('component.welcomeSetup.keepTheCopycatCopying')}</strong>
                <span>{translate('component.welcomeSetup.nothingToUnlockJustUsefulSoftwareAndOneMoreReasonToKeep')}</span>
              </div>
              <ActionButton onClick={() => void openBackingPage()}>
                {translate('component.welcomeSetup.backPasted999')} <ExternalLink />
              </ActionButton>
            </div>
            {backingError && <div role="alert" className="theme-status-danger mt-3 rounded-xl border px-3 py-2 text-xs">{backingError}</div>}
          </div>
        )}
      </div>

      <footer className="welcome-setup-footer">
        <div>
          {stepIndex > 0 && (
            <ActionButton onClick={goBack}>
              <ArrowLeft className="rtl:-scale-x-100" /> {translate('common.back')}
            </ActionButton>
          )}
          {step !== 'ready' && (
            <button type="button" className="welcome-setup-skip" onClick={complete}>{translate('component.welcomeSetup.skipSetup')}</button>
          )}
        </div>
        <ActionButton autoFocus={step === 'welcome'} variant="primary" onClick={advance}>
          {step === 'welcome' ? translate('component.welcomeSetup.setUpPasted') : step === 'ready' ? translate('component.welcomeSetup.openPasted') : translate('component.welcomeSetup.continue')}
          {step !== 'ready' && <ArrowRight className="rtl:-scale-x-100" />}
        </ActionButton>
      </footer>
    </AppDialog>
  );
}
