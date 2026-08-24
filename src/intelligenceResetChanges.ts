import type { DetectedIntelligenceConnection, IntelligenceConnection } from './types';
import { translate } from './localization/runtime';
import { resetBooleanLabel, type SettingsResetChange } from './components/SettingsResetChanges';

function detectedEndpoint(connection: DetectedIntelligenceConnection) {
  return connection.providerKind === 'cli' ? connection.executablePath : connection.defaultEndpoint;
}

export function intelligenceResetChanges(
  connections: IntelligenceConnection[],
  detected: DetectedIntelligenceConnection[],
): SettingsResetChange[] {
  const ordered = intelligenceDefaultOrder(connections, detected);
  return connections.flatMap((connection) => {
    const nextPriority = ordered.indexOf(connection);
    const changes: SettingsResetChange[] = [];
    if (connection.enabled) changes.push({
      label: translate('format.labelStatus', { label: connection.name, status: translate('common.enabled') }),
      before: resetBooleanLabel(true),
      after: resetBooleanLabel(false),
    });
    if (connection.priority !== nextPriority) changes.push({
      label: translate('format.labelStatus', { label: connection.name, status: translate('common.priority') }),
      before: String(connection.priority + 1),
      after: String(nextPriority + 1),
    });
    return changes;
  });
}

function intelligenceDefaultOrder(connections: IntelligenceConnection[], detected: DetectedIntelligenceConnection[]) {
  const ordered: IntelligenceConnection[] = [];
  for (const candidate of detected) {
    const connection = connections.find((item) => item.providerKind === candidate.providerKind
      && item.endpoint === detectedEndpoint(candidate));
    if (connection && !ordered.includes(connection)) ordered.push(connection);
  }
  ordered.push(...connections.filter((connection) => !ordered.includes(connection)));
  return ordered;
}
