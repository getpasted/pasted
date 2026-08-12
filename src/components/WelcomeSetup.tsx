import { useEffect, useMemo, useState } from 'react';
import { ArrowLeft, ArrowRight, Check, ClipboardCheck, Command, FolderKanban, HardDrive, ListOrdered, LockKeyhole, ShieldCheck, Sparkles, WandSparkles } from 'lucide-react';
import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { ExternalHistoryImport, type ExternalImportReport } from './ExternalHistoryImport';
import { PastedMark } from './PastedMark';
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
            <div className="welcome-setup-mark" aria-hidden="true">
              <PastedMark />
              <span><Sparkles /></span>
            </div>
            <p className="welcome-setup-kicker">Your clipboard, kept close</p>
            <h1 id="pasted-welcome-title">Welcome to Pasted</h1>
            <p>
              Keep the things you copy, find them again quickly, and build a clipboard library that stays on your computer.
            </p>
            <ul className="welcome-setup-highlights" aria-label="Pasted highlights">
              <li><HardDrive /> <span>Local library</span></li>
              <li><LockKeyhole /> <span>Private by default</span></li>
              <li><Command /> <span>Keyboard ready</span></li>
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
                <p>If you used another clipboard manager, Pasted can merge supported text history into this library.</p>
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
                <p className="welcome-setup-kicker">Know what is saved</p>
                <h1 id="pasted-welcome-title">A local library with visible controls</h1>
                <p>Pasted stores clipboard history in its SQLite library. You decide when capture pauses, what apps are ignored, and when history is removed.</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <div className="welcome-setup-facts">
                <article>
                  <HardDrive />
                  <div><strong>Stored locally</strong><span>Your clipboard library stays on this computer unless you export or move it.</span></div>
                </article>
                <article>
                  <LockKeyhole />
                  <div><strong>Sensitive apps excluded</strong><span>Password managers are included in the starter ignore list.</span></div>
                </article>
                <article>
                  <ShieldCheck />
                  <div><strong>Reversible cleanup</strong><span>Trash, protection, and backups provide safer ways to manage history.</span></div>
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
                <p className="welcome-setup-kicker">Work your way</p>
                <h1 id="pasted-welcome-title">A few ways to move faster</h1>
                <p>Reach your history from anywhere, organize it automatically, or reshape text before it leaves Pasted.</p>
              </div>
            </div>
            <div className="welcome-setup-content">
              <div className="welcome-setup-shortcut-card">
                <span className="welcome-setup-keycap">{shortcutLabel}</span>
                <div>
                  <strong>Quick HUD</strong>
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
                  <FolderKanban />
                  <div>
                    <strong>Organize itself</strong>
                    <span>Use Bins and smart rules to gather clips by app, type, content, or workflow.</span>
                  </div>
                </article>
                <article>
                  <WandSparkles />
                  <div>
                    <strong>Transform before pasting</strong>
                    <span>Clean, format, extract, or combine text with reusable Operations and Pipelines.</span>
                  </div>
                </article>
              </div>
            </div>
          </div>
        )}

        {step === 'ready' && (
          <div className="welcome-setup-hero">
            <div className="welcome-setup-mark is-ready" aria-hidden="true"><Check /></div>
            <p className="welcome-setup-kicker">Setup complete</p>
            <h1 id="pasted-welcome-title">Your library is ready</h1>
            <p>
              {importedCount > 0
                ? `${importedCount} clips came with you. New copies will appear at the top of History.`
                : 'Copy something new and it will appear at the top of History.'}
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
