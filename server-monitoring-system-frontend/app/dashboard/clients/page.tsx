"use client";

import { useState, useMemo } from 'react';
import { notFound } from 'next/navigation';
import Link from 'next/link';
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
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { useToast } from '@/contexts/toast-context';
import { 
    KeyRound, 
    Plus, 
    Copy, 
    Check, 
    Loader2, 
    Calendar, 
    ShieldCheck, 
    ShieldAlert,
    Building,
    Users,
    Activity,
    ClipboardCopy,
    Trash2,
    RefreshCw,
    Sliders,
    Power,
    Eye,
    EyeOff,
    Info,
    Clock,
    X,
    Server,
    Shield,
    Save,
    Search,
    ArrowRight
} from 'lucide-react';
import { cn } from '@/lib/utils';

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

    // API Key generation fields
    const [newKeyName, setNewKeyName] = useState('');
    const [newKeyDesc, setNewKeyDesc] = useState('');
    const [newKeyEnv, setNewKeyEnv] = useState<'production' | 'staging' | 'development' | 'testing'>('production');
    const [newKeyExpires, setNewKeyExpires] = useState<number>(1440); // 24 hours in minutes
    const [newKeyCanIngest, setNewKeyCanIngest] = useState(true);
    const [newKeyCanRead, setNewKeyCanRead] = useState(false);
    const [newKeyServices, setNewKeyServices] = useState('');
    const [newKeyIPs, setNewKeyIPs] = useState('0.0.0.0/0');
    const [newKeyOrigins, setNewKeyOrigins] = useState('*');
    const [newKeyWarnDays, setNewKeyWarnDays] = useState<number>(30);

    const [isKeyModalOpen, setIsKeyModalOpen] = useState(false);
    const [generatedKey, setGeneratedKey] = useState<string | null>(null);
    const [copied, setCopied] = useState(false);

    // Search query state
    const [keySearchQuery, setKeySearchQuery] = useState('');

    // Unified interactive key configuration modal state
    const [selectedKeyForDetails, setSelectedKeyForDetails] = useState<ApiKey | null>(null);
    const [showModalKeyValue, setShowModalKeyValue] = useState(false);

    // Form inputs for editing currently selected key inside popup
    const [editKeyName, setEditKeyName] = useState('');
    const [editKeyDesc, setEditKeyDesc] = useState('');
    const [editKeyEnv, setEditKeyEnv] = useState<'production' | 'staging' | 'development' | 'testing'>('production');
    const [editKeyCanIngest, setEditKeyCanIngest] = useState(true);
    const [editKeyCanRead, setEditKeyCanRead] = useState(false);
    const [editKeyServices, setEditKeyServices] = useState('');
    const [editKeyIPs, setEditKeyIPs] = useState('');
    const [editKeyOrigins, setEditKeyOrigins] = useState('');
    const [editKeyWarnDays, setEditKeyWarnDays] = useState<number>(30);

    // Queries and mutations
    const { data: apiKeys = [], isLoading: loadingKeys } = useClientApiKeysQuery(selectedClientId);
    
    // Filtered keys array
    const filteredApiKeys = useMemo(() => {
        return apiKeys.filter(key => 
            key.name.toLowerCase().includes(keySearchQuery.toLowerCase()) ||
            (key.description && key.description.toLowerCase().includes(keySearchQuery.toLowerCase())) ||
            (key.environment && key.environment.toLowerCase().includes(keySearchQuery.toLowerCase())) ||
            (key.prefix && key.prefix.toLowerCase().includes(keySearchQuery.toLowerCase()))
        );
    }, [apiKeys, keySearchQuery]);

    // Lifecycle Mutations
    const createKeyMutation = useCreateApiKeyMutation(selectedClientId);
    const updateKeyMutation = useUpdateApiKeyMutation(selectedClientId);
    const deleteKeyMutation = useDeleteApiKeyMutation(selectedClientId);
    const deactivateKeyMutation = useDeactivateApiKeyMutation(selectedClientId);
    const activateKeyMutation = useActivateApiKeyMutation(selectedClientId);
    const rotateKeyMutation = useRotateApiKeyMutation(selectedClientId);

    // Handle Ingestion Key Generation
    const handleGenerateKey = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newKeyName.trim() || !selectedClientId) return;

        const ipsArray = newKeyIPs.split(',').map(s => s.trim()).filter(Boolean);
        const originsArray = newKeyOrigins.split(',').map(s => s.trim()).filter(Boolean);
        const servicesArray = newKeyServices.split(',').map(s => s.trim()).filter(Boolean);

        try {
            const res = await createKeyMutation.mutateAsync({
                name: newKeyName,
                description: newKeyDesc || undefined,
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
            setGeneratedKey(res.keyValue || res.key || 'Failed to retrieve raw token');
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
            setIsKeyModalOpen(true);
            toast('API Key generated!', 'success');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to generate Ingestion Key', 'error');
        }
    };

    // Copy Raw Key to Clipboard
    const handleCopyKey = (keyValueText: string) => {
        if (!keyValueText) return;
        navigator.clipboard.writeText(keyValueText);
        setCopied(true);
        toast('API Key copied to clipboard', 'success');
        setTimeout(() => setCopied(false), 2000);
    };

    // Toggle Status
    const handleToggleKeyStatus = async (keyId: string, isActive: boolean) => {
        try {
            if (isActive) {
                await deactivateKeyMutation.mutateAsync(keyId);
                toast('API Key deactivated successfully', 'info');
                if (selectedKeyForDetails?.id === keyId || selectedKeyForDetails?.keyId === keyId) {
                    setSelectedKeyForDetails(prev => prev ? { ...prev, isActive: false } : null);
                }
            } else {
                await activateKeyMutation.mutateAsync(keyId);
                toast('API Key activated successfully', 'success');
                if (selectedKeyForDetails?.id === keyId || selectedKeyForDetails?.keyId === keyId) {
                    setSelectedKeyForDetails(prev => prev ? { ...prev, isActive: true } : null);
                }
            }
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to change API key status', 'error');
        }
    };

    // Rotate Key
    const handleRotateKey = async (keyId: string) => {
        if (!confirm("Are you sure you want to rotate this Ingestion Key? The old key will immediately stop working!")) {
            return;
        }

        try {
            const res = await rotateKeyMutation.mutateAsync(keyId);
            setGeneratedKey(res.keyValue || res.key || 'Failed to retrieve rotated token');
            setIsKeyModalOpen(true);
            setSelectedKeyForDetails(null);
            toast('API Key rotated successfully!', 'success');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to rotate API key', 'error');
        }
    };

    // Save Edited Key
    const handleSaveEditKey = async (e: React.FormEvent) => {
        e.preventDefault();
        const keyId = selectedKeyForDetails?.id || selectedKeyForDetails?.keyId;
        if (!editKeyName.trim() || !keyId) return;

        const ipsArray = editKeyIPs.split(',').map(s => s.trim()).filter(Boolean);
        const originsArray = editKeyOrigins.split(',').map(s => s.trim()).filter(Boolean);
        const servicesArray = editKeyServices.split(',').map(s => s.trim()).filter(Boolean);

        try {
            await updateKeyMutation.mutateAsync({
                keyId,
                name: editKeyName,
                description: editKeyDesc || undefined,
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
            toast('API Key parameters updated successfully', 'success');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to update API key', 'error');
        }
    };

    // Delete Key
    const handleDeleteKey = async (keyId: string) => {
        if (!confirm("Are you sure you want to delete this API Key permanently?")) {
            return;
        }

        try {
            await deleteKeyMutation.mutateAsync(keyId);
            setSelectedKeyForDetails(null);
            toast('API Key deleted permanently', 'info');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to delete API key', 'error');
        }
    };

    // Open Unified Detail Modal
    const handleOpenDetails = (key: ApiKey) => {
        setSelectedKeyForDetails(key);
        setShowModalKeyValue(false);
        
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

    if (loading || (user && user.role === 'super_admin')) {
        return null;
    }

    return (
        <div className="space-y-8 max-w-7xl mx-auto pb-12 animate-in fade-in duration-300">
            {/* Page Header */}
            <div className="border-b border-border-color/30 pb-5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                        <KeyRound className="text-cyan-400 w-7 h-7" />
                        API Keys & Ingestion Tokens
                    </h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Manage ingestion tokens, configure CIDR & origin scopes, rotate credentials, and track token security.
                    </p>
                </div>
                {isClientAdmin && (
                    <Link href="/dashboard/operators">
                        <Button variant="secondary" size="sm" className="text-xs h-8 gap-1.5 cursor-pointer">
                            <Users size={13} />
                            Manage Operator Users
                            <ArrowRight size={12} />
                        </Button>
                    </Link>
                )}
            </div>

            {/* Top Workspace Info Banner */}
            <div className="glass-panel p-5 rounded-2xl border border-cyan-500/10 bg-glass-card/65 shadow-lg shadow-cyan-500/5">
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 text-xs font-semibold">
                    <div className="flex flex-wrap items-center gap-4 md:gap-8">
                        <div className="flex items-center gap-2">
                            <Building className="text-cyan-400 w-5 h-5 shrink-0" />
                            <div>
                                <span className="text-[9px] uppercase tracking-wider text-muted-foreground block leading-none">Tenant Workspace ID</span>
                                <span className="font-mono text-foreground font-semibold text-xs mt-0.5 block">{user?.clientId || 'N/A'}</span>
                            </div>
                        </div>
                        <div className="h-6 w-[1px] bg-border-color/30 hidden sm:block" />
                        <div>
                            <span className="text-[9px] uppercase tracking-wider text-muted-foreground block leading-none">Active Operator</span>
                            <span className="text-foreground text-xs mt-0.5 block">{user?.username}</span>
                        </div>
                        <div className="h-6 w-[1px] bg-border-color/30 hidden sm:block" />
                        <div>
                            <span className="text-[9px] uppercase tracking-wider text-muted-foreground block leading-none">Operator Role</span>
                            <Badge variant="default" className="mt-0.5 font-mono uppercase text-[9px] h-4.5 px-1.5">
                                {user?.role?.replace('_', ' ')}
                            </Badge>
                        </div>
                    </div>
                    <Badge variant="info" className="w-fit text-[10px] font-mono shrink-0">
                        Tenant Scoped
                    </Badge>
                </div>
            </div>

            {/* Main Content Area */}
            <div className="space-y-6">
                {/* Provision New Ingestion Key */}
                {canCreateKeys && (
                    <Card className="border-cyan-500/10 bg-glass-card/20">
                        <CardHeader className="pb-3">
                            <CardTitle className="text-sm flex items-center gap-2">
                                <Plus size={14} className="text-cyan-400" />
                                Provision New Ingestion Key
                            </CardTitle>
                            <CardDescription className="text-xs">Configure Ingest Credentials, Expirations, Permissions, and Network Scopes</CardDescription>
                        </CardHeader>
                        <CardContent>
                            <form onSubmit={handleGenerateKey} className="space-y-4">
                                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Name *</label>
                                        <Input
                                            value={newKeyName}
                                            onChange={(e) => setNewKeyName(e.target.value)}
                                            placeholder="e.g. production-gateway"
                                            required
                                        />
                                    </div>

                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Target Environment</label>
                                        <select
                                            value={newKeyEnv}
                                            onChange={(e: any) => setNewKeyEnv(e.target.value)}
                                            className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground h-9"
                                        >
                                            <option value="production">Production</option>
                                            <option value="staging">Staging</option>
                                            <option value="development">Development</option>
                                            <option value="testing">Testing</option>
                                        </select>
                                    </div>

                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Lifetime Expiry</label>
                                        <select
                                            value={newKeyExpires}
                                            onChange={(e) => setNewKeyExpires(Number(e.target.value))}
                                            className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground h-9"
                                        >
                                            <option value={24}>24 Minutes (Temporary)</option>
                                            <option value={60}>1 Hour</option>
                                            <option value={1440}>1 Day (24 hours)</option>
                                            <option value={43200}>30 Days</option>
                                            <option value={525600}>365 Days (1 Year)</option>
                                        </select>
                                    </div>
                                </div>

                                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Description</label>
                                        <Input
                                            value={newKeyDesc}
                                            onChange={(e) => setNewKeyDesc(e.target.value)}
                                            placeholder="Describe key purpose..."
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Restrict to Service Names (Optional)</label>
                                        <Input
                                            value={newKeyServices}
                                            onChange={(e) => setNewKeyServices(e.target.value)}
                                            placeholder="e.g. rust-ingest, backend-auth (empty for all)"
                                        />
                                    </div>
                                </div>

                                <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed IP Filters</label>
                                        <Input
                                            value={newKeyIPs}
                                            onChange={(e) => setNewKeyIPs(e.target.value)}
                                            placeholder="e.g. 192.168.1.0/24, 0.0.0.0/0"
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed Domains / Origins</label>
                                        <Input
                                            value={newKeyOrigins}
                                            onChange={(e) => setNewKeyOrigins(e.target.value)}
                                            placeholder="e.g. https://domain.com, *"
                                        />
                                    </div>
                                    <div className="space-y-1.5">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Rotation Alert Threshold (Days)</label>
                                        <Input
                                            type="number"
                                            value={newKeyWarnDays}
                                            onChange={(e) => setNewKeyWarnDays(Number(e.target.value))}
                                            min={1}
                                            max={365}
                                        />
                                    </div>
                                </div>

                                <div className="flex flex-wrap items-center gap-6 p-3 rounded-lg border border-border-color bg-glass-card/25">
                                    <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Permissions Scope:</span>
                                    <label className="flex items-center gap-2 text-xs font-semibold text-foreground cursor-pointer select-none">
                                        <input 
                                            type="checkbox" 
                                            checked={newKeyCanIngest} 
                                            onChange={(e) => setNewKeyCanIngest(e.target.checked)}
                                            className="rounded bg-input-bg border-border-color text-cyan-500 focus:ring-cyan-500"
                                        />
                                        Ingest Analytics Data
                                    </label>
                                    <label className="flex items-center gap-2 text-xs font-semibold text-foreground cursor-pointer select-none">
                                        <input 
                                            type="checkbox" 
                                            checked={newKeyCanRead} 
                                            onChange={(e) => setNewKeyCanRead(e.target.checked)}
                                            className="rounded bg-input-bg border-border-color text-cyan-500 focus:ring-cyan-500"
                                        />
                                        Read Telemetry Aggregations
                                    </label>
                                </div>

                                <div className="flex justify-end">
                                    <Button type="submit" className="text-xs h-9 gap-1.5 cursor-pointer" isLoading={createKeyMutation.isPending}>
                                        <Plus size={14} />
                                        Generate Ingestion Token
                                    </Button>
                                </div>
                            </form>
                        </CardContent>
                    </Card>
                )}

                {/* Ingestion Credentials Table list with Search */}
                <Card>
                    <CardHeader className="pb-3 border-b border-border-color/20">
                        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                            <div>
                                <CardTitle className="text-base flex items-center gap-2">
                                    <KeyRound className="text-cyan-400 w-4 h-4" />
                                    Access Tokens Workspace
                                </CardTitle>
                                <CardDescription className="text-xs">Click on any key to inspect details, edit network scopes, rotate or deactivate.</CardDescription>
                            </div>
                            
                            <div className="relative w-full sm:max-w-xs shrink-0">
                                <Input
                                    value={keySearchQuery}
                                    onChange={(e) => setKeySearchQuery(e.target.value)}
                                    placeholder="Search workspace keys..."
                                    className="h-9 text-xs pl-8 pr-3"
                                />
                                <span className="absolute left-2.5 top-3 text-muted-foreground/60">
                                    <Search size={13} />
                                </span>
                            </div>
                        </div>
                    </CardHeader>
                    <CardContent className="p-0">
                        {loadingKeys ? (
                            <div className="flex items-center justify-center py-12">
                                <Loader2 className="animate-spin text-cyan-400 w-6 h-6" />
                            </div>
                        ) : filteredApiKeys.length === 0 ? (
                            <div className="text-center py-12 flex flex-col items-center justify-center p-6">
                                <ShieldAlert className="w-10 h-10 text-muted-foreground/30 mb-2.5" />
                                <p className="text-sm font-semibold text-foreground">No Access Tokens Found</p>
                                <p className="text-xs text-muted-foreground max-w-[280px] mt-1">
                                    {keySearchQuery ? "No matches found for your search." : "There are no active ingestion credentials mapped to this client gateway."}
                                </p>
                            </div>
                        ) : (
                            <Table>
                                <TableHeader>
                                    <TableRow className="hover:bg-transparent">
                                        <TableHead className="w-2/5">Ingestion Key Name</TableHead>
                                        <TableHead className="w-1/5">Environment</TableHead>
                                        <TableHead className="w-1/5">Created On</TableHead>
                                        <TableHead className="w-1/5">Status</TableHead>
                                        <TableHead className="text-right w-24">Quick Toggle</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {filteredApiKeys.map((key, index) => (
                                        <TableRow 
                                            key={key.id || key.keyId || `apikey-${index}`}
                                            onClick={() => handleOpenDetails(key)}
                                            className="cursor-pointer hover:bg-glass-card-hover/40 transition-colors select-none"
                                        >
                                            <TableCell className="font-semibold text-sm w-2/5">
                                                <div className="flex flex-col gap-0.5 pr-2">
                                                    <span className="text-foreground hover:text-cyan-400 transition-colors flex items-center gap-1.5">
                                                        {key.name}
                                                        <Info size={11} className="text-muted-foreground/60" />
                                                    </span>
                                                    {key.prefix && (
                                                        <code className="text-[10px] font-mono text-cyan-400 bg-cyan-500/5 px-1.5 py-0.5 rounded w-fit border border-cyan-500/10">
                                                            Prefix: {key.prefix}***
                                                        </code>
                                                    )}
                                                </div>
                                            </TableCell>
                                            
                                            <TableCell className="w-1/5">
                                                <Badge variant="outline" className="font-mono text-[9px] uppercase tracking-wider border-border-color bg-zinc-950/20">
                                                    {key.environment || 'production'}
                                                </Badge>
                                            </TableCell>

                                            <TableCell className="text-xs text-muted-foreground w-1/5">
                                                {new Date(key.createdAt).toLocaleDateString()}
                                            </TableCell>
                                            
                                            <TableCell className="w-1/5">
                                                <Badge variant={key.isActive ? "success" : "destructive"}>
                                                    {key.isActive ? "Active" : "Disabled"}
                                                </Badge>
                                            </TableCell>
                                            
                                            <TableCell className="text-right w-24 align-middle">
                                                {canCreateKeys ? (
                                                    <div className="flex items-center justify-end">
                                                        <button
                                                            onClick={(e) => {
                                                                e.stopPropagation();
                                                                handleToggleKeyStatus(key.id || key.keyId, key.isActive);
                                                            }}
                                                            title={key.isActive ? "Disable Token" : "Enable Token"}
                                                            className={cn(
                                                                "p-1.5 rounded-lg border transition-colors cursor-pointer inline-flex items-center justify-center",
                                                                key.isActive 
                                                                    ? "text-orange-400 border-orange-500/20 hover:bg-orange-500/10" 
                                                                    : "text-emerald-400 border-emerald-500/20 hover:bg-emerald-500/10"
                                                            )}
                                                        >
                                                            <Power size={13} />
                                                        </button>
                                                    </div>
                                                ) : (
                                                    <span className="text-[10px] text-muted-foreground">None</span>
                                                )}
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        )}
                    </CardContent>
                </Card>
            </div>

            {/* Generated Ingestion Token Display Modal */}
            {isKeyModalOpen && generatedKey && (
                <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md p-4 animate-in fade-in duration-200">
                    <Card className="w-full max-w-lg border-cyan-500/30 bg-zinc-950 shadow-2xl relative">
                        <CardHeader className="pb-3 border-b border-border-color/30">
                            <CardTitle className="text-base flex items-center gap-2 text-foreground">
                                <KeyRound className="text-cyan-400 w-5 h-5" />
                                Save Your Ingestion Secret Key
                            </CardTitle>
                            <CardDescription className="text-xs text-rose-400 font-medium">
                                Warning: This full secret key will never be shown again! Copy and store it in a secure environment.
                            </CardDescription>
                        </CardHeader>
                        <CardContent className="space-y-4 pt-4">
                            <div className="space-y-2">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Raw Secret Token
                                </label>
                                <div className="flex items-center gap-2 p-3 bg-zinc-900 border border-border-color rounded-lg">
                                    <code className="text-xs font-mono text-cyan-300 break-all select-all flex-1">
                                        {generatedKey}
                                    </code>
                                    <Button
                                        size="sm"
                                        variant="outline"
                                        onClick={() => handleCopyKey(generatedKey)}
                                        className="shrink-0 h-8 gap-1.5 text-xs cursor-pointer"
                                    >
                                        {copied ? <Check size={13} className="text-emerald-400" /> : <Copy size={13} />}
                                        {copied ? 'Copied' : 'Copy'}
                                    </Button>
                                </div>
                            </div>
                            <div className="flex justify-end pt-2">
                                <Button
                                    size="sm"
                                    onClick={() => {
                                        setIsKeyModalOpen(false);
                                        setGeneratedKey(null);
                                    }}
                                    className="text-xs h-8 px-4 cursor-pointer"
                                >
                                    I Have Stored This Key Safely
                                </Button>
                            </div>
                        </CardContent>
                    </Card>
                </div>
            )}

            {/* Key Configuration Details Modal */}
            {selectedKeyForDetails && (
                <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-md p-4 animate-in fade-in duration-200">
                    <Card className="w-full max-w-2xl border-cyan-500/30 bg-zinc-950 shadow-2xl relative max-h-[90vh] overflow-y-auto">
                        <CardHeader className="pb-3 border-b border-border-color/30 flex flex-row items-center justify-between">
                            <div>
                                <CardTitle className="text-base flex items-center gap-2 text-foreground">
                                    <KeyRound className="text-cyan-400 w-4 h-4" />
                                    Key Details: {selectedKeyForDetails.name}
                                </CardTitle>
                                <CardDescription className="text-xs">
                                    Configure permissions, rotate credentials, or update CIDR and domain filters.
                                </CardDescription>
                            </div>
                            <button
                                onClick={() => setSelectedKeyForDetails(null)}
                                className="p-1 rounded-lg text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                            >
                                <X size={18} />
                            </button>
                        </CardHeader>
                        <CardContent className="space-y-5 pt-4">
                            {/* Key metadata banner */}
                            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-3 bg-zinc-900/60 rounded-xl border border-border-color/40 text-xs">
                                <div>
                                    <span className="text-[9px] uppercase font-bold text-muted-foreground block">Key ID</span>
                                    <code className="font-mono text-[11px] text-cyan-300 truncate block mt-0.5">
                                        {(selectedKeyForDetails.id || selectedKeyForDetails.keyId).substring(0, 10)}...
                                    </code>
                                </div>
                                <div>
                                    <span className="text-[9px] uppercase font-bold text-muted-foreground block">Prefix</span>
                                    <code className="font-mono text-[11px] text-zinc-300 block mt-0.5">
                                        {selectedKeyForDetails.prefix ? `${selectedKeyForDetails.prefix}***` : 'None'}
                                    </code>
                                </div>
                                <div>
                                    <span className="text-[9px] uppercase font-bold text-muted-foreground block">Created Date</span>
                                    <span className="text-[11px] text-zinc-300 block mt-0.5">
                                        {new Date(selectedKeyForDetails.createdAt).toLocaleDateString()}
                                    </span>
                                </div>
                                <div>
                                    <span className="text-[9px] uppercase font-bold text-muted-foreground block">Status</span>
                                    <Badge variant={selectedKeyForDetails.isActive ? "success" : "destructive"} className="mt-0.5 text-[9px]">
                                        {selectedKeyForDetails.isActive ? "Active" : "Disabled"}
                                    </Badge>
                                </div>
                            </div>

                            {/* Edit Form */}
                            <form onSubmit={handleSaveEditKey} className="space-y-4">
                                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                    <div className="space-y-1">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Name</label>
                                        <Input
                                            value={editKeyName}
                                            onChange={(e) => setEditKeyName(e.target.value)}
                                            required
                                            disabled={!canCreateKeys}
                                            className="h-8 text-xs"
                                        />
                                    </div>
                                    <div className="space-y-1">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Environment</label>
                                        <select
                                            value={editKeyEnv}
                                            onChange={(e: any) => setEditKeyEnv(e.target.value)}
                                            disabled={!canCreateKeys}
                                            className="w-full text-xs bg-input-bg border border-border-color rounded-lg p-2 text-foreground h-8"
                                        >
                                            <option value="production">Production</option>
                                            <option value="staging">Staging</option>
                                            <option value="development">Development</option>
                                            <option value="testing">Testing</option>
                                        </select>
                                    </div>
                                </div>

                                <div className="space-y-1">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Description</label>
                                    <Input
                                        value={editKeyDesc}
                                        onChange={(e) => setEditKeyDesc(e.target.value)}
                                        disabled={!canCreateKeys}
                                        className="h-8 text-xs"
                                    />
                                </div>

                                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                    <div className="space-y-1">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed IP Filters</label>
                                        <Input
                                            value={editKeyIPs}
                                            onChange={(e) => setEditKeyIPs(e.target.value)}
                                            disabled={!canCreateKeys}
                                            className="h-8 text-xs font-mono"
                                        />
                                    </div>
                                    <div className="space-y-1">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed Domains / Origins</label>
                                        <Input
                                            value={editKeyOrigins}
                                            onChange={(e) => setEditKeyOrigins(e.target.value)}
                                            disabled={!canCreateKeys}
                                            className="h-8 text-xs font-mono"
                                        />
                                    </div>
                                </div>

                                <div className="flex flex-wrap items-center gap-6 p-3 rounded-lg border border-border-color bg-zinc-900/40">
                                    <label className="flex items-center gap-2 text-xs font-semibold text-foreground cursor-pointer">
                                        <input
                                            type="checkbox"
                                            checked={editKeyCanIngest}
                                            onChange={(e) => setEditKeyCanIngest(e.target.checked)}
                                            disabled={!canCreateKeys}
                                            className="rounded bg-input-bg border-border-color text-cyan-500"
                                        />
                                        Can Ingest Metrics
                                    </label>
                                    <label className="flex items-center gap-2 text-xs font-semibold text-foreground cursor-pointer">
                                        <input
                                            type="checkbox"
                                            checked={editKeyCanRead}
                                            onChange={(e) => setEditKeyCanRead(e.target.checked)}
                                            disabled={!canCreateKeys}
                                            className="rounded bg-input-bg border-border-color text-cyan-500"
                                        />
                                        Can Read Analytics
                                    </label>
                                </div>

                                {canCreateKeys && (
                                    <div className="flex items-center justify-between pt-3 border-t border-border-color/30">
                                        <div className="flex gap-2">
                                            <Button
                                                type="button"
                                                variant="outline"
                                                size="sm"
                                                onClick={() => handleRotateKey(selectedKeyForDetails.id || selectedKeyForDetails.keyId)}
                                                className="h-8 text-xs gap-1 border-amber-500/20 text-amber-400 hover:bg-amber-500/10 cursor-pointer"
                                            >
                                                <RefreshCw size={12} />
                                                Rotate Secret
                                            </Button>
                                            <Button
                                                type="button"
                                                variant="outline"
                                                size="sm"
                                                onClick={() => handleDeleteKey(selectedKeyForDetails.id || selectedKeyForDetails.keyId)}
                                                className="h-8 text-xs gap-1 border-rose-500/20 text-rose-400 hover:bg-rose-500/10 cursor-pointer"
                                            >
                                                <Trash2 size={12} />
                                                Delete Key
                                            </Button>
                                        </div>
                                        <Button
                                            type="submit"
                                            size="sm"
                                            className="h-8 text-xs gap-1 cursor-pointer"
                                            isLoading={updateKeyMutation.isPending}
                                        >
                                            <Save size={12} />
                                            Save Changes
                                        </Button>
                                    </div>
                                )}
                            </form>
                        </CardContent>
                    </Card>
                </div>
            )}
        </div>
    );
}
