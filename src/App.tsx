import { AppShellView } from './components/AppShellView';
import { useAppController } from './hooks/useAppController';
import { FeatureProvider } from './hooks/useFeatures';
import './App.css';

export default function App() {
  const controller = useAppController();
  const { enabledFeatures } = controller.shell;

  return (
    <FeatureProvider features={enabledFeatures}>
      <AppShellView controller={controller} />
    </FeatureProvider>
  );
}
