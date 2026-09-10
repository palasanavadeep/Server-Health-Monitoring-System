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
    EyeOff,
    Activity,
    BarChart3,
    Globe,
    Shield,
    ShieldAlert,
    Clock,
    Terminal,
    Calendar,
    Sparkles
} from 'lucide-react';
import { cn } from '@/lib/utils';

type EnvironmentType = 'production' | 'staging' | 'development' | 'testing';

interface EnvironmentOption {
    id: EnvironmentType;
    label: string;
}

const ENVIRONMENT_OPTIONS: EnvironmentOption[] = [
    { id: 'production', label: 'Production' },
    { id: 'staging', label: 'Staging' },
    { id: 'development', label: 'Development' },
    { id: 'testing', label: 'Testing' },
];

const EXPIRY_PRESETS = [
    { label: '30 Days', minutes: 43200 },
    { label: '90 Days', minutes: 129600 },
    { label: '1 Year', minutes: 525600 },
    { label: 'No Expiry', minutes: 5256000 }
];

const ROTATION_PRESETS = [
    { label: '7 Days', days: 7 },
    { label: '14 Days', days: 14 },
    { label: '30 Days', days: 30 }
];

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
    const [copiedMasked, setCopiedMasked] = useState(false);

    // Selected key for detail / edit modal
    const [selectedKeyForDetails, setSelectedKeyForDetails] = useState<ApiKey | null>(null);

    // Search query
    const [searchQuery, setSearchQuery] = useState('');

    // Creation Form inputs
    const [newKeyName, setNewKeyName] = useState('');
    const [newKeyDesc, setNewKeyDesc] = useState('');
    const [newKeyEnv, setNewKeyEnv] = useState<EnvironmentType>('production');
    const [newKeyExpires, setNewKeyExpires] = useState<number>(43200);
    const [newKeyCanIngest, setNewKeyCanIngest] = useState(true);
    const [newKeyCanRead, setNewKeyCanRead] = useState(false);
    const [newKeyIPs, setNewKeyIPs] = useState('0.0.0.0/0');
    const [newKeyOrigins, setNewKeyOrigins] = useState('*');
    const [newKeyWarnDays, setNewKeyWarnDays] = useState<number>(30);

    // Edit Form inputs
    const [editKeyName, setEditKeyName] = useState('');
    const [editKeyDesc, setEditKeyDesc] = useState('');
    const [editKeyEnv, setEditKeyEnv] = useState<EnvironmentType>('production');
    const [editKeyCanIngest, setEditKeyCanIngest] = useState(true);
    const [editKeyCanRead, setEditKeyCanRead] = useState(false);
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

        try {
            const res = await createKeyMutation.mutateAsync({
                name: newKeyName.trim(),
                description: newKeyDesc.trim() || undefined,
                environment: newKeyEnv,
                expiresAt: Number(newKeyExpires) || 43200,
                permissions: {
                    canIngest: newKeyCanIngest,
                    canReadAnalytics: newKeyCanRead,
                    allowedServices: []
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
            setNewKeyExpires(43200);
            setNewKeyCanIngest(true);
            setNewKeyCanRead(false);
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

    const handleCopyMasked = (keyText: string) => {
        if (!keyText) return;
        navigator.clipboard.writeText(keyText);
        setCopiedMasked(true);
        toast('Token identifier copied', 'success');
        setTimeout(() => setCopiedMasked(false), 2000);
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
        setEditKeyEnv((key.environment || 'production') as EnvironmentType);
        setEditKeyCanIngest(key.permissions?.canIngest !== false);
        setEditKeyCanRead(key.permissions?.canReadAnalytics === true);
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

        try {
            await updateKeyMutation.mutateAsync({
                keyId,
                name: editKeyName.trim(),
                description: editKeyDesc.trim() || undefined,
                environment: editKeyEnv,
                permissions: {
                    canIngest: editKeyCanIngest,
                    canReadAnalytics: editKeyCanRead,
                    allowedServices: []
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
        if (!confirm("Are you sure you want to delete this API Key permanently? This action cannot be undone.")) {
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
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100 flex items-center gap-2">
                        <KeyRound className="w-5 h-5 text-[#4CB8D6]" />
                        API Keys
                    </h1>
                    <div className="flex items-center gap-2 mt-0.5 text-xs text-zinc-400">
                        <span>Manage telemetry ingestion credentials, access scopes, and firewall filters.</span>
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
                        placeholder="Search API keys by name, environment, or token..."
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
                                    <th className="py-2.5 px-4 font-semibold">Permissions</th>
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
                                            <span className="group-hover:text-[#4CB8D6] transition-colors flex items-center gap-1.5">
                                                {key.name}
                                            </span>
                                            {key.description && (
                                                <span className="block text-[11px] text-zinc-500 font-normal truncate max-w-xs">
                                                    {key.description}
                                                </span>
                                            )}
                                        </td>
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded border border-[#242932] bg-[#0E1014] text-zinc-300 inline-flex items-center">
                                                {key.environment || 'production'}
                                            </span>
                                        </td>
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <span className="font-mono text-zinc-300 text-[11px] bg-[#0E1014] px-2 py-1 rounded border border-[#242932]">
                                                {formatShortApiKey(key.keyValue, key.prefix)}
                                            </span>
                                        </td>
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <div className="flex items-center gap-1">
                                                {key.permissions?.canIngest !== false && (
                                                    <span className="text-[9px] font-mono uppercase bg-[#4CB8D6]/10 text-[#4CB8D6] px-1.5 py-0.5 rounded border border-[#4CB8D6]/30">
                                                        INGEST
                                                    </span>
                                                )}
                                                {key.permissions?.canReadAnalytics && (
                                                    <span className="text-[9px] font-mono uppercase bg-[#48B982]/10 text-[#48B982] px-1.5 py-0.5 rounded border border-[#48B982]/30">
                                                        READ
                                                    </span>
                                                )}
                                                {!key.permissions?.canIngest && !key.permissions?.canReadAnalytics && (
                                                    <span className="text-[9px] text-zinc-500 font-mono">None</span>
                                                )}
                                            </div>
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

            {/* Redesigned Clean & Rich Create API Key Modal */}
            <Modal
                isOpen={isCreateModalOpen}
                onClose={() => setIsCreateModalOpen(false)}
                title="Create API Key"
                description="Generate a high-throughput telemetry ingestion token with scoped permissions and network filters."
                maxWidth="max-w-2xl"
            >
                <form onSubmit={handleCreateKey} className="space-y-4">
                    {/* Section 1: Identity & Environment */}
                    <div className="space-y-2.5">
                        <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                            <Sliders size={13} className="text-[#4CB8D6]" />
                            <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                01. Identity & Environment
                            </span>
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs font-medium text-zinc-200 block">
                                Key Name <span className="text-[#E45865]">*</span>
                            </label>
                            <input
                                type="text"
                                value={newKeyName}
                                onChange={(e) => setNewKeyName(e.target.value)}
                                placeholder="e.g. gateway-production-ingress"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                            />
                            <p className="text-[10px] text-zinc-500">
                                A recognizable name to identify this token in telemetry streams and audit events.
                            </p>
                        </div>

                        {/* Segmented Environment Picker */}
                        <div className="space-y-1.5">
                            <label className="text-xs font-medium text-zinc-200 block">
                                Deployment Environment
                            </label>
                            <div className="grid grid-cols-2 sm:grid-cols-4 gap-1 p-1 bg-[#090B0E] border border-[#242932] rounded-lg">
                                {ENVIRONMENT_OPTIONS.map((env) => {
                                    const isSelected = newKeyEnv === env.id;
                                    return (
                                        <button
                                            key={env.id}
                                            type="button"
                                            onClick={() => setNewKeyEnv(env.id)}
                                            className={cn(
                                                "py-1.5 px-3 rounded-md text-xs font-medium transition-all text-center cursor-pointer select-none",
                                                isSelected 
                                                    ? "bg-[#181D24] text-zinc-100 font-semibold border border-[#323946] shadow-xs" 
                                                    : "text-zinc-400 hover:text-zinc-200 hover:bg-[#12151B] border border-transparent"
                                            )}
                                        >
                                            {env.label}
                                        </button>
                                    );
                                })}
                            </div>
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs font-medium text-zinc-200 block">
                                Description <span className="text-zinc-500 text-[10px] font-normal">(Optional)</span>
                            </label>
                            <input
                                type="text"
                                value={newKeyDesc}
                                onChange={(e) => setNewKeyDesc(e.target.value)}
                                placeholder="e.g. Microservice metrics token for European Kubernetes node pool"
                                className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-600 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                            />
                        </div>
                    </div>

                    {/* Section 2: Permissions & Scopes */}
                    <div className="space-y-2.5">
                        <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                            <Shield size={13} className="text-[#4CB8D6]" />
                            <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                02. Permissions & Scopes
                            </span>
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                            {/* Ingest Telemetry Option */}
                            <div
                                role="checkbox"
                                aria-checked={newKeyCanIngest}
                                tabIndex={0}
                                onKeyDown={(e) => {
                                    if (e.key === ' ' || e.key === 'Enter') {
                                        e.preventDefault();
                                        setNewKeyCanIngest(!newKeyCanIngest);
                                    }
                                }}
                                onClick={() => setNewKeyCanIngest(!newKeyCanIngest)}
                                className={cn(
                                    "group flex items-center justify-between p-3 rounded-lg border transition-all cursor-pointer select-none",
                                    newKeyCanIngest 
                                        ? "bg-[#141920] border-[#4CB8D6]/50 shadow-xs" 
                                        : "bg-[#0E1014] border-[#242932] hover:border-zinc-700 hover:bg-[#12151B]"
                                )}
                            >
                                <div className="flex items-center gap-2.5">
                                    <div className={cn(
                                        "w-4.5 h-4.5 rounded-[4px] border flex items-center justify-center transition-all shrink-0",
                                        newKeyCanIngest
                                            ? "bg-[#4CB8D6] border-[#4CB8D6] text-zinc-950 shadow-xs shadow-[#4CB8D6]/20"
                                            : "bg-[#0B0D10] border-[#2E3642] group-hover:border-zinc-500"
                                    )}>
                                        {newKeyCanIngest && <Check size={12} strokeWidth={3} className="text-zinc-950" />}
                                    </div>
                                    <span className={cn(
                                        "text-xs font-medium transition-colors",
                                        newKeyCanIngest ? "text-zinc-100 font-semibold" : "text-zinc-300 group-hover:text-zinc-200"
                                    )}>
                                        Ingest Telemetry Data
                                    </span>
                                </div>
                                <span className={cn(
                                    "text-[9px] font-mono uppercase px-1.5 py-0.5 rounded border transition-colors",
                                    newKeyCanIngest
                                        ? "bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/30 font-semibold"
                                        : "bg-[#161A22] text-zinc-500 border-[#242932]"
                                )}>
                                    INGEST
                                </span>
                            </div>

                            {/* Read Analytics Option */}
                            <div
                                role="checkbox"
                                aria-checked={newKeyCanRead}
                                tabIndex={0}
                                onKeyDown={(e) => {
                                    if (e.key === ' ' || e.key === 'Enter') {
                                        e.preventDefault();
                                        setNewKeyCanRead(!newKeyCanRead);
                                    }
                                }}
                                onClick={() => setNewKeyCanRead(!newKeyCanRead)}
                                className={cn(
                                    "group flex items-center justify-between p-3 rounded-lg border transition-all cursor-pointer select-none",
                                    newKeyCanRead 
                                        ? "bg-[#141920] border-[#4CB8D6]/50 shadow-xs" 
                                        : "bg-[#0E1014] border-[#242932] hover:border-zinc-700 hover:bg-[#12151B]"
                                )}
                            >
                                <div className="flex items-center gap-2.5">
                                    <div className={cn(
                                        "w-4.5 h-4.5 rounded-[4px] border flex items-center justify-center transition-all shrink-0",
                                        newKeyCanRead
                                            ? "bg-[#4CB8D6] border-[#4CB8D6] text-zinc-950 shadow-xs shadow-[#4CB8D6]/20"
                                            : "bg-[#0B0D10] border-[#2E3642] group-hover:border-zinc-500"
                                    )}>
                                        {newKeyCanRead && <Check size={12} strokeWidth={3} className="text-zinc-950" />}
                                    </div>
                                    <span className={cn(
                                        "text-xs font-medium transition-colors",
                                        newKeyCanRead ? "text-zinc-100 font-semibold" : "text-zinc-300 group-hover:text-zinc-200"
                                    )}>
                                        Read Analytics & Metrics
                                    </span>
                                </div>
                                <span className={cn(
                                    "text-[9px] font-mono uppercase px-1.5 py-0.5 rounded border transition-colors",
                                    newKeyCanRead
                                        ? "bg-[#48B982]/15 text-[#48B982] border-[#48B982]/30 font-semibold"
                                        : "bg-[#161A22] text-zinc-500 border-[#242932]"
                                )}>
                                    READ
                                </span>
                            </div>
                        </div>
                    </div>

                    {/* Section 3: Network Security & Restrictions */}
                    <div className="space-y-2.5">
                        <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                            <Globe size={13} className="text-[#4CB8D6]" />
                            <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                03. Network Security & Filters
                            </span>
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                            {/* Allowed IPs */}
                            <div className="space-y-1">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Allowed IP Ranges (CIDR)
                                </label>
                                <input
                                    type="text"
                                    value={newKeyIPs}
                                    onChange={(e) => setNewKeyIPs(e.target.value)}
                                    placeholder="0.0.0.0/0"
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                                />
                                <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
                                    <span className="text-[10px] text-zinc-500">Presets:</span>
                                    <button
                                        type="button"
                                        onClick={() => setNewKeyIPs('0.0.0.0/0')}
                                        className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                    >
                                        0.0.0.0/0 (Anywhere)
                                    </button>
                                    <button
                                        type="button"
                                        onClick={() => setNewKeyIPs('127.0.0.1/32')}
                                        className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                    >
                                        127.0.0.1/32 (Local)
                                    </button>
                                </div>
                            </div>

                            {/* Allowed Origins */}
                            <div className="space-y-1">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Allowed HTTP Origins (CORS)
                                </label>
                                <input
                                    type="text"
                                    value={newKeyOrigins}
                                    onChange={(e) => setNewKeyOrigins(e.target.value)}
                                    placeholder="*"
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                                />
                                <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
                                    <span className="text-[10px] text-zinc-500">Presets:</span>
                                    <button
                                        type="button"
                                        onClick={() => setNewKeyOrigins('*')}
                                        className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                    >
                                        * (Wildcard)
                                    </button>
                                    <button
                                        type="button"
                                        onClick={() => setNewKeyOrigins('http://localhost:3000')}
                                        className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                    >
                                        localhost:3000
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Section 4: Key Lifespan & Rotation Alerts */}
                    <div className="space-y-2.5">
                        <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                            <Clock size={13} className="text-[#4CB8D6]" />
                            <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                04. Key Lifespan & Rotation Alerts
                            </span>
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                            <div className="space-y-1">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Expiration Lifespan
                                </label>
                                <div className="grid grid-cols-4 gap-1.5">
                                    {EXPIRY_PRESETS.map(p => (
                                        <button
                                            key={p.minutes}
                                            type="button"
                                            onClick={() => setNewKeyExpires(p.minutes)}
                                            className={cn(
                                                "py-1 px-1.5 rounded border text-[10px] font-medium text-center transition-colors cursor-pointer",
                                                newKeyExpires === p.minutes
                                                    ? "bg-[#4CB8D6]/15 border-[#4CB8D6]/50 text-[#4CB8D6]"
                                                    : "bg-[#0E1014] border-[#242932] text-zinc-400 hover:text-zinc-200"
                                            )}
                                        >
                                            {p.label}
                                        </button>
                                    ))}
                                </div>
                            </div>

                            <div className="space-y-1">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Rotation Warning Ahead
                                </label>
                                <div className="grid grid-cols-3 gap-1.5">
                                    {ROTATION_PRESETS.map(p => (
                                        <button
                                            key={p.days}
                                            type="button"
                                            onClick={() => setNewKeyWarnDays(p.days)}
                                            className={cn(
                                                "py-1 px-1.5 rounded border text-[10px] font-medium text-center transition-colors cursor-pointer",
                                                newKeyWarnDays === p.days
                                                    ? "bg-[#4CB8D6]/15 border-[#4CB8D6]/50 text-[#4CB8D6]"
                                                    : "bg-[#0E1014] border-[#242932] text-zinc-400 hover:text-zinc-200"
                                            )}
                                        >
                                            {p.label}
                                        </button>
                                    ))}
                                </div>
                            </div>
                        </div>
                    </div>

                    {/* Actions - Sticky at bottom of modal */}
                    <div className="sticky -bottom-4 sm:-bottom-5 bg-[#111419]/95 backdrop-blur-xs pt-3 pb-1 -mx-4 sm:-mx-5 px-4 sm:px-5 border-t border-[#242932] flex items-center justify-end gap-2.5 z-10 mt-3">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setIsCreateModalOpen(false)}
                            className="text-xs h-8 px-4 cursor-pointer"
                        >
                            Cancel
                        </Button>
                        <Button
                            type="submit"
                            size="sm"
                            isLoading={createKeyMutation.isPending}
                            className="text-xs h-8 px-5 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer gap-1.5"
                        >
                            <KeyRound size={13} />
                            Generate Key
                        </Button>
                    </div>
                </form>
            </Modal>

            {/* Redesigned Secret Token Generated Modal */}
            <Modal
                isOpen={isSecretGeneratedModalOpen}
                onClose={() => {
                    setIsSecretGeneratedModalOpen(false);
                    setGeneratedKey(null);
                }}
                title="API Key Created Successfully"
                description="Securely store your raw token now. For security purposes, it will never be displayed again."
                maxWidth="max-w-lg"
            >
                <div className="space-y-5 pt-1">
                    {/* Security Alert Banner */}
                    <div className="p-3 rounded-lg bg-[#D99A3D]/10 border border-[#D99A3D]/30 flex items-start gap-3">
                        <ShieldAlert className="w-5 h-5 text-[#D99A3D] shrink-0 mt-0.5" />
                        <div className="space-y-0.5">
                            <p className="text-xs font-semibold text-[#D99A3D]">Confidential Secret Token</p>
                            <p className="text-[11px] text-zinc-300 leading-relaxed">
                                Save this key in your secrets manager or environment configuration immediately. If lost, you must rotate the key to generate a new secret.
                            </p>
                        </div>
                    </div>

                    {/* High Contrast Token Box */}
                    <div className="space-y-1.5">
                        <label className="text-xs font-medium text-zinc-300 block">Raw API Key</label>
                        <div className="p-3 rounded-lg bg-[#0B0D10] border border-[#242932] flex items-center justify-between gap-3">
                            <code className="text-xs font-mono text-[#4CB8D6] select-all break-all flex-1 font-semibold">
                                {generatedKey}
                            </code>
                            <Button
                                size="sm"
                                variant="outline"
                                onClick={() => handleCopyKey(generatedKey || '')}
                                className="h-8 px-3 text-xs gap-1.5 cursor-pointer shrink-0 border-[#242932] hover:border-[#4CB8D6]"
                            >
                                {copied ? <Check size={13} className="text-[#48B982]" /> : <Copy size={13} />}
                                {copied ? 'Copied' : 'Copy'}
                            </Button>
                        </div>
                    </div>

                    {/* Quick Integration Usage Tip */}
                    <div className="p-3 rounded-lg bg-[#0E1014] border border-[#242932] space-y-1.5">
                        <div className="flex items-center gap-2 text-zinc-400 text-[11px]">
                            <Terminal size={12} className="text-[#4CB8D6]" />
                            <span className="font-semibold uppercase tracking-wider text-[10px]">Usage Example (HTTP Header)</span>
                        </div>
                        <pre className="text-[11px] font-mono text-zinc-300 overflow-x-auto p-2 rounded bg-[#0B0D10] border border-[#242932]">
                            {`curl -X POST https://api.monitor.domain/hit \\
  -H "x-api-key: ${generatedKey ? formatShortApiKey(generatedKey) : 'sm_key_...'}" \\
  -H "Content-Type: application/json"`}
                        </pre>
                    </div>

                    <div className="flex justify-end pt-2 border-t border-[#242932]">
                        <Button
                            size="sm"
                            onClick={() => {
                                setIsSecretGeneratedModalOpen(false);
                                setGeneratedKey(null);
                            }}
                            className="text-xs h-8 px-5 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                        >
                            Done
                        </Button>
                    </div>
                </div>
            </Modal>

            {/* Redesigned Clean & Rich Edit API Key Modal */}
            {selectedKeyForDetails && (
                <Modal
                    isOpen={!!selectedKeyForDetails}
                    onClose={() => setSelectedKeyForDetails(null)}
                    title={`Key Configuration: ${selectedKeyForDetails.name}`}
                    description="Update environment tags, access capabilities, or network whitelist filters."
                    maxWidth="max-w-2xl"
                >
                    <form onSubmit={handleSaveEdit} className="space-y-4">
                        {/* Token Metadata Banner */}
                        <div className="p-2.5 sm:p-3 rounded-lg bg-[#0E1014] border border-[#242932] flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
                            <div className="space-y-1">
                                <div className="flex items-center gap-2">
                                    <span className="text-[10px] font-semibold text-zinc-400 uppercase tracking-wider">Active Token ID</span>
                                    <StatusBadge 
                                        status={selectedKeyForDetails.isActive ? 'healthy' : 'offline'} 
                                        label={selectedKeyForDetails.isActive ? 'Active' : 'Disabled'} 
                                    />
                                </div>
                                <div className="flex items-center gap-2">
                                    <code className="text-xs font-mono text-zinc-300 font-semibold">
                                        {formatShortApiKey(selectedKeyForDetails.keyValue, selectedKeyForDetails.prefix)}
                                    </code>
                                    <button
                                        type="button"
                                        onClick={() => handleCopyMasked(selectedKeyForDetails.keyValue || selectedKeyForDetails.prefix || '')}
                                        title="Copy token prefix"
                                        className="text-zinc-500 hover:text-zinc-200 cursor-pointer p-0.5"
                                    >
                                        {copiedMasked ? <Check size={12} className="text-[#48B982]" /> : <Copy size={12} />}
                                    </button>
                                </div>
                            </div>

                            <div className="flex items-center gap-3 text-zinc-400 text-xs font-mono">
                                <div>
                                    <span className="text-[10px] text-zinc-500 block uppercase">Created</span>
                                    <span>{new Date(selectedKeyForDetails.createdAt).toLocaleDateString()}</span>
                                </div>
                            </div>
                        </div>

                        {/* Section 1: Identity & Environment */}
                        <div className="space-y-2.5">
                            <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                                <Sliders size={13} className="text-[#4CB8D6]" />
                                <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                    01. Identity & Environment
                                </span>
                            </div>

                            <div className="space-y-1">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Key Name <span className="text-[#E45865]">*</span>
                                </label>
                                <input
                                    type="text"
                                    value={editKeyName}
                                    onChange={(e) => setEditKeyName(e.target.value)}
                                    required
                                    disabled={!canCreateKeys}
                                    placeholder="e.g. gateway-production-ingress"
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6] transition-colors disabled:opacity-60"
                                />
                            </div>

                            {/* Segmented Environment Picker */}
                            <div className="space-y-1.5">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Deployment Environment
                                </label>
                                <div className="grid grid-cols-2 sm:grid-cols-4 gap-1 p-1 bg-[#090B0E] border border-[#242932] rounded-lg">
                                    {ENVIRONMENT_OPTIONS.map((env) => {
                                        const isSelected = editKeyEnv === env.id;
                                        return (
                                            <button
                                                key={env.id}
                                                type="button"
                                                disabled={!canCreateKeys}
                                                onClick={() => setEditKeyEnv(env.id)}
                                                className={cn(
                                                    "py-1.5 px-3 rounded-md text-xs font-medium transition-all text-center cursor-pointer select-none",
                                                    isSelected 
                                                        ? "bg-[#181D24] text-zinc-100 font-semibold border border-[#323946] shadow-xs" 
                                                        : "text-zinc-400 hover:text-zinc-200 hover:bg-[#12151B] border border-transparent",
                                                    !canCreateKeys && "pointer-events-none opacity-60"
                                                )}
                                            >
                                                {env.label}
                                            </button>
                                        );
                                    })}
                                </div>
                            </div>

                            <div className="space-y-1">
                                <label className="text-xs font-medium text-zinc-200 block">
                                    Description <span className="text-zinc-500 text-[10px] font-normal">(Optional)</span>
                                </label>
                                <input
                                    type="text"
                                    value={editKeyDesc}
                                    onChange={(e) => setEditKeyDesc(e.target.value)}
                                    disabled={!canCreateKeys}
                                    placeholder="Purpose of this key, service or deployment..."
                                    className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6] transition-colors disabled:opacity-60"
                                />
                            </div>
                        </div>

                        {/* Section 2: Permissions & Scopes */}
                        <div className="space-y-2.5">
                            <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                                <Shield size={13} className="text-[#4CB8D6]" />
                                <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                    02. Permissions & Scopes
                                </span>
                            </div>

                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                                {/* Ingest Telemetry Option */}
                                <div
                                    role="checkbox"
                                    aria-checked={editKeyCanIngest}
                                    tabIndex={canCreateKeys ? 0 : -1}
                                    onKeyDown={(e) => {
                                        if (!canCreateKeys) return;
                                        if (e.key === ' ' || e.key === 'Enter') {
                                            e.preventDefault();
                                            setEditKeyCanIngest(!editKeyCanIngest);
                                        }
                                    }}
                                    onClick={() => {
                                        if (!canCreateKeys) return;
                                        setEditKeyCanIngest(!editKeyCanIngest);
                                    }}
                                    className={cn(
                                        "group flex items-center justify-between p-3 rounded-lg border transition-all select-none",
                                        canCreateKeys ? "cursor-pointer" : "opacity-60 cursor-not-allowed",
                                        editKeyCanIngest 
                                            ? "bg-[#141920] border-[#4CB8D6]/50 shadow-xs" 
                                            : "bg-[#0E1014] border-[#242932] hover:border-zinc-700 hover:bg-[#12151B]"
                                    )}
                                >
                                    <div className="flex items-center gap-2.5">
                                        <div className={cn(
                                            "w-4.5 h-4.5 rounded-[4px] border flex items-center justify-center transition-all shrink-0",
                                            editKeyCanIngest
                                                ? "bg-[#4CB8D6] border-[#4CB8D6] text-zinc-950 shadow-xs shadow-[#4CB8D6]/20"
                                                : "bg-[#0B0D10] border-[#2E3642] group-hover:border-zinc-500"
                                        )}>
                                            {editKeyCanIngest && <Check size={12} strokeWidth={3} className="text-zinc-950" />}
                                        </div>
                                        <span className={cn(
                                            "text-xs font-medium transition-colors",
                                            editKeyCanIngest ? "text-zinc-100 font-semibold" : "text-zinc-300 group-hover:text-zinc-200"
                                        )}>
                                            Ingest Telemetry Data
                                        </span>
                                    </div>
                                    <span className={cn(
                                        "text-[9px] font-mono uppercase px-1.5 py-0.5 rounded border transition-colors",
                                        editKeyCanIngest
                                            ? "bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/30 font-semibold"
                                            : "bg-[#161A22] text-zinc-500 border-[#242932]"
                                    )}>
                                        INGEST
                                    </span>
                                </div>

                                {/* Read Analytics Option */}
                                <div
                                    role="checkbox"
                                    aria-checked={editKeyCanRead}
                                    tabIndex={canCreateKeys ? 0 : -1}
                                    onKeyDown={(e) => {
                                        if (!canCreateKeys) return;
                                        if (e.key === ' ' || e.key === 'Enter') {
                                            e.preventDefault();
                                            setEditKeyCanRead(!editKeyCanRead);
                                        }
                                    }}
                                    onClick={() => {
                                        if (!canCreateKeys) return;
                                        setEditKeyCanRead(!editKeyCanRead);
                                    }}
                                    className={cn(
                                        "group flex items-center justify-between p-3 rounded-lg border transition-all select-none",
                                        canCreateKeys ? "cursor-pointer" : "opacity-60 cursor-not-allowed",
                                        editKeyCanRead 
                                            ? "bg-[#141920] border-[#4CB8D6]/50 shadow-xs" 
                                            : "bg-[#0E1014] border-[#242932] hover:border-zinc-700 hover:bg-[#12151B]"
                                    )}
                                >
                                    <div className="flex items-center gap-2.5">
                                        <div className={cn(
                                            "w-4.5 h-4.5 rounded-[4px] border flex items-center justify-center transition-all shrink-0",
                                            editKeyCanRead
                                                ? "bg-[#4CB8D6] border-[#4CB8D6] text-zinc-950 shadow-xs shadow-[#4CB8D6]/20"
                                                : "bg-[#0B0D10] border-[#2E3642] group-hover:border-zinc-500"
                                        )}>
                                            {editKeyCanRead && <Check size={12} strokeWidth={3} className="text-zinc-950" />}
                                        </div>
                                        <span className={cn(
                                            "text-xs font-medium transition-colors",
                                            editKeyCanRead ? "text-zinc-100 font-semibold" : "text-zinc-300 group-hover:text-zinc-200"
                                        )}>
                                            Read Analytics & Metrics
                                        </span>
                                    </div>
                                    <span className={cn(
                                        "text-[9px] font-mono uppercase px-1.5 py-0.5 rounded border transition-colors",
                                        editKeyCanRead
                                            ? "bg-[#48B982]/15 text-[#48B982] border-[#48B982]/30 font-semibold"
                                            : "bg-[#161A22] text-zinc-500 border-[#242932]"
                                    )}>
                                        READ
                                    </span>
                                </div>
                            </div>
                        </div>

                        {/* Section 3: Network Security & Restrictions */}
                        <div className="space-y-2.5">
                            <div className="flex items-center gap-2 pb-1 border-b border-[#242932]">
                                <Globe size={13} className="text-[#4CB8D6]" />
                                <span className="text-[10px] font-semibold text-zinc-300 uppercase tracking-wider">
                                    03. Network Security & Filters
                                </span>
                            </div>

                            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                                {/* Allowed IPs */}
                                <div className="space-y-1">
                                    <label className="text-xs font-medium text-zinc-200 block">
                                        Allowed IP Ranges (CIDR)
                                    </label>
                                    <input
                                        type="text"
                                        value={editKeyIPs}
                                        onChange={(e) => setEditKeyIPs(e.target.value)}
                                        disabled={!canCreateKeys}
                                        placeholder="0.0.0.0/0"
                                        className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6] transition-colors disabled:opacity-60"
                                    />
                                    {canCreateKeys && (
                                        <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
                                            <span className="text-[10px] text-zinc-500">Presets:</span>
                                            <button
                                                type="button"
                                                onClick={() => setEditKeyIPs('0.0.0.0/0')}
                                                className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                            >
                                                0.0.0.0/0 (Anywhere)
                                            </button>
                                            <button
                                                type="button"
                                                onClick={() => setEditKeyIPs('127.0.0.1/32')}
                                                className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                            >
                                                127.0.0.1/32 (Local)
                                            </button>
                                        </div>
                                    )}
                                </div>

                                {/* Allowed Origins */}
                                <div className="space-y-1">
                                    <label className="text-xs font-medium text-zinc-200 block">
                                        Allowed HTTP Origins (CORS)
                                    </label>
                                    <input
                                        type="text"
                                        value={editKeyOrigins}
                                        onChange={(e) => setEditKeyOrigins(e.target.value)}
                                        disabled={!canCreateKeys}
                                        placeholder="*"
                                        className="w-full bg-[#0E1014] border border-[#242932] rounded-md px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6] transition-colors disabled:opacity-60"
                                    />
                                    {canCreateKeys && (
                                        <div className="flex items-center gap-1.5 flex-wrap pt-0.5">
                                            <span className="text-[10px] text-zinc-500">Presets:</span>
                                            <button
                                                type="button"
                                                onClick={() => setEditKeyOrigins('*')}
                                                className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                            >
                                                * (Wildcard)
                                            </button>
                                            <button
                                                type="button"
                                                onClick={() => setEditKeyOrigins('http://localhost:3000')}
                                                className="text-[10px] font-mono bg-[#181D24] text-zinc-300 hover:text-[#4CB8D6] hover:border-[#4CB8D6]/40 px-1.5 py-0.5 rounded border border-[#242932] cursor-pointer"
                                            >
                                                localhost:3000
                                            </button>
                                        </div>
                                    )}
                                </div>
                            </div>
                        </div>

                        {/* Actions - Sticky at bottom of modal */}
                        <div className="sticky -bottom-4 sm:-bottom-5 bg-[#111419]/95 backdrop-blur-xs pt-3 pb-1 -mx-4 sm:-mx-5 px-4 sm:px-5 border-t border-[#242932] flex items-center justify-between z-10 mt-3">
                            {canCreateKeys ? (
                                <div className="flex items-center gap-2">
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleRotateKey(selectedKeyForDetails.keyId || selectedKeyForDetails.id)}
                                        className="h-8 text-xs gap-1.5 border border-[#D99A3D]/40 bg-[#D99A3D]/10 text-[#D99A3D] hover:bg-[#D99A3D]/20 cursor-pointer"
                                    >
                                        <RefreshCw size={12} />
                                        Rotate Secret
                                    </Button>
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleDeleteKey(selectedKeyForDetails.keyId || selectedKeyForDetails.id)}
                                        className="h-8 text-xs gap-1.5 border border-[#E45865]/40 bg-[#E45865]/10 text-[#E45865] hover:bg-[#E45865]/20 cursor-pointer"
                                    >
                                        <Trash2 size={12} />
                                        Delete
                                    </Button>
                                </div>
                            ) : (
                                <div />
                            )}

                            <div className="flex items-center gap-2.5 ml-auto">
                                <Button
                                    type="button"
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setSelectedKeyForDetails(null)}
                                    className="h-8 text-xs px-4 cursor-pointer"
                                >
                                    Cancel
                                </Button>
                                {canCreateKeys && (
                                    <Button
                                        type="submit"
                                        size="sm"
                                        isLoading={updateKeyMutation.isPending}
                                        className="h-8 text-xs px-5 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer gap-1.5"
                                    >
                                        <Save size={13} />
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
