import { createContext, useContext, type ReactNode } from 'react';
import { FEATURE_DEFINITIONS, type FeatureId } from '../utils/features';

const ALL_FEATURES = Object.fromEntries(
  FEATURE_DEFINITIONS.map(({ id }) => [id, true]),
) as Record<FeatureId, boolean>;

const FeatureContext = createContext<Record<FeatureId, boolean>>(ALL_FEATURES);

export function FeatureProvider({
  features,
  children,
}: {
  features: Record<FeatureId, boolean>;
  children: ReactNode;
}) {
  return <FeatureContext.Provider value={features}>{children}</FeatureContext.Provider>;
}

export function useFeatures() {
  return useContext(FeatureContext);
}
