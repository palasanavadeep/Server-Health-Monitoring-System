"use client";

import { useState, useMemo } from 'react';
import { notFound } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { 
    useClientApiKeysQuery, 
    useCreateApiKeyMutation,
    useUpdateApiKeyMutation,
    useDeleteApiKeyMutation,
    useDeactivateApiKeyMutation,
    useActivateApiKeyMutation,
    useRotateApiKeyMutation
} from '@/hooks/use-client-queries';
import { ApiKey } from '@/lib/api';
import { StatusBadge } from '@/components/ui/status-badge';
import { Modal } from '@/components/ui/modal';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { 
    KeyRound, 
    Plus, 
    Copy, 
    Check, 
    Loader2, 
    Search, 
    RefreshCw, 
    Power, 
    Trash2, 
    Save, 
    Sliders,
    Building,
    Eye,
    EyeOff
} from 'lucide-react';
import { cn } from '@/lib/utils';

function formatShortApiKey(keyValue?: string, prefix?: string): string {
    if (!keyValue && !prefix) return '—';
    const key = keyValue || prefix || '';
    if (key.startsWith('sm_key_')) {
        const rest = key.slice(7);
        const cleaned = rest.replace(/\*/g, '');
        if (cleaned.length >= 8) {
            return `sm_key_${cleaned.slice(0, 4)}...${cleaned.slice(-4)}`;
        }
        if (rest.length >= 8) {
            return `sm_key_${rest.slice(0, 4)}...${rest.slice(-4)}`;
        }
        return `sm_key_${rest}`;
    }
    if (key.length > 16) {
        return `${key.slice(0, 8)}...${key.slice(-4)}`;
    }
    return key;
}

