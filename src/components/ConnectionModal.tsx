import { useState, type FormEvent } from 'react';
import { BrainCircuit } from 'lucide-react';
import type { IntelligenceProviderKind } from '../types';
import { INTELLIGENCE_PROVIDERS } from '../utils/intelligenceProviders';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';

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

  const isDirty = name !== 'Local AI'
    || providerKind !== 'ollama'
    || endpoint !== INTELLIGENCE_PROVIDERS[0].endpoint
    || Boolean(model || credentialEnvironmentVariable);

  return (
    <AppDialog
      isOpen
      onClose={onClose}
      labelledBy="connection-modal-title"
      isDirty={isDirty}
      panelClassName="theme-panel border rounded-2xl w-full max-w-2xl shadow-2xl overflow-hidden"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} closeLabel="Close add connection dialog">
          <AppDialogHeading id="connection-modal-title" title="Add connection" description="Add a provider, local endpoint, or executable that was not detected automatically." icon={<BrainCircuit />} />
        </AppDialogHeader>

        <form onSubmit={createConnection}>
          <AppDialogBody className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Engine</span>
              <MenuSelect
                value={providerKind}
                options={INTELLIGENCE_PROVIDERS.map((provider) => ({ value: provider.value, label: provider.label }))}
                onChange={(value) => selectProvider(value as IntelligenceProviderKind)}
                label="Connection engine"
                className="w-full"
              />
            </div>
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Connection name</span>
              <input autoFocus value={name} onChange={(event) => setName(event.target.value)} className="theme-input ui-field-radius border px-3 py-2.5 w-full" />
            </label>
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Endpoint or executable</span>
              <input value={endpoint} onChange={(event) => setEndpoint(event.target.value)} placeholder={providerKind === 'cli' ? '/usr/local/bin/my-planner' : 'http://127.0.0.1:11434'} className="theme-input ui-field-radius border px-3 py-2.5 w-full font-mono" />
            </label>
            <label className="text-xs theme-text-muted space-y-1.5">
              <span className="block font-semibold">Preferred model</span>
              <input value={model} onChange={(event) => setModel(event.target.value)} placeholder="Optional until model discovery" className="theme-input ui-field-radius border px-3 py-2.5 w-full font-mono" />
            </label>
            {!['ollama', 'lm_studio', 'cli'].includes(providerKind) && (
              <label className="text-xs theme-text-muted space-y-1.5 md:col-span-2">
                <span className="block font-semibold">Credential environment variable</span>
                <input value={credentialEnvironmentVariable} onChange={(event) => setCredentialEnvironmentVariable(event.target.value)} placeholder="OPENAI_API_KEY" className="theme-input ui-field-radius border px-3 py-2.5 w-full font-mono" />
                <span className="block text-[10px]">Only the variable name is saved. The secret remains with the provider or operating system.</span>
              </label>
            )}
            {error && <div className="theme-status-danger border rounded-xl px-3 py-2 text-xs md:col-span-2">{error}</div>}
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose}>Cancel</AppDialogButton>
            <AppDialogButton type="submit" variant="primary" disabled={!name.trim() || isSaving}>
              <SaveButtonContent isSaving={isSaving} />
            </AppDialogButton>
          </AppDialogFooter>
        </form>
      </>}
    </AppDialog>
  );
}
