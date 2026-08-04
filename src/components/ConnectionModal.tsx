import { useEffect, useRef, useState, type FormEvent } from 'react';
import { BrainCircuit, X } from 'lucide-react';
import type { IntelligenceProviderKind } from '../types';
import { INTELLIGENCE_PROVIDERS } from '../utils/intelligenceProviders';
import { safeInvoke as invoke } from '../utils/tauri';

interface ConnectionModalProps {
  onClose: () => void;
  onCreated: () => void | Promise<void>;
}

function errorMessage(reason: unknown) {
  if (reason && typeof reason === 'object' && 'message' in reason) return String(reason.message);
  return String(reason);
}

export function ConnectionModal({ onClose, onCreated }: ConnectionModalProps) {
  const [name, setName] = useState('Local AI');
  const [providerKind, setProviderKind] = useState<IntelligenceProviderKind>('ollama');
  const [endpoint, setEndpoint] = useState(INTELLIGENCE_PROVIDERS[0].endpoint);
  const [model, setModel] = useState('');
  const [credentialEnvironmentVariable, setCredentialEnvironmentVariable] = useState('');
  const [error, setError] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previousFocus = document.activeElement as HTMLElement | null;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== 'Tab' || !modalRef.current) return;
      const focusable = modalRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      previousFocus?.focus();
    };
  }, [onClose]);

  const selectProvider = (kind: IntelligenceProviderKind) => {
    const provider = INTELLIGENCE_PROVIDERS.find((candidate) => candidate.value === kind)!;
    setProviderKind(kind);
    setEndpoint(provider.endpoint);
    setModel(provider.model);
    setName(provider.local ? `Local ${provider.label}` : provider.label);
    setCredentialEnvironmentVariable('');
  };

  const createConnection = async (event: FormEvent) => {
    event.preventDefault();
    if (!name.trim() || isSaving) return;
    setError('');
    setIsSaving(true);
    try {
      await invoke('create_intelligence_connection', {
        name: name.trim(),
        providerKind,
        endpoint: endpoint.trim() || null,
        model: model.trim() || null,
        credentialRef: credentialEnvironmentVariable.trim()
          ? `env:${credentialEnvironmentVariable.trim()}`
          : null,
      });
      await onCreated();
      onClose();
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div
      className="app-dialog-overlay fixed inset-0 flex items-center justify-center p-4 animate-in fade-in duration-150"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={modalRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="connection-modal-title"
        className="app-dialog-panel theme-panel border rounded-2xl w-full max-w-2xl shadow-2xl overflow-hidden"
      >
        <div className="theme-divider border-b px-5 py-4 flex items-start justify-between gap-4">
          <div className="flex items-start gap-3">
            <span className="theme-badge border rounded-xl p-2.5 shrink-0"><BrainCircuit className="w-5 h-5" /></span>
            <div>
              <h2 id="connection-modal-title" className="theme-title text-sm font-bold">Add connection</h2>
              <p className="theme-text-muted text-xs mt-1">Add a provider, local endpoint, or executable Pasted could not detect.</p>
            </div>
          </div>
          <button type="button" onClick={onClose} className="theme-icon-button border rounded-lg p-2" aria-label="Close add connection dialog">
            <X className="w-4 h-4" />
          </button>
        </div>

        <form onSubmit={createConnection}>
          <div className="p-5 grid grid-cols-1 md:grid-cols-2 gap-4">
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Engine</span>
              <select value={providerKind} onChange={(event) => selectProvider(event.target.value as IntelligenceProviderKind)} className="theme-input border rounded-xl px-3 py-2.5 w-full">
                {INTELLIGENCE_PROVIDERS.map((provider) => <option key={provider.value} value={provider.value}>{provider.label}</option>)}
              </select>
            </label>
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Connection name</span>
              <input autoFocus value={name} onChange={(event) => setName(event.target.value)} className="theme-input border rounded-xl px-3 py-2.5 w-full" />
            </label>
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Endpoint or executable</span>
              <input value={endpoint} onChange={(event) => setEndpoint(event.target.value)} placeholder={providerKind === 'cli' ? '/usr/local/bin/my-planner' : 'http://127.0.0.1:11434'} className="theme-input border rounded-xl px-3 py-2.5 w-full font-mono" />
            </label>
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Preferred model</span>
              <input value={model} onChange={(event) => setModel(event.target.value)} placeholder="Optional until model discovery" className="theme-input border rounded-xl px-3 py-2.5 w-full font-mono" />
            </label>
            {!['ollama', 'lm_studio', 'cli'].includes(providerKind) && (
              <label className="text-xs theme-text-muted space-y-1.5 md:col-span-2">
                <span className="block font-semibold">Credential environment variable</span>
                <input value={credentialEnvironmentVariable} onChange={(event) => setCredentialEnvironmentVariable(event.target.value)} placeholder="OPENAI_API_KEY" className="theme-input border rounded-xl px-3 py-2.5 w-full font-mono" />
                <span className="block text-[10px]">Only the variable name is saved. The secret remains with the provider or operating system.</span>
              </label>
            )}
            {error && <div className="theme-status-danger border rounded-xl px-3 py-2 text-xs md:col-span-2">{error}</div>}
          </div>
          <div className="theme-divider border-t px-5 py-4 flex justify-end gap-2">
            <button type="button" onClick={onClose} className="theme-secondary-button border rounded-xl px-4 py-2 text-xs font-semibold">Cancel</button>
            <button type="submit" disabled={!name.trim() || isSaving} className="theme-primary-button border rounded-xl px-4 py-2 text-xs font-semibold disabled:opacity-40">
              {isSaving ? 'Saving…' : 'Save connection'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