export default function ApiKeysPage() {
    const toast = useToast();
    const { user, loading } = useAuth();
    
    // Route guard: super_admin does not have access to client API keys workspace
    if (!loading && user && user.role === 'super_admin') {
        notFound();
    }
    
    const isClientAdmin = user?.role === 'client_admin';
    const canCreateKeys = isClientAdmin;
    const selectedClientId = user?.clientId || '';

    // Modals
    const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
    const [isSecretGeneratedModalOpen, setIsSecretGeneratedModalOpen] = useState(false);
    const [generatedKey, setGeneratedKey] = useState<string | null>(null);
    const [copied, setCopied] = useState(false);

    // Selected key for detail / edit modal
    const [selectedKeyForDetails, setSelectedKeyForDetails] = useState<ApiKey | null>(null);

    // Search query
    const [searchQuery, setSearchQuery] = useState('');

    // Creation Form inputs
    const [newKeyName, setNewKeyName] = useState('');
    const [newKeyDesc, setNewKeyDesc] = useState('');
    const [newKeyEnv, setNewKeyEnv] = useState<'production' | 'staging' | 'development' | 'testing'>('production');
    const [newKeyExpires, setNewKeyExpires] = useState<number>(1440);
    const [newKeyCanIngest, setNewKeyCanIngest] = useState(true);
    const [newKeyCanRead, setNewKeyCanRead] = useState(false);
    const [newKeyServices, setNewKeyServices] = useState('');
    const [newKeyIPs, setNewKeyIPs] = useState('0.0.0.0/0');
    const [newKeyOrigins, setNewKeyOrigins] = useState('*');
    const [newKeyWarnDays, setNewKeyWarnDays] = useState<number>(30);

    // Edit Form inputs
    const [editKeyName, setEditKeyName] = useState('');
    const [editKeyDesc, setEditKeyDesc] = useState('');
    const [editKeyEnv, setEditKeyEnv] = useState<'production' | 'staging' | 'development' | 'testing'>('production');
    const [editKeyCanIngest, setEditKeyCanIngest] = useState(true);
    const [editKeyCanRead, setEditKeyCanRead] = useState(false);
    const [editKeyServices, setEditKeyServices] = useState('');
    const [editKeyIPs, setEditKeyIPs] = useState('');
    const [editKeyOrigins, setEditKeyOrigins] = useState('');
    const [editKeyWarnDays, setEditKeyWarnDays] = useState<number>(30);

    // Queries & mutations
    const { data: apiKeys = [], isLoading: loadingKeys, refetch } = useClientApiKeysQuery(selectedClientId);
    
    const createKeyMutation = useCreateApiKeyMutation(selectedClientId);
    const updateKeyMutation = useUpdateApiKeyMutation(selectedClientId);
    const deleteKeyMutation = useDeleteApiKeyMutation(selectedClientId);
    const deactivateKeyMutation = useDeactivateApiKeyMutation(selectedClientId);
    const activateKeyMutation = useActivateApiKeyMutation(selectedClientId);
    const rotateKeyMutation = useRotateApiKeyMutation(selectedClientId);

    // Filtered keys
    const filteredApiKeys = useMemo(() => {
        if (!searchQuery.trim()) return apiKeys;
        const q = searchQuery.toLowerCase();
        return apiKeys.filter(k => 
            k.name.toLowerCase().includes(q) ||
            (k.description && k.description.toLowerCase().includes(q)) ||
            (k.environment && k.environment.toLowerCase().includes(q)) ||
            (k.prefix && k.prefix.toLowerCase().includes(q))
        );
    }, [apiKeys, searchQuery]);

    const handleCreateKey = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newKeyName.trim() || !selectedClientId) return;

        const ipsArray = newKeyIPs.split(',').map(s => s.trim()).filter(Boolean);
        const originsArray = newKeyOrigins.split(',').map(s => s.trim()).filter(Boolean);
        const servicesArray = newKeyServices.split(',').map(s => s.trim()).filter(Boolean);

        try {
            const res = await createKeyMutation.mutateAsync({
                name: newKeyName.trim(),
                description: newKeyDesc.trim() || undefined,
                environment: newKeyEnv,
                expiresAt: Number(newKeyExpires) || 1440,
                permissions: {
                    canIngest: newKeyCanIngest,
                    canReadAnalytics: newKeyCanRead,
                    allowedServices: servicesArray
                },
                security: {
                    allowedIPs: ipsArray.length ? ipsArray : undefined,
                    allowedOrigins: originsArray.length ? originsArray : undefined,
                    rotationWarningDays: Number(newKeyWarnDays) || 30
                }
            });

            setGeneratedKey(res.keyValue || res.key || 'Failed to retrieve raw key');
            setIsCreateModalOpen(false);
            setIsSecretGeneratedModalOpen(true);
            
            // Reset fields
            setNewKeyName('');
            setNewKeyDesc('');
            setNewKeyEnv('production');
            setNewKeyExpires(1440);
            setNewKeyCanIngest(true);
            setNewKeyCanRead(false);
            setNewKeyServices('');
            setNewKeyIPs('0.0.0.0/0');
            setNewKeyOrigins('*');
            setNewKeyWarnDays(30);
            toast('API Key generated successfully', 'success');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to generate API Key', 'error');
        }
    };

    const handleCopyKey = (keyText: string) => {
        if (!keyText) return;
        navigator.clipboard.writeText(keyText);
        setCopied(true);
        toast('API Key copied to clipboard', 'success');
        setTimeout(() => setCopied(false), 2000);
    };

    const handleToggleKeyStatus = async (keyId: string, isActive: boolean) => {
        try {
            if (isActive) {
                await deactivateKeyMutation.mutateAsync(keyId);
                toast('API Key deactivated', 'info');
                if (selectedKeyForDetails?.keyId === keyId || selectedKeyForDetails?.id === keyId) {
                    setSelectedKeyForDetails(prev => prev ? { ...prev, isActive: false } : null);
                }
            } else {
                await activateKeyMutation.mutateAsync(keyId);
                toast('API Key activated', 'success');
                if (selectedKeyForDetails?.keyId === keyId || selectedKeyForDetails?.id === keyId) {
                    setSelectedKeyForDetails(prev => prev ? { ...prev, isActive: true } : null);
                }
            }
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to toggle status', 'error');
        }
    };

    const handleRotateKey = async (keyId: string) => {
        if (!confirm("Are you sure you want to rotate this API Key? Existing requests using the previous secret will fail immediately.")) {
            return;
        }

        try {
            const res = await rotateKeyMutation.mutateAsync(keyId);
            setGeneratedKey(res.keyValue || res.key || 'Failed to retrieve rotated token');
            setSelectedKeyForDetails(null);
            setIsSecretGeneratedModalOpen(true);
            toast('API Key rotated successfully', 'success');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to rotate key', 'error');
        }
    };

    const handleOpenEdit = (key: ApiKey) => {
        setSelectedKeyForDetails(key);
        setEditKeyName(key.name);
        setEditKeyDesc(key.description || '');
        setEditKeyEnv((key.environment || 'production') as any);
        setEditKeyCanIngest(key.permissions?.canIngest !== false);
        setEditKeyCanRead(key.permissions?.canReadAnalytics === true);
        setEditKeyServices(key.permissions?.allowedServices?.join(', ') || '');
        setEditKeyIPs(key.security?.allowedIPs?.join(', ') || '0.0.0.0/0');
        setEditKeyOrigins(key.security?.allowedOrigins?.join(', ') || '*');
        setEditKeyWarnDays(key.security?.rotationWarningDays || 30);
    };

    const handleSaveEdit = async (e: React.FormEvent) => {
        e.preventDefault();
        const keyId = selectedKeyForDetails?.keyId || selectedKeyForDetails?.id;
        if (!editKeyName.trim() || !keyId) return;

        const ipsArray = editKeyIPs.split(',').map(s => s.trim()).filter(Boolean);
        const originsArray = editKeyOrigins.split(',').map(s => s.trim()).filter(Boolean);
        const servicesArray = editKeyServices.split(',').map(s => s.trim()).filter(Boolean);

        try {
            await updateKeyMutation.mutateAsync({
                keyId,
                name: editKeyName.trim(),
                description: editKeyDesc.trim() || undefined,
                environment: editKeyEnv,
                permissions: {
                    canIngest: editKeyCanIngest,
                    canReadAnalytics: editKeyCanRead,
                    allowedServices: servicesArray
                },
                security: {
                    allowedIPs: ipsArray,
                    allowedOrigins: originsArray,
                    rotationWarningDays: Number(editKeyWarnDays) || 30
                }
            });
            setSelectedKeyForDetails(null);
            toast('API Key configuration updated', 'success');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to update key', 'error');
        }
    };

    const handleDeleteKey = async (keyId: string) => {
        if (!confirm("Are you sure you want to delete this API Key permanently?")) {
            return;
        }

        try {
            await deleteKeyMutation.mutateAsync(keyId);
            setSelectedKeyForDetails(null);
            toast('API Key deleted permanently', 'info');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to delete key', 'error');
        }
    };

    if (loading || (user && user.role === 'super_admin')) {
        return null;
    }

    return (
        <div className="space-y-6 max-w-7xl mx-auto pb-12 animate-in fade-in duration-150">
            {/* Page Header with Compact Workspace Context */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#242932]">
                <div>
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                        API Keys
                    </h1>
                    <div className="flex items-center gap-2 mt-0.5 text-xs text-zinc-400">
                        <span>Manage telemetry ingestion credentials and network scopes.</span>
                        <span className="text-zinc-600">•</span>
                        <span className="font-mono text-zinc-300">
                            Workspace: {selectedClientId.substring(0, 10)}...
                        </span>
                    </div>
                </div>

                {canCreateKeys && (
                    <Button
                        size="sm"
                        onClick={() => setIsCreateModalOpen(true)}
                        className="text-xs h-8 gap-1.5 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold self-start sm:self-auto"
                    >
                        <Plus size={14} />
                        Create API Key
                    </Button>
                )}
            </div>

            {/* Flat Toolbar */}
            <div className="flex items-center justify-between gap-4 text-xs">
                <div className="relative max-w-sm w-full">
                    <Search size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                    <input
                        type="text"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        placeholder="Search API keys by name or environment..."
                        className="w-full bg-[#111419] border border-[#242932] rounded px-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                    />
                </div>

                <span className="text-xs text-zinc-500 font-mono">
                    {filteredApiKeys.length} {filteredApiKeys.length === 1 ? 'key' : 'keys'}
                </span>
            </div>

            {/* Dense Flat Data Table */}
            <div className="surface-panel overflow-hidden">
                {loadingKeys ? (
                    <div className="flex items-center justify-center py-12">
                        <Loader2 className="animate-spin text-[#4CB8D6] w-5 h-5" />
                    </div>
                ) : filteredApiKeys.length === 0 ? (
                    <div className="text-center py-12 p-6">
                        <KeyRound className="w-8 h-8 text-zinc-600 mx-auto mb-2" />
                        <p className="text-xs font-semibold text-zinc-300">No API Keys Configured</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mx-auto mt-0.5">
                            {searchQuery ? "No keys matched your search term." : "Create an API Key to enable microservice telemetry ingestion."}
                        </p>
                        {canCreateKeys && !searchQuery && (
                            <Button 
                                size="sm" 
                                onClick={() => setIsCreateModalOpen(true)}
                                className="mt-3 text-xs h-7 gap-1 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                            >
                                <Plus size={12} />
                                Create API Key
                            </Button>
                        )}
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-xs border-collapse">
                            <thead>
                                <tr className="border-b border-[#242932] text-zinc-400 text-[11px] uppercase tracking-wider bg-[#0E1014]/60 select-none">
                                    <th className="py-2.5 px-4 font-semibold w-24">Status</th>
                                    <th className="py-2.5 px-4 font-semibold">Key Name</th>
                                    <th className="py-2.5 px-4 font-semibold">Environment</th>
                                    <th className="py-2.5 px-4 font-semibold">API Key / Token</th>
                                    <th className="py-2.5 px-4 font-semibold">Created</th>
                                    <th className="py-2.5 px-4 font-semibold text-right w-28">Actions</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[#242932]">
                                {filteredApiKeys.map((key, idx) => (
                                    <tr 
                                        key={key.id || key.keyId || idx}
                                        onClick={() => handleOpenEdit(key)}
                                        className="hover:bg-[#181D24] transition-colors cursor-pointer group"
                                    >
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <StatusBadge status={key.isActive ? 'healthy' : 'offline'} label={key.isActive ? 'Active' : 'Disabled'} />
                                        </td>
                                        <td className="py-3 px-4 font-medium text-zinc-200 whitespace-nowrap">
                                            <span className="group-hover:text-[#4CB8D6] transition-colors">
                                                {key.name}
                                            </span>
                                            {key.description && (
                                                <span className="block text-[11px] text-zinc-500 font-normal">
                                                    {key.description}
                                                </span>
                                            )}
                                        </td>
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <span className="text-[10px] font-mono uppercase bg-[#181D24] text-zinc-300 px-1.5 py-0.5 rounded border border-[#242932]">
                                                {key.environment || 'production'}
                                            </span>
                                        </td>
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <span className="font-mono text-zinc-300 text-[11px] bg-[#0E1014] px-2 py-0.5 rounded border border-[#242932]">
                                                {formatShortApiKey(key.keyValue, key.prefix)}
                                            </span>
                                        </td>
                                        <td className="py-3 px-4 text-zinc-400 whitespace-nowrap font-mono text-[11px]">
                                            {new Date(key.createdAt).toLocaleDateString()}
                                        </td>
                                        <td className="py-3 px-4 text-right whitespace-nowrap" onClick={(e) => e.stopPropagation()}>
                                            {canCreateKeys ? (
                                                <div className="flex items-center justify-end gap-1.5">
                                                    <button
                                                        onClick={() => handleToggleKeyStatus(key.keyId || key.id, key.isActive)}
                                                        title={key.isActive ? "Deactivate key" : "Activate key"}
                                                        className={cn(
                                                            "p-1.5 rounded hover:bg-white/5 transition-colors cursor-pointer",
                                                            key.isActive ? "text-[#D99A3D]" : "text-[#48B982]"
                                                        )}
                                                    >
                                                        <Power size={13} />
                                                    </button>
                                                    <button
                                                        onClick={() => handleRotateKey(key.keyId || key.id)}
                                                        title="Rotate key"
                                                        className="p-1.5 rounded hover:bg-white/5 text-zinc-400 hover:text-[#4CB8D6] transition-colors cursor-pointer"
                                                    >
                                                        <RefreshCw size={13} />
                                                    </button>
                                                    <button
                                                        onClick={() => handleDeleteKey(key.keyId || key.id)}
                                                        title="Delete key"
                                                        className="p-1.5 rounded hover:bg-white/5 text-zinc-400 hover:text-[#E45865] transition-colors cursor-pointer"
                                                    >
                                                        <Trash2 size={13} />
                                                    </button>
                                                </div>
                                            ) : (
                                                <span className="text-[10px] text-zinc-500 font-mono">Read Only</span>
                                            )}
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>

            {/* Semantic Create API Key Modal */}
            <Modal
                isOpen={isCreateModalOpen}
                onClose={() => setIsCreateModalOpen(false)}
                title="Create API Key"
                description="Generate an ingestion token with scoped permissions and network filters."
                maxWidth="max-w-xl"
            >
                <form onSubmit={handleCreateKey} className="space-y-4">
                    {/* General Section */}
                    <div className="space-y-3">
                        <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider block">1. General Details</span>
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                            <div className="space-y-1">
                                <label className="text-xs text-zinc-300 block">Key Name *</label>
                                <input
                                    type="text"
                                    value={newKeyName}
                                    onChange={(e) => setNewKeyName(e.target.value)}
                                    placeholder="e.g. gateway-production"
                                    required
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                                />
                            </div>

                            <div className="space-y-1">
                                <label className="text-xs text-zinc-300 block">Environment</label>
                                <select
                                    value={newKeyEnv}
                                    onChange={(e: any) => setNewKeyEnv(e.target.value)}
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                                >
                                    <option value="production">Production</option>
                                    <option value="staging">Staging</option>
                                    <option value="development">Development</option>
                                    <option value="testing">Testing</option>
                                </select>
                            </div>
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs text-zinc-300 block">Description (Optional)</label>
                            <input
                                type="text"
                                value={newKeyDesc}
                                onChange={(e) => setNewKeyDesc(e.target.value)}
                                placeholder="Describe service or team..."
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                        </div>
                    </div>

                    {/* Permissions Section */}
                    <div className="space-y-2 pt-2 border-t border-[#242932]">
                        <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider block">2. Permissions Scope</span>
                        <div className="flex items-center gap-6">
                            <label className="flex items-center gap-2 text-xs text-zinc-300 cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={newKeyCanIngest}
                                    onChange={(e) => setNewKeyCanIngest(e.target.checked)}
                                    className="rounded bg-[#0E1014] border-[#242932] text-[#4CB8D6]"
                                />
                                Ingest telemetry events
                            </label>
                            <label className="flex items-center gap-2 text-xs text-zinc-300 cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={newKeyCanRead}
                                    onChange={(e) => setNewKeyCanRead(e.target.checked)}
                                    className="rounded bg-[#0E1014] border-[#242932] text-[#4CB8D6]"
                                />
                                Read analytics metrics
                            </label>
                        </div>
                    </div>

                    {/* Network Restrictions Section */}
                    <div className="space-y-3 pt-2 border-t border-[#242932]">
                        <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider block">3. Network Restrictions</span>
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                            <div className="space-y-1">
                                <label className="text-xs text-zinc-300 block">Allowed IP Ranges (CIDR)</label>
                                <input
                                    type="text"
                                    value={newKeyIPs}
                                    onChange={(e) => setNewKeyIPs(e.target.value)}
                                    placeholder="0.0.0.0/0 (all IPs)"
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 font-mono focus:outline-none focus:border-[#4CB8D6]"
                                />
                            </div>

                            <div className="space-y-1">
                                <label className="text-xs text-zinc-300 block">Allowed Origins</label>
                                <input
                                    type="text"
                                    value={newKeyOrigins}
                                    onChange={(e) => setNewKeyOrigins(e.target.value)}
                                    placeholder="* or https://app.domain.com"
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 font-mono focus:outline-none focus:border-[#4CB8D6]"
                                />
                            </div>
                        </div>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center justify-end gap-2 pt-4 border-t border-[#242932]">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setIsCreateModalOpen(false)}
                            className="text-xs h-8 cursor-pointer"
                        >
                            Cancel
                        </Button>
                        <Button
                            type="submit"
                            size="sm"
                            isLoading={createKeyMutation.isPending}
                            className="text-xs h-8 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer"
                        >
                            Create Key
                        </Button>
                    </div>
                </form>
            </Modal>

            {/* Secret Token Generated Modal */}
            <Modal
                isOpen={isSecretGeneratedModalOpen}
                onClose={() => {
                    setIsSecretGeneratedModalOpen(false);
                    setGeneratedKey(null);
                }}
                title="API Key Created"
                description="Copy and securely store your raw secret token. It will not be shown again."
                maxWidth="max-w-md"
            >
                <div className="space-y-4">
                    <div className="p-3 rounded bg-[#0E1014] border border-[#242932] flex items-center justify-between gap-2">
                        <code className="text-xs font-mono text-[#4CB8D6] select-all break-all flex-1">
                            {generatedKey}
                        </code>
                        <Button
                            size="sm"
                            variant="outline"
                            onClick={() => handleCopyKey(generatedKey || '')}
                            className="h-7 px-2.5 text-xs gap-1 cursor-pointer shrink-0"
                        >
                            {copied ? <Check size={12} className="text-[#48B982]" /> : <Copy size={12} />}
                            {copied ? 'Copied' : 'Copy'}
                        </Button>
                    </div>

                    <div className="flex justify-end pt-2">
                        <Button
                            size="sm"
                            onClick={() => {
                                setIsSecretGeneratedModalOpen(false);
                                setGeneratedKey(null);
                            }}
                            className="text-xs h-8 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                        >
                            Done
                        </Button>
                    </div>
                </div>
            </Modal>

            {/* Edit Key Modal */}
            {selectedKeyForDetails && (
                <Modal
                    isOpen={!!selectedKeyForDetails}
                    onClose={() => setSelectedKeyForDetails(null)}
                    title={`Key Settings: ${selectedKeyForDetails.name}`}
                    description="Update environment, IP filters, or rotate secret."
                    maxWidth="max-w-lg"
                >
                    <form onSubmit={handleSaveEdit} className="space-y-4">
                        <div className="grid grid-cols-2 gap-3">
                            <div className="space-y-1">
                                <label className="text-xs text-zinc-300 block">Key Name</label>
                                <input
                                    type="text"
                                    value={editKeyName}
                                    onChange={(e) => setEditKeyName(e.target.value)}
                                    required
                                    disabled={!canCreateKeys}
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                                />
                            </div>
                            <div className="space-y-1">
                                <label className="text-xs text-zinc-300 block">Environment</label>
                                <select
                                    value={editKeyEnv}
                                    onChange={(e: any) => setEditKeyEnv(e.target.value)}
                                    disabled={!canCreateKeys}
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                                >
                                    <option value="production">Production</option>
                                    <option value="staging">Staging</option>
                                    <option value="development">Development</option>
                                    <option value="testing">Testing</option>
                                </select>
                            </div>
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs text-zinc-300 block">Description (Optional)</label>
                            <input
                                type="text"
                                value={editKeyDesc}
                                onChange={(e) => setEditKeyDesc(e.target.value)}
                                disabled={!canCreateKeys}
                                placeholder="Purpose of this key, service or deployment..."
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                        </div>

                        {/* Scopes & Permissions */}
                        <div className="space-y-2 pt-1 border-t border-[#242932]">
                            <label className="text-xs font-medium text-zinc-300 block">Key Permissions & Scopes</label>
                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                                <label className={cn(
                                    "flex items-start gap-2 p-2.5 rounded border transition-colors cursor-pointer",
                                    editKeyCanIngest ? "bg-[#4CB8D6]/10 border-[#4CB8D6]/30" : "bg-[#0E1014] border-[#242932]",
                                    !canCreateKeys && "opacity-60 pointer-events-none"
                                )}>
                                    <input
                                        type="checkbox"
                                        checked={editKeyCanIngest}
                                        onChange={(e) => setEditKeyCanIngest(e.target.checked)}
                                        disabled={!canCreateKeys}
                                        className="mt-0.5 rounded bg-zinc-900 border-zinc-700 text-[#4CB8D6] focus:ring-0"
                                    />
                                    <div>
                                        <p className="text-xs font-medium text-zinc-200">Can Ingest Metrics</p>
                                        <p className="text-[10px] text-zinc-400">Allows sending hit & metric telemetry to /hit</p>
                                    </div>
                                </label>

                                <label className={cn(
                                    "flex items-start gap-2 p-2.5 rounded border transition-colors cursor-pointer",
                                    editKeyCanRead ? "bg-[#4CB8D6]/10 border-[#4CB8D6]/30" : "bg-[#0E1014] border-[#242932]",
                                    !canCreateKeys && "opacity-60 pointer-events-none"
                                )}>
                                    <input
                                        type="checkbox"
                                        checked={editKeyCanRead}
                                        onChange={(e) => setEditKeyCanRead(e.target.checked)}
                                        disabled={!canCreateKeys}
                                        className="mt-0.5 rounded bg-zinc-900 border-zinc-700 text-[#4CB8D6] focus:ring-0"
                                    />
                                    <div>
                                        <p className="text-xs font-medium text-zinc-200">Can Read Analytics</p>
                                        <p className="text-[10px] text-zinc-400">Allows query access to metrics & statistics</p>
                                    </div>
                                </label>
                            </div>
                        </div>

                        {/* Security / Network Filters */}
                        <div className="space-y-3 pt-1 border-t border-[#242932]">
                            <label className="text-xs font-medium text-zinc-300 block">Security & Origin Filters</label>
                            
                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                                <div className="space-y-1">
                                    <label className="text-xs text-zinc-300 block">Allowed IP Filters (CIDR)</label>
                                    <input
                                        type="text"
                                        value={editKeyIPs}
                                        onChange={(e) => setEditKeyIPs(e.target.value)}
                                        disabled={!canCreateKeys}
                                        placeholder="0.0.0.0/0 (all IPs)"
                                        className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 font-mono focus:outline-none focus:border-[#4CB8D6]"
                                    />
                                </div>

                                <div className="space-y-1">
                                    <label className="text-xs text-zinc-300 block">Allowed Origins</label>
                                    <input
                                        type="text"
                                        value={editKeyOrigins}
                                        onChange={(e) => setEditKeyOrigins(e.target.value)}
                                        disabled={!canCreateKeys}
                                        placeholder="* or https://app.domain.com"
                                        className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 font-mono focus:outline-none focus:border-[#4CB8D6]"
                                    />
                                </div>
                            </div>
                        </div>

                        <div className="flex items-center justify-between pt-4 border-t border-[#242932]">
                            {canCreateKeys && (
                                <div className="flex items-center gap-2">
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleRotateKey(selectedKeyForDetails.keyId || selectedKeyForDetails.id)}
                                        className="h-8 text-xs gap-1 text-[#D99A3D] hover:bg-[#D99A3D]/10 cursor-pointer"
                                    >
                                        <RefreshCw size={12} />
                                        Rotate Secret
                                    </Button>
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleDeleteKey(selectedKeyForDetails.keyId || selectedKeyForDetails.id)}
                                        className="h-8 text-xs gap-1 text-[#E45865] hover:bg-[#E45865]/10 cursor-pointer"
                                    >
                                        <Trash2 size={12} />
                                        Delete
                                    </Button>
                                </div>
                            )}

                            <div className="flex items-center gap-2 ml-auto">
                                <Button
                                    type="button"
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setSelectedKeyForDetails(null)}
                                    className="h-8 text-xs cursor-pointer"
                                >
                                    Cancel
                                </Button>
                                {canCreateKeys && (
                                    <Button
                                        type="submit"
                                        size="sm"
                                        isLoading={updateKeyMutation.isPending}
                                        className="h-8 text-xs bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer"
                                    >
                                        Save Changes
                                    </Button>
                                )}
                            </div>
                        </div>
                    </form>
                </Modal>
            )}
        </div>
    );
}
