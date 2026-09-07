export type SetupStep = 'welcome' | 'migration' | 'privacy' | 'hotkey' | 'ready';
interface WelcomeFeatures { enableBackups: boolean; enableHotkeys: boolean }
const STEPS: readonly SetupStep[] = ['welcome', 'migration', 'privacy', 'hotkey', 'ready'];

export function welcomeSetupSteps(features: WelcomeFeatures): SetupStep[] {
  return STEPS.filter(step => (step !== 'migration' || features.enableBackups)
    && (step !== 'hotkey' || features.enableHotkeys));
}

export function visibleWelcomeStep(step: SetupStep, features: WelcomeFeatures): SetupStep {
  if (step === 'migration' && !features.enableBackups) return 'privacy';
  if (step === 'hotkey' && !features.enableHotkeys) return 'ready';
  return step;
}
