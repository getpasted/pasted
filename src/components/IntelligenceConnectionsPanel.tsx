import { useEffect, useRef, useState } from 'react';
import { BrainCircuit, Cloud, Cpu, Plus, Power, Terminal, Trash2 } from 'lucide-react';
import type { DetectedIntelligenceConnection, IntelligenceConnection } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import { intelligenceProviderLabel } from '../utils/intelligenceProviders';
import { ConnectionModal } from './ConnectionModal';

let cachedConnections: IntelligenceConnection[] | null = null;
let cachedDetectedConnections: DetectedIntelligenceConnection[] | null = null;

export function IntelligenceConnectionsPanel() {
  const [connections, setConnections] = useState<IntelligenceConnection[]>(() => cachedConnections ?? []);
  const [detectedConnections, setDetectedConnections] = useState<DetectedIntelligenceConnection[]>(() => cachedDetectedConnections ?? []);
  const [isAddConnectionOpen, setIsAddConnectionOpen] = useState(false);
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(cachedConnections === null);
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
        setIsLoading(false);
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
      try {
        const detected = await invoke<DetectedIntelligenceConnection[]>('detect_intelligence_connections');
        if (cancelled) return;
        cachedDetectedConnections = detected;
        setDetectedConnections(detected);
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

  return (
    <div className="space-y-5">
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
                return (
                  <article
                    key={connection.id}
                    data-stable-reorder-id={connection.id}
                    onPointerDown={(event) => startConnectionReorder(connection.id, event)}
                    title="Reorder Connection"
                    style={offset !== 0 || isDragging ? { transform: `translateY(${offset}px)`, zIndex: isDragging ? 'var(--layer-drag)' : 1 } : undefined}
                    className={`connection-priority-card theme-card-idle border rounded-xl p-3 flex items-center justify-between gap-3 relative cursor-grab active:cursor-grabbing touch-none transition-[background-color,border-color,box-shadow,opacity,transform] duration-100 ${isDragging ? 'is-dragging' : ''} ${isOperational ? '' : 'opacity-60'}`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="theme-text-muted font-mono text-[10px] w-4 text-center">{index + 1}</span>
                      <span className="theme-badge border rounded-xl p-2"><Icon className="w-4 h-4" /></span>
                      <div className="min-w-0">
                        <div className="theme-text-main text-xs font-bold truncate">{connection.name}</div>
                        <div className="theme-text-muted text-[10px] truncate mt-0.5">
                          {detected?.version || intelligenceProviderLabel(connection.providerKind)}{connection.model ? ` · ${connection.model}` : ''}
                        </div>
                        {isInteractiveOnly && <div className="theme-text-muted text-[9px] mt-0.5">Interactive/MCP client · not automatic fallback</div>}
                        {executionUnavailable && !isInteractiveOnly && <div className="theme-text-muted text-[9px] mt-0.5">Detected · execution adapter coming later</div>}
                        <div className="theme-text-muted text-[10px] font-mono truncate mt-1">{connection.endpoint || 'No endpoint configured'}</div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                      <button type="button" onPointerDown={(event) => event.stopPropagation()} onClick={() => toggleConnection(connection)} disabled={executionUnavailable} className={`connection-power-button theme-icon-button border rounded-lg p-2 ${isOperational ? 'is-enabled' : ''}`} title={executionUnavailable ? 'Execution adapter coming later' : connection.enabled ? 'Disable' : 'Enable'}>
                        <Power className="w-4 h-4" />
                      </button>
                      {!detected && (
                        <button type="button" onPointerDown={(event) => event.stopPropagation()} onClick={() => deleteConnection(connection.id)} className="theme-icon-button theme-danger-text border rounded-lg p-2" title="Delete Connection">
                          <Trash2 className="w-4 h-4" />
                        </button>
                      )}
                    </div>
                  </article>
                );
              })}
            </div>
          ) : (
            <div className="theme-text-muted border border-dashed rounded-2xl p-6 text-center text-xs">
              {isLoading ? 'Detecting available intelligence…' : 'No compatible local tools or added connections yet.'}
            </div>
          )}
        </section>

        <aside className="theme-surface border rounded-2xl p-5 space-y-4">
          <div className="flex items-start gap-3">
            <span className="theme-badge border rounded-xl p-2.5 shrink-0"><BrainCircuit className="w-5 h-5" /></span>
            <div className="min-w-0">
              <h2 className="theme-title text-sm font-bold">Bring your own intelligence</h2>
              <p className="theme-text-muted text-xs mt-1 leading-relaxed">
                Add a remote provider, local endpoint, or executable Pasted could not detect.
              </p>
            </div>
          </div>
          <p className="theme-text-muted text-[10px] leading-relaxed">
            Pasted saves connection details and credential references, never API keys. Secrets stay with the provider or operating system.
          </p>
          <button type="button" onClick={() => setIsAddConnectionOpen(true)} className="theme-primary-button border rounded-xl px-3 py-2.5 text-xs font-semibold flex items-center justify-center gap-1.5 w-full">
            <Plus className="w-4 h-4" />
            <span>Add connection</span>
          </button>
        </aside>
      </div>

      {error && <div className="theme-status-danger border rounded-xl px-3 py-2 text-xs">{error}</div>}

      {isAddConnectionOpen && (
        <ConnectionModal
          onClose={() => setIsAddConnectionOpen(false)}
          onCreated={refresh}
        />
      )}
    </div>
  );
}
