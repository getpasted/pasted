import { useEffect, useRef, useState } from 'react';
import { BrainCircuit, Cloud, Cpu, Plus, Power, Terminal, Trash2 } from 'lucide-react';
import type { DetectedIntelligenceConnection, IntelligenceConnection } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import { intelligenceProviderLabel } from '../utils/intelligenceProviders';
import { ConnectionModal } from './ConnectionModal';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { OverflowText } from './OverflowText';
import { ActionButton } from './AppDialogLayout';
import { translate } from '../localization/runtime';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { SettingsPanelResetNote } from './SettingsPanelResetNote';
import { SettingsResetChanges } from './SettingsResetChanges';
import { intelligenceResetChanges } from '../intelligenceResetChanges';

let cachedConnections: IntelligenceConnection[] | null = null;
let cachedDetectedConnections: DetectedIntelligenceConnection[] | null = null;
let cachedDetectionAt = 0;
const DETECTION_CACHE_MS = 30_000;

export function IntelligenceConnectionsPanel() {
  const [connections, setConnections] = useState<IntelligenceConnection[]>(() => cachedConnections ?? []);
  const [detectedConnections, setDetectedConnections] = useState<DetectedIntelligenceConnection[]>(() => cachedDetectedConnections ?? []);
  const [isAddConnectionOpen, setIsAddConnectionOpen] = useState(false);
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(cachedConnections === null);
  const [hasDetectionResult, setHasDetectionResult] = useState(cachedDetectedConnections !== null);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const connectionListRef = useRef<HTMLDivElement>(null);

  const {
    activeId: activeConnectionId,
    offsets: connectionOffsets,
    isSettling: isConnectionSettling,
    startPointerReorder: startConnectionReorder,
  } = useStableVerticalReorder({
    itemIds: connections.map((connection) => connection.id),
    containerRef: connectionListRef,
    onCommit: (orderedIds) => {
      const byId = new Map(connections.map((connection) => [connection.id, connection]));
      setConnections(orderedIds.map((id, priority) => ({ ...byId.get(id)!, priority })));
      invoke('reorder_intelligence_connections', { ids: orderedIds }).catch((reason) => {
        setError(String(reason));
        refresh();
      });
    },
  });

  const refresh = () => (
    invoke<IntelligenceConnection[]>('get_intelligence_connections')
      .then((nextConnections) => {
        cachedConnections = nextConnections;
        setConnections(nextConnections);
      })
      .catch((reason) => setError(String(reason)))
  );

  useEffect(() => {
    let cancelled = false;
    const loadConnections = async () => {
      // Stored connections are local SQLite data and can paint immediately.
      // Discovery/version checks refresh quietly afterward.
      await refresh();
      if (cancelled) return;
      if (cachedDetectedConnections && Date.now() - cachedDetectionAt < DETECTION_CACHE_MS) {
        setDetectedConnections(cachedDetectedConnections);
        setHasDetectionResult(true);
        setIsLoading(false);
        return;
      }
      try {
        const detected = await invoke<DetectedIntelligenceConnection[]>('detect_intelligence_connections');
        if (cancelled) return;
        cachedDetectedConnections = detected;
        cachedDetectionAt = Date.now();
        setDetectedConnections(detected);
        setHasDetectionResult(true);
        await refresh();
      } catch (reason) {
        if (!cancelled) setError(String(reason));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };
    void loadConnections();
    return () => {
      cancelled = true;
    };
  }, []);

  const detectedEndpoint = (connection: DetectedIntelligenceConnection) =>
    connection.providerKind === 'cli' ? connection.executablePath : connection.defaultEndpoint;

  const detectionForConnection = (connection: IntelligenceConnection) =>
    detectedConnections.find((detected) =>
      detected.providerKind === connection.providerKind
      && detectedEndpoint(detected) === connection.endpoint
    );

  const toggleConnection = async (connection: IntelligenceConnection) => {
    await invoke('update_intelligence_connection', {
      id: connection.id,
      name: connection.name,
      providerKind: connection.providerKind,
      endpoint: connection.endpoint,
      model: connection.model,
      credentialRef: connection.credentialRef,
      enabled: !connection.enabled,
    });
    refresh();
  };

  const deleteConnection = async (id: string) => {
    await invoke('delete_intelligence_connection', { id });
    refresh();
  };

  const requestReset = () => {
    const changes = intelligenceResetChanges(connections, detectedConnections);
    setConfirmation({
      title: translate('component.intelligenceConnectionsPanel.resetIntelligence'),
      description: translate('component.settingsResetChanges.description'),
      details: <SettingsResetChanges changes={changes} />,
      confirmLabel: translate('common.reset'),
      confirmDisabled: changes.length === 0,
      onConfirm: async () => {
        const reset = await invoke<IntelligenceConnection[]>('reset_intelligence_connections');
        cachedConnections = reset;
        setConnections(reset);
        setConfirmation(null);
      },
    });
  };

  return (
    <div className="space-y-5">
      <SettingsPanelHeader
        icon={BrainCircuit}
        title={translate('component.intelligenceConnectionsPanel.intelligence')}
        description={translate('component.intelligenceConnectionsPanel.manageLocalAndRemoteIntelligenceProviders')}
        actions={(
          <ActionButton variant="primary" onClick={() => setIsAddConnectionOpen(true)}>
            <Plus className="w-4 h-4" />
            <span>{translate('component.intelligenceConnectionsPanel.addConnection')}</span>
          </ActionButton>
        )}
      />
      <div className="space-y-5">
        <section className="space-y-2.5 min-w-0">
          {connections.length > 0 ? (
            <div ref={connectionListRef} className={`stable-reorder-list space-y-2 ${isConnectionSettling ? 'is-settling-stable-reorder' : ''}`}>
              {connections.map((connection, index) => {
                const detected = detectionForConnection(connection);
                const isLocal = ['ollama', 'lm_studio', 'cli'].includes(connection.providerKind);
                const Icon = connection.providerKind === 'cli' ? Terminal : isLocal ? Cpu : Cloud;
                const isDragging = activeConnectionId === connection.id;
                const offset = connectionOffsets[connection.id] ?? 0;
                const isInteractiveOnly = detected?.capabilities.includes('interactive_chat') && !detected.capabilities.includes('structured_output');
                const executionUnavailable = detected?.executionSupported === false;
                const isOperational = connection.enabled && !executionUnavailable;
                const canDelete = hasDetectionResult && !detected;
                return (
                  <article
                    key={connection.id}
                    data-stable-reorder-id={connection.id}
                    onPointerDown={(event) => startConnectionReorder(connection.id, event)}
                    title={translate('component.intelligenceConnectionsPanel.reorderConnection')}
                    style={offset !== 0 || isDragging ? { transform: `translateY(${offset}px)`, zIndex: isDragging ? 'var(--layer-drag)' : 1 } : undefined}
                    className={`connection-priority-card theme-card-idle border p-3 flex items-center justify-between gap-3 relative cursor-grab active:cursor-grabbing touch-none transition-[background-color,border-color,box-shadow,opacity,transform] duration-100 ${isDragging ? 'is-dragging' : ''} ${isOperational ? '' : 'opacity-60'}`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="theme-text-muted font-mono text-[10px] w-4 text-center">{index + 1}</span>
                      <span className="theme-badge border rounded-xl p-2"><Icon className="w-4 h-4" /></span>
                      <div className="min-w-0">
                        <OverflowText as="div" text={connection.name} className="theme-text-main text-xs font-bold truncate" />
                        <OverflowText
                          as="div"
                          text={connection.model
                            ? translate('component.intelligenceConnectionsPanel.versionModel', {
                              version: detected?.version || intelligenceProviderLabel(connection.providerKind),
                              model: connection.model,
                            })
                            : detected?.version || intelligenceProviderLabel(connection.providerKind)}
                          className="theme-text-muted text-[10px] truncate mt-0.5"
                        />
                        {isInteractiveOnly && <div className="theme-text-muted text-[9px] mt-0.5">{translate('component.intelligenceConnectionsPanel.interactiveMcpClientNotAutomaticFallback')}</div>}
                        {executionUnavailable && !isInteractiveOnly && <div className="theme-text-muted text-[9px] mt-0.5">{translate('component.intelligenceConnectionsPanel.detectedAutomaticExecutionUnavailable')}</div>}
                        <OverflowText as="div" text={connection.endpoint || translate('component.intelligenceConnectionsPanel.noEndpointConfigured')} className="theme-text-muted text-[10px] font-mono truncate mt-1" />
                      </div>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button type="button" onPointerDown={(event) => event.stopPropagation()} onClick={() => toggleConnection(connection)} disabled={executionUnavailable} className={`connection-power-button theme-icon-button border rounded-lg p-2 ${isOperational ? 'is-enabled' : ''}`} title={executionUnavailable ? translate('component.intelligenceConnectionsPanel.automaticExecutionUnavailable') : connection.enabled ? translate('component.intelligenceConnectionsPanel.disable') : translate('component.intelligenceConnectionsPanel.enable')}>
                        <Power className="w-4 h-4" />
                      </button>
                      <button
                        type="button"
                        onPointerDown={(event) => event.stopPropagation()}
                        onClick={() => canDelete && deleteConnection(connection.id)}
                        disabled={!canDelete}
                        tabIndex={canDelete ? 0 : -1}
                        aria-hidden={!canDelete}
                        className={`theme-icon-button theme-danger-text border rounded-lg p-2 ${canDelete ? '' : 'invisible pointer-events-none'}`}
                        title={canDelete ? translate('component.intelligenceConnectionsPanel.deleteConnection') : undefined}
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>
          ) : isLoading ? (
            <div className="connection-loading-list space-y-2" role="status" aria-label={translate('component.intelligenceConnectionsPanel.detectingAvailableIntelligenceConnections')} aria-busy="true">
              {[0, 1, 2].map((index) => (
                <div key={index} className="connection-loading-card theme-card-idle border p-3 flex items-center justify-between gap-3" aria-hidden="true">
                  <div className="flex min-w-0 flex-1 items-center gap-3">
                    <span className="connection-loading-rank" />
                    <span className="connection-loading-icon rounded-xl" />
                    <span className="min-w-0 flex-1 space-y-2">
                      <span className="connection-loading-line is-name" />
                      <span className="connection-loading-line is-detail" />
                    </span>
                  </div>
                  <span className="connection-loading-action rounded-lg" />
                </div>
              ))}
              <span className="sr-only">{translate('component.intelligenceConnectionsPanel.detectingAvailableIntelligence')}</span>
            </div>
          ) : (
            <div className="theme-text-muted border border-dashed rounded-2xl p-6 text-center text-xs">
              {translate('component.intelligenceConnectionsPanel.noCompatibleLocalToolsOrAddedConnectionsYet')}
            </div>
          )}
        </section>

      </div>

      {error && <div className="theme-status-danger border rounded-xl px-3 py-2 text-xs">{error}</div>}

      <SettingsPanelResetNote onReset={requestReset} disabled={isLoading}>
        {translate('component.intelligenceConnectionsPanel.resetIntelligenceNote')}
      </SettingsPanelResetNote>
      <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />

      {isAddConnectionOpen && (
        <ConnectionModal
          onClose={() => setIsAddConnectionOpen(false)}
          onCreated={refresh}
        />
      )}
    </div>
  );
}
