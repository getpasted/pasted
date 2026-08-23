import { AppShellView } from './components/AppShellView';
import { QuickHudWindow } from './components/QuickHudWindow';
import { useAppController } from './hooks/useAppController';
import { FeatureProvider } from './hooks/useFeatures';
import './App.css';

export default function App() {
  const controller = useAppController();
  const { enabledFeatures, isHudView } = controller.shell;

  return (
    <FeatureProvider features={enabledFeatures}>
      {isHudView ? <QuickHudWindow /> : <AppShellView controller={controller} />}
    </FeatureProvider>
  );
}
