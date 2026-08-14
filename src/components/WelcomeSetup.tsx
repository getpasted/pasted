import { useEffect, useMemo, useState } from 'react';
import { ArrowLeft, ArrowRight, Bot, Check, ClipboardCheck, Command, HardDrive, HeartHandshake, ListOrdered, LockKeyhole, RadioTower, ShieldCheck, TerminalSquare } from 'lucide-react';
import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { ExternalHistoryImport, type ExternalImportReport } from './ExternalHistoryImport';
import { CopycatMark } from './CopycatMark';
import { ActionButton } from './AppDialogLayout';

const ONBOARDING_VERSION = 1;

type SetupStep = 'welcome' | 'migration' | 'privacy' | 'shortcut' | 'ready';

const STEPS: SetupStep[] = ['welcome', 'migration', 'privacy', 'shortcut', 'ready'];

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
  const [step, setStep] = useState<SetupStep>('welcome');
  const [permission, setPermission] = useState<HotkeyCapabilityStatus | null>(null);
  const [importedCount, setImportedCount] = useState(0);
  const stepIndex = STEPS.indexOf(step);

  useEffect(() => {
    if (!isOpen) return;
    setStep('welcome');
    setImportedCount(0);
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || step !== 'shortcut') return undefined;
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

  const shortcutLabel = useMemo(() => {
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
    setStep(STEPS[Math.min(STEPS.length - 1, stepIndex + 1)]);
  };

  const goBack = () => setStep(STEPS[Math.max(0, stepIndex - 1)]);
  const requestPermission = async () => {
    try {
      await invoke('request_accessibility_permission');
    } catch (error) {
      console.error('Could not open permission settings:', error);
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
      <div className="welcome-setup-progress" aria-label={`Setup step ${stepIndex + 1} of ${STEPS.length}`}>
        {STEPS.map((candidate, index) => (
          <span key={candidate} className={index <= stepIndex ? 'is-complete' : ''} />
        ))}
      </div>

      <div className="welcome-setup-body">
        {step === 'welcome' && (
          <div className="welcome-setup-hero">
            <div className="welcome-setup-mark is-copycat" aria-hidden="true">
              <CopycatMark />
            </div>
            <p className="welcome-setup-kicker">The private, local clipboard workspace</p>
            <h1 id="pasted-welcome-title">Welcome, copycat.</h1>
            <p>
              Pasted gives you, your scripts, and your agents one shared place to remember, organize, and reshape everything you copy.
            </p>
            <ul className="welcome-setup-highlights" aria-label="Pasted highlights">
              <li><HardDrive /> <span>No cloud account</span></li>
              <li><RadioTower /> <span>No off-device telemetry</span></li>
              <li><HeartHandshake /> <span>No subscription</span></li>
            </ul>
          </div>
        )}

        {step === 'migration' && (
          <div className="welcome-setup-step">
            <div className="welcome-setup-heading">
              <span className="welcome-setup-heading-icon"><ClipboardCheck /></span>
              <div>
                <p className="welcome-setup-kicker">Bring your history</p>
                <h1 id="pasted-welcome-title">Start where you left off</h1>
                <p>Supported text history from another clipboard manager can be merged into History.</p>
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
                <p className="welcome-setup-kicker">The Copycat Covenant</p>
                <h1 id="pasted-welcome-title">Your clipboard is none of our business</h1>
                <p>Pasted works for you without turning your work into our data. These are product constraints, not marketing preferences.</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <div className="welcome-setup-facts">
                <article>
                  <HardDrive />
                  <div><strong>Local by default</strong><span>Your clipboard library stays on this computer unless you explicitly export, move, or send something.</span></div>
                </article>
                <article>
                  <RadioTower />
                  <div><strong>No off-device telemetry</strong><span>Usage insights stay on this computer; Pasted does not report clipboard activity.</span></div>
                </article>
                <article>
                  <HeartHandshake />
                  <div><strong>No subscription</strong><span>Pasted will not rent your own clipboard back to you or make payment a condition of remembering.</span></div>
                </article>
                <article>
                  <LockKeyhole />
                  <div><strong>Connections are a choice</strong><span>Outside intelligence runs only through a connection you explicitly enable and use.</span></div>
                </article>
              </div>
            </div>
          </div>
        )}

        {step === 'shortcut' && (
          <div className="welcome-setup-step welcome-setup-shortcut-step">
            <div className="welcome-setup-heading">
              <span className="welcome-setup-heading-icon"><Command /></span>
              <div>
                <p className="welcome-setup-kicker">Every copycat welcome</p>
                <h1 id="pasted-welcome-title">One workspace, more than one way in</h1>
                <p>Use the interface directly, the CLI from a script, or the shared library as context for an agent.</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <div className="welcome-setup-shortcut-card">
                <span className="welcome-setup-keycap">{shortcutLabel}</span>
                <div>
                  <strong>HUD</strong>
                  <span>{permission?.state === 'ready' && permission.is_trusted ? 'Shortcut access is ready.' : 'You can change this shortcut later in Settings → Hotkeys.'}</span>
                </div>
                {permission?.platform === 'macos' && !permission.is_trusted && (
                  <ActionButton onClick={() => void requestPermission()}>
                    Allow Accessibility
                  </ActionButton>
                )}
                {permission && (permission.platform !== 'macos' || permission.is_trusted) && (
                  <span className="welcome-setup-ready-badge"><Check /> Ready</span>
                )}
              </div>
              <div className="welcome-setup-feature-grid" aria-label="More Pasted features">
                <article>
                  <ListOrdered />
                  <div>
                    <strong>Paste in sequence</strong>
                    <span>Build a Queue, then paste each item into forms or repetitive workflows in order.</span>
                  </div>
                </article>
                <article>
                  <TerminalSquare />
                  <div>
                    <strong>Script the same workspace</strong>
                    <span>The bundled CLI searches, organizes, transforms, and returns structured output from the same local data.</span>
                  </div>
                </article>
                <article>
                  <Bot />
                  <div>
                    <strong>Bring your own intelligence</strong>
                    <span>Agents and optional providers can help only when you connect them and ask them to.</span>
                  </div>
                </article>
              </div>
            </div>
          </div>
        )}

        {step === 'ready' && (
          <div className="welcome-setup-hero">
            <div className="welcome-setup-mark is-copycat is-ready" aria-hidden="true"><CopycatMark /></div>
            <p className="welcome-setup-kicker">Copycat status: ready</p>
            <h1 id="pasted-welcome-title">Go copy irresponsibly.</h1>
            <p>
              {importedCount > 0
                ? `${importedCount} clips came with you. Human and machine copycats can find them in the same local workspace.`
                : 'Copy something new. Pasted will remember it without sending it anywhere.'}
            </p>
          </div>
        )}
      </div>

      <footer className="welcome-setup-footer">
        <div>
          {stepIndex > 0 && (
            <ActionButton onClick={goBack}>
              <ArrowLeft /> Back
            </ActionButton>
          )}
          {step !== 'ready' && (
            <button type="button" className="welcome-setup-skip" onClick={complete}>Skip setup</button>
          )}
        </div>
        <ActionButton autoFocus={step === 'welcome'} variant="primary" onClick={advance}>
          {step === 'welcome' ? 'Set Up Pasted' : step === 'ready' ? 'Open Pasted' : 'Continue'}
          {step !== 'ready' && <ArrowRight />}
        </ActionButton>
      </footer>
    </AppDialog>
  );
}
