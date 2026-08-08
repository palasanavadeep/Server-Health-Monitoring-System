"use client";

import { useState, useEffect, useMemo } from 'react';
import { useAuth } from '@/contexts/auth-context';
import { 
    useCreateClientMutation, 
    useClientApiKeysQuery, 
    useCreateApiKeyMutation,
    useCreateClientUserMutation,
    useUpdateApiKeyMutation,
    useDeleteApiKeyMutation,
    useDeactivateApiKeyMutation,
    useActivateApiKeyMutation,
    useRotateApiKeyMutation
} from '@/hooks/use-client-queries';
import { authApi, ApiKey } from '@/lib/api';
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
    User, 
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
    Search
} from 'lucide-react';
import { cn } from '@/lib/utils';

export default function ClientsPage() {
    const toast = useToast();
    const { user } = useAuth();
    
    // Check roles
    const isSuperAdmin = user?.role === 'super_admin';
    const isClientAdmin = user?.role === 'client_admin';
    const isClientViewer = user?.role === 'client_viewer';
    const canCreateKeys = isSuperAdmin || isClientAdmin;
    const canManageUsers = isSuperAdmin || isClientAdmin;

    // Selected Ingestion Workspace Client ID
    const [selectedClientId, setSelectedClientId] = useState<string>('');
    const [tempClientId, setTempClientId] = useState<string>('');
    const [sessionOnboardedClients, setSessionOnboardedClients] = useState<Array<{id: string, name: string}>>([]);

    // Local state for registered operators (to allow deactivating them)
    const [sessionOperators, setSessionOperators] = useState<Array<{id: string, username: string, email?: string, role: string, isActive: boolean}>>([]);

    // Workspace tab display states
    const [superAdminTab, setSuperAdminTab] = useState<'access' | 'onboard'>('access');
    const [activeWorkspaceTab, setActiveWorkspaceTab] = useState<'keys' | 'operators'>('keys');

    // Onboarding client form
    const [newClientName, setNewClientName] = useState('');
    const [newClientEmail, setNewClientEmail] = useState('');
    const [newClientDesc, setNewClientDesc] = useState('');
    const [newClientWeb, setNewClientWeb] = useState('');
    const [latestOnboardedClient, setLatestOnboardedClient] = useState<any>(null);

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

    // Operator User Form
    const [operatorUsername, setOperatorUsername] = useState('');
    const [operatorEmail, setOperatorEmail] = useState('');
    const [operatorPassword, setOperatorPassword] = useState('');
    const [operatorRole, setOperatorRole] = useState<'client_admin' | 'client_viewer'>('client_viewer');
    const [onboardingOperator, setOnboardingOperator] = useState(false);
    const [showOperatorPassword, setShowOperatorPassword] = useState(false);

    // Bind selected ID based on auth status
    useEffect(() => {
        if (user?.clientId) {
            setSelectedClientId(user.clientId);
        }
    }, [user]);

    // Queries and mutations
    const createClientMutation = useCreateClientMutation();
    const { data: apiKeys = [], isLoading: loadingKeys, refetch: refetchKeys } = useClientApiKeysQuery(selectedClientId);
    
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
    
    const createClientUserMutation = useCreateClientUserMutation(selectedClientId);

    // Handle Client Onboarding
    const handleCreateClient = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newClientName.trim()) return;

        try {
            const res = await createClientMutation.mutateAsync({
                name: newClientName,
                email: newClientEmail || undefined,
                description: newClientDesc || undefined,
                website: newClientWeb || undefined
            });

            toast('Tenant registered successfully!', 'success');
            setLatestOnboardedClient(res);
            
            // Add to session list so the super admin doesn't lose it
            if (res.id) {
                setSessionOnboardedClients(prev => [...prev, { id: res.id, name: res.name }]);
                // Auto switch to managing this tenant
                setSelectedClientId(res.id);
                setTempClientId(res.id);
            }
            
            setNewClientName('');
            setNewClientEmail('');
            setNewClientDesc('');
            setNewClientWeb('');
            setSuperAdminTab('access');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to onboard tenant', 'error');
        }
    };

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
                // Refresh local details view if modal is open for this key
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
            setSelectedKeyForDetails(null); // Close details modal to force reload from list query
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
            setSelectedKeyForDetails(null); // Close modal
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
            setSelectedKeyForDetails(null); // Close modal
            toast('API Key deleted permanently', 'info');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to delete API key', 'error');
        }
    };

    // Open Unified Detail Modal
    const handleOpenDetails = (key: ApiKey) => {
        setSelectedKeyForDetails(key);
        setShowModalKeyValue(false);
        
        // Preload form inputs inside popup
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

    // Handle User Onboarding
    const handleOnboardOperator = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!operatorUsername.trim() || !operatorPassword.trim() || !selectedClientId) return;

        setOnboardingOperator(true);
        try {
            const res = await createClientUserMutation.mutateAsync({
                username: operatorUsername,
                email: operatorEmail || undefined,
                password: operatorPassword,
                role: operatorRole
            });

            toast(`Operator '${operatorUsername}' onboarded successfully!`, 'success');
            
            // Add operator user details into local workspace state
            setSessionOperators(prev => [...prev, {
                id: res.id || res._id,
                username: res.username,
                email: res.email,
                role: res.role,
                isActive: true
            }]);

            setOperatorUsername('');
            setOperatorEmail('');
            setOperatorPassword('');
            setOperatorRole('client_viewer');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to register operator user', 'error');
        } finally {
            setOnboardingOperator(false);
        }
    };

    // Handle Operator User Deactivation
    const handleDeactivateOperator = async (operatorId: string) => {
        if (!confirm("Are you sure you want to deactivate this operator user? They will be blocked from logging in.")) {
            return;
        }

        try {
            await authApi.deactivateUser(operatorId);
            toast('Operator account deactivated successfully', 'info');
            setSessionOperators(prev => 
                prev.map(op => op.id === operatorId ? { ...op, isActive: false } : op)
            );
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to deactivate user', 'error');
        }
    };

    return (
        <div className="space-y-8 max-w-7xl mx-auto pb-12 animate-in fade-in duration-300">
            {/* Page Header */}
            <div className="border-b border-border-color/30 pb-5">
                <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                    Tenant Control Panel
                </h1>
                <p className="text-sm text-muted-foreground mt-1">
                    Manage Ingestion Gateways, configure CIDR/Origin filters, rotate tokens, and onboard security operators.
                </p>
            </div>

            {/* Top Workspace Selector Banner */}
            <div className="glass-panel p-5 rounded-2xl border border-cyan-500/10 bg-glass-card/65 shadow-lg shadow-cyan-500/5 space-y-4">
                {isSuperAdmin ? (
                    <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                        <div className="flex flex-wrap items-center gap-4">
                            <div className="flex items-center gap-2 font-bold text-foreground">
                                <Building className="text-cyan-400 w-5 h-5" />
                                <span className="text-sm">Tenant Workspace ID:</span>
                            </div>
                            <div className="flex items-center gap-2">
                                <Input
                                    value={tempClientId}
                                    onChange={(e) => setTempClientId(e.target.value)}
                                    placeholder="Paste Client UUID"
                                    className="font-mono text-xs w-64 h-9"
                                />
                                <Button 
                                    size="sm"
                                    className="text-xs h-9 cursor-pointer shrink-0" 
                                    onClick={() => {
                                        if (tempClientId.trim()) {
                                            setSelectedClientId(tempClientId.trim());
                                            toast('Workspace scoped to Client ID', 'success');
                                        }
                                    }}
                                >
                                    Load Workspace
                                </Button>
                            </div>
                            
                            {sessionOnboardedClients.length > 0 && (
                                <div className="flex items-center gap-2">
                                    <span className="text-xs text-muted-foreground">Recent:</span>
                                    <select
                                        value={selectedClientId}
                                        onChange={(e) => {
                                            setSelectedClientId(e.target.value);
                                            setTempClientId(e.target.value);
                                        }}
                                        className="bg-input-bg border border-border-color rounded-lg text-xs p-2 text-foreground outline-none focus:border-cyan-500"
                                    >
                                        <option value="">-- Choose Tenant --</option>
                                        {sessionOnboardedClients.map(c => (
                                            <option key={c.id} value={c.id}>{c.name}</option>
                                        ))}
                                    </select>
                                </div>
                            )}
                        </div>

                        <div className="flex gap-2">
                            <Button 
                                variant="secondary" 
                                size="sm" 
                                onClick={() => setSuperAdminTab(superAdminTab === 'onboard' ? 'access' : 'onboard')}
                                className="text-xs h-9 gap-1.5 cursor-pointer"
                            >
                                <Plus size={13} />
                                {superAdminTab === 'onboard' ? "Hide Register Form" : "Register New Client"}
                            </Button>
                        </div>
                    </div>
                ) : (
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
                            Tenant Locked
                        </Badge>
                    </div>
                )}

                {/* Collapsible Super Admin Onboarding Form */}
                {isSuperAdmin && superAdminTab === 'onboard' && (
                    <form onSubmit={handleCreateClient} className="p-4 rounded-xl border border-cyan-500/10 bg-zinc-950/40 space-y-4 animate-in slide-in-from-top-2 duration-200">
                        <div className="flex items-center gap-2 border-b border-border-color/20 pb-2">
                            <Building size={16} className="text-cyan-400" />
                            <h4 className="text-xs font-bold text-foreground">Onboard New Client Tenant</h4>
                        </div>
                        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                            <div className="space-y-1">
                                <label className="text-[9px] font-bold text-muted-foreground uppercase block">Tenant Name</label>
                                <Input
                                    value={newClientName}
                                    onChange={(e) => setNewClientName(e.target.value)}
                                    placeholder="e.g. Acme Corp"
                                    className="h-8 text-xs"
                                    required
                                />
                            </div>
                            <div className="space-y-1">
                                <label className="text-[9px] font-bold text-muted-foreground uppercase block">Contact Email</label>
                                <Input
                                    type="email"
                                    value={newClientEmail}
                                    onChange={(e) => setNewClientEmail(e.target.value)}
                                    placeholder="ops@acme.com"
                                    className="h-8 text-xs"
                                />
                            </div>
                            <div className="space-y-1">
                                <label className="text-[9px] font-bold text-muted-foreground uppercase block">Website URL</label>
                                <Input
                                    value={newClientWeb}
                                    onChange={(e) => setNewClientWeb(e.target.value)}
                                    placeholder="https://acme.com"
                                    className="h-8 text-xs"
                                />
                            </div>
                            <div className="space-y-1">
                                <label className="text-[9px] font-bold text-muted-foreground uppercase block">Description</label>
                                <Input
                                    value={newClientDesc}
                                    onChange={(e) => setNewClientDesc(e.target.value)}
                                    placeholder="Brief purpose description..."
                                    className="h-8 text-xs"
                                />
                            </div>
                        </div>
                        <div className="flex justify-end pt-1">
                            <Button 
                                type="submit" 
                                className="text-xs h-8 px-4 gap-1.5 cursor-pointer" 
                                isLoading={createClientMutation.isPending}
                            >
                                <Building size={12} />
                                Submit Tenant Registration
                            </Button>
                        </div>
                    </form>
                )}

                {latestOnboardedClient && isSuperAdmin && (
                    <Card className="border-emerald-500/20 bg-emerald-500/5 animate-in slide-in-from-top-3 duration-300">
                        <CardContent className="p-4 flex items-center justify-between gap-4 text-xs">
                            <div className="flex items-center gap-2">
                                <ShieldCheck size={16} className="text-emerald-400" />
                                <div>
                                    <span className="font-bold text-foreground block">Tenant Organization Registered!</span>
                                    <span className="text-muted-foreground">ID: <code className="font-mono text-emerald-300">{latestOnboardedClient.id}</code></span>
                                </div>
                            </div>
                            <Button 
                                size="sm" 
                                variant="outline" 
                                onClick={() => {
                                    navigator.clipboard.writeText(latestOnboardedClient.id);
                                    toast('Client ID copied to clipboard', 'success');
                                }}
                                className="h-8 text-xs gap-1.5 cursor-pointer"
                            >
                                <Copy size={12} />
                                Copy Tenant ID
                            </Button>
                        </CardContent>
                    </Card>
                )}
            </div>

            {/* Main Content Area */}
            {selectedClientId ? (
                <div className="space-y-6">
                    {/* Tab Navigation header */}
                    <div className="flex items-center justify-between border-b border-border-color/30 pb-3">
                        <div className="flex gap-4">
                            <button
                                onClick={() => setActiveWorkspaceTab('keys')}
                                className={cn(
                                    "pb-2 text-sm font-bold border-b-2 transition-all cursor-pointer",
                                    activeWorkspaceTab === 'keys'
                                        ? "border-cyan-500 text-cyan-400"
                                        : "border-transparent text-muted-foreground hover:text-foreground"
                                )}
                            >
                                Ingestion Keys
                            </button>
                            <button
                                onClick={() => setActiveWorkspaceTab('operators')}
                                className={cn(
                                    "pb-2 text-sm font-bold border-b-2 transition-all cursor-pointer",
                                    activeWorkspaceTab === 'operators'
                                        ? "border-cyan-500 text-cyan-400"
                                        : "border-transparent text-muted-foreground hover:text-foreground"
                                )}
                            >
                                Onboard Operator Users
                            </button>
                        </div>
                    </div>

                    {/* Tab: Keys Display */}
                    {activeWorkspaceTab === 'keys' && (
                        <div className="space-y-6">
                            {/* Provision New Ingestion Key */}
                            {canCreateKeys && (
                                <Card className="border-cyan-500/5 bg-glass-card/10">
                                    <CardHeader className="pb-3">
                                        <CardTitle className="text-sm">Provision New Ingestion Key</CardTitle>
                                        <CardDescription className="text-xs">Configure Ingest Credentials, Expirations, Permissions, and Network Scopes</CardDescription>
                                    </CardHeader>
                                    <CardContent>
                                        <form onSubmit={handleGenerateKey} className="space-y-4">
                                            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                                {/* Key Name */}
                                                <div className="space-y-1.5">
                                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Name *</label>
                                                    <Input
                                                        value={newKeyName}
                                                        onChange={(e) => setNewKeyName(e.target.value)}
                                                        placeholder="e.g. production-gateway"
                                                        required
                                                    />
                                                </div>

                                                {/* Environment */}
                                                <div className="space-y-1.5">
                                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Target Environment</label>
                                                    <select
                                                        value={newKeyEnv}
                                                        onChange={(e: any) => setNewKeyEnv(e.target.value)}
                                                        className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground"
                                                    >
                                                        <option value="production">Production</option>
                                                        <option value="staging">Staging</option>
                                                        <option value="development">Development</option>
                                                        <option value="testing">Testing</option>
                                                    </select>
                                                </div>

                                                {/* Expires At (Minutes) */}
                                                <div className="space-y-1.5">
                                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Lifetime Expiry</label>
                                                    <select
                                                        value={newKeyExpires}
                                                        onChange={(e) => setNewKeyExpires(Number(e.target.value))}
                                                        className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground"
                                                    >
                                                        <option value={24}>24 Minutes (Temporary)</option>
                                                        <option value={60}>1 Hour</option>
                                                        <option value={1440}>1 Day (24 hours)</option>
                                                        <option value={43200}>30 Days</option>
                                                        <option value={525600}>365 Days (1 Year)</option>
                                                    </select>
                                                </div>
                                            </div>

                                            {/* Description & Restrict Services */}
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
                                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Restrict to Service Names (Optional, comma-separated)</label>
                                                    <Input
                                                        value={newKeyServices}
                                                        onChange={(e) => setNewKeyServices(e.target.value)}
                                                        placeholder="e.g. rust-ingest, backend-auth (empty for all)"
                                                    />
                                                </div>
                                            </div>

                                            {/* Security Rules: Allowed IPs, Allowed Origins */}
                                            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                                <div className="space-y-1.5">
                                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed IP Filters (Comma-separated)</label>
                                                    <Input
                                                        value={newKeyIPs}
                                                        onChange={(e) => setNewKeyIPs(e.target.value)}
                                                        placeholder="e.g. 192.168.1.0/24, 0.0.0.0/0"
                                                    />
                                                </div>
                                                <div className="space-y-1.5">
                                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed Domains / Origins (Comma-separated)</label>
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

                                            {/* Permissions Toggles */}
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
                                            <CardDescription className="text-xs">Click on any key to directly open its configuration control panel.</CardDescription>
                                        </div>
                                        
                                        {/* Search Bar Input */}
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
                                                {keySearchQuery ? "No matches found for your filter." : "There are no active ingestion credentials mapped to this client gateway."}
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
                                                        key={key.id || key.keyId || (key as any)._id || `apikey-${index}`}
                                                        onClick={() => handleOpenDetails(key)}
                                                        className="cursor-pointer hover:bg-glass-card-hover/40 transition-colors select-none"
                                                    >
                                                        {/* Descriptor and prefix */}
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
                                                        
                                                        {/* Environment */}
                                                        <TableCell className="w-1/5">
                                                            <Badge variant="outline" className="font-mono text-[9px] uppercase tracking-wider border-border-color bg-zinc-950/20">
                                                                {key.environment || 'production'}
                                                            </Badge>
                                                        </TableCell>

                                                        {/* Created Date */}
                                                        <TableCell className="text-xs text-muted-foreground w-1/5">
                                                            {new Date(key.createdAt).toLocaleDateString()}
                                                        </TableCell>
                                                        
                                                        {/* Status */}
                                                        <TableCell className="w-1/5">
                                                            <Badge variant={key.isActive ? "success" : "destructive"}>
                                                                {key.isActive ? "Active" : "Disabled"}
                                                            </Badge>
                                                        </TableCell>
                                                        
                                                        {/* Quick toggle switch */}
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
                    )}

                    {/* Tab: Operators Onboarding Form */}
                    {activeWorkspaceTab === 'operators' && (
                        <div className="space-y-6">
                            {/* Onboard Operator Form */}
                            {canManageUsers && (
                                <Card>
                                    <CardHeader>
                                        <CardTitle className="text-base flex items-center gap-2">
                                            <Users className="text-cyan-400 w-4 h-4" />
                                            Onboard Operator User
                                        </CardTitle>
                                        <CardDescription className="text-xs">
                                            Register a new client user to log in and review telemetry metrics for this client tenant.
                                        </CardDescription>
                                    </CardHeader>
                                    <CardContent>
                                        <form onSubmit={handleOnboardOperator} className="max-w-md space-y-4">
                                            <div className="space-y-1.5">
                                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Username</label>
                                                <Input
                                                    value={operatorUsername}
                                                    onChange={(e) => setOperatorUsername(e.target.value)}
                                                    placeholder="Enter operator username"
                                                    required
                                                />
                                            </div>
                                            <div className="space-y-1.5">
                                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Email Address</label>
                                                <Input
                                                    type="email"
                                                    value={operatorEmail}
                                                    onChange={(e) => setOperatorEmail(e.target.value)}
                                                    placeholder="operator@acme.com"
                                                />
                                            </div>
                                            <div className="space-y-1.5">
                                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Default Password</label>
                                                <div className="relative">
                                                    <Input
                                                        type={showOperatorPassword ? "text" : "password"}
                                                        value={operatorPassword}
                                                        onChange={(e) => setOperatorPassword(e.target.value)}
                                                        placeholder="Choose secure password"
                                                        className="pr-10"
                                                        required
                                                    />
                                                    <button
                                                        type="button"
                                                        onClick={() => setShowOperatorPassword(!showOperatorPassword)}
                                                        className="absolute inset-y-0 right-0 pr-3 flex items-center text-muted-foreground hover:text-foreground cursor-pointer"
                                                    >
                                                        {showOperatorPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                                                    </button>
                                                </div>
                                            </div>
                                            <div className="space-y-1.5">
                                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Operator Role Access</label>
                                                <select
                                                    value={operatorRole}
                                                    onChange={(e: any) => setOperatorRole(e.target.value)}
                                                    className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground"
                                                >
                                                    <option value="client_viewer">Client Viewer (Read Only)</option>
                                                    <option value="client_admin">Client Admin (Read & Write API keys/Users)</option>
                                                </select>
                                            </div>
                                            <Button 
                                                type="submit" 
                                                className="w-full text-xs h-9 gap-1.5 mt-2 cursor-pointer" 
                                                isLoading={onboardingOperator}
                                            >
                                                <User size={14} />
                                                Register operator user
                                            </Button>
                                        </form>
                                    </CardContent>
                                </Card>
                            )}

                            {/* Session Operators Table */}
                            {sessionOperators.length > 0 && (
                                <Card>
                                    <CardHeader>
                                        <CardTitle className="text-sm">Registered Operators (This Session)</CardTitle>
                                        <CardDescription className="text-xs">Manage active sessions and disable access credentials instantly</CardDescription>
                                    </CardHeader>
                                    <CardContent className="p-0">
                                        <Table>
                                            <TableHeader>
                                                <TableRow>
                                                    <TableHead>Username</TableHead>
                                                    <TableHead>Email</TableHead>
                                                    <TableHead>Role</TableHead>
                                                    <TableHead>Security State</TableHead>
                                                    <TableHead className="text-right">Actions</TableHead>
                                                </TableRow>
                                            </TableHeader>
                                            <TableBody>
                                                {sessionOperators.map((op, index) => (
                                                    <TableRow key={op.id || index}>
                                                        <TableCell className="font-semibold text-sm">{op.username}</TableCell>
                                                        <TableCell className="text-xs text-muted-foreground">{op.email || 'N/A'}</TableCell>
                                                        <TableCell>
                                                            <Badge variant="outline" className="text-[10px] uppercase font-mono">
                                                                {op.role.replace('_', ' ')}
                                                            </Badge>
                                                        </TableCell>
                                                        <TableCell>
                                                            <Badge variant={op.isActive ? "success" : "destructive"}>
                                                                {op.isActive ? "Active" : "Deactivated"}
                                                            </Badge>
                                                        </TableCell>
                                                        <TableCell className="text-right">
                                                            {op.isActive && canManageUsers ? (
                                                                <Button
                                                                    variant="outline"
                                                                    size="sm"
                                                                    onClick={() => handleDeactivateOperator(op.id)}
                                                                    className="h-7 px-2 border-rose-500/20 text-rose-400 hover:bg-rose-500/10 text-xs cursor-pointer"
                                                                >
                                                                    Deactivate
                                                                </Button>
                                                            ) : (
                                                                <span className="text-[10px] text-muted-foreground">Disabled</span>
                                                            )}
                                                        </TableCell>
                                                    </TableRow>
                                                ))}
                                            </TableBody>
                                        </Table>
                                    </CardContent>
                                </Card>
                            )}

                            {!canManageUsers && (
                                <Card>
                                    <CardContent className="p-6 text-center border border-dashed border-border-color rounded-xl bg-glass-card/10">
                                        <ShieldAlert className="w-8 h-8 text-rose-400 mx-auto mb-2" />
                                        <p className="text-sm font-semibold text-foreground">Access Denied</p>
                                        <p className="text-xs text-muted-foreground mt-1">
                                            Your operator role level ({user?.role}) does not have permission to provision new client users.
                                        </p>
                                    </CardContent>
                                </Card>
                            )}
                        </div>
                    )}
                </div>
            ) : (
                <Card className="h-64 flex flex-col items-center justify-center text-center p-8 border border-dashed border-border-color bg-glass-card/10 select-none">
                    <Activity className="w-12 h-12 text-muted-foreground/20 mb-3" />
                    <p className="text-sm font-semibold text-muted-foreground">Select Ingestion Tenant Scope</p>
                    <p className="text-xs text-muted-foreground/70 max-w-[340px] mt-1">
                        Paste a client ID under "Tenant Workspace ID" in the banner widget above to explore ingestion credentials and manage operational users.
                    </p>
                </Card>
            )}

            {/* UNIFIED INTERACTIVE CONFIGURATION FORM MODAL */}
            {selectedKeyForDetails && (
                <div className="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center p-4 overflow-y-auto">
                    <form 
                        onSubmit={handleSaveEditKey} 
                        className="relative w-full max-w-2xl glass-panel p-6 rounded-2xl shadow-2xl border border-cyan-500/20 bg-zinc-950 animate-in zoom-in-95 duration-200 my-8 space-y-5"
                    >
                        {/* Close button */}
                        <button 
                            type="button"
                            onClick={() => setSelectedKeyForDetails(null)}
                            className="absolute top-4 right-4 text-muted-foreground hover:text-foreground p-1 rounded-lg border border-border-color/60 hover:bg-white/5 transition-colors cursor-pointer"
                        >
                            <X size={15} />
                        </button>

                        {/* Title Row */}
                        <div className="pb-3 border-b border-border-color/30 flex items-center gap-2">
                            <Sliders size={20} className="text-cyan-400 animate-pulse" />
                            <h3 className="text-lg font-bold text-foreground">API Key Configurator</h3>
                            <Badge variant={selectedKeyForDetails.isActive ? "success" : "destructive"} className="ml-2 font-mono">
                                {selectedKeyForDetails.isActive ? "Active" : "Inactive"}
                            </Badge>
                        </div>

                        {/* 1. API KEY DISPLAY & MASK REVEAL (Highlighted Block) */}
                        <div className="p-3 rounded-xl border border-cyan-500/10 bg-zinc-900/60 space-y-1.5">
                            <div className="flex items-center justify-between">
                                <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">API KEY VALUE</span>
                                <span className="text-[9px] text-cyan-400/70 font-semibold uppercase tracking-widest">Gateway Credential</span>
                            </div>
                            <div className="flex items-center justify-between gap-4 font-mono text-xs text-cyan-400 select-all break-all leading-normal">
                                <span>
                                    {showModalKeyValue 
                                        ? (selectedKeyForDetails.keyValue || selectedKeyForDetails.key || 'No Value') 
                                        : `sm_key_${selectedKeyForDetails.prefix || '***'}************************************`}
                                </span>
                                <div className="flex items-center gap-1 shrink-0">
                                    <button
                                        type="button"
                                        onClick={() => setShowModalKeyValue(!showModalKeyValue)}
                                        className="p-1.5 rounded border border-border-color/55 text-muted-foreground hover:text-cyan-400 hover:bg-cyan-500/10 transition-colors cursor-pointer"
                                    >
                                        {showModalKeyValue ? <EyeOff size={14} /> : <Eye size={14} />}
                                    </button>
                                    <button
                                        type="button"
                                        onClick={() => handleCopyKey(selectedKeyForDetails.keyValue || selectedKeyForDetails.key || '')}
                                        className={cn(
                                            "p-1.5 rounded border border-border-color/55 transition-colors cursor-pointer",
                                            copied ? "text-emerald-400 border-emerald-500/30 bg-emerald-500/10" : "text-muted-foreground hover:text-cyan-400 hover:bg-cyan-500/10"
                                        )}
                                    >
                                        {copied ? <Check size={14} /> : <ClipboardCopy size={14} />}
                                    </button>
                                </div>
                            </div>
                        </div>

                        {/* 2. CONFIG FORM INPUTS */}
                        <div className="space-y-4 text-xs">
                            {/* Key Name & Target Environment */}
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Name *</label>
                                    <Input
                                        value={editKeyName}
                                        onChange={(e) => setEditKeyName(e.target.value)}
                                        placeholder="Name this key descriptor"
                                        required
                                        disabled={!canCreateKeys}
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Environment</label>
                                    <select
                                        value={editKeyEnv}
                                        onChange={(e: any) => setEditKeyEnv(e.target.value)}
                                        disabled={!canCreateKeys}
                                        className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground"
                                    >
                                        <option value="production">Production</option>
                                        <option value="staging">Staging</option>
                                        <option value="development">Development</option>
                                        <option value="testing">Testing</option>
                                    </select>
                                </div>
                            </div>

                            {/* Description & Restrict Services */}
                            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Key Description</label>
                                    <Input
                                        value={editKeyDesc}
                                        onChange={(e) => setEditKeyDesc(e.target.value)}
                                        placeholder="API key purpose or notes..."
                                        disabled={!canCreateKeys}
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Restrict to Service Names (Comma-separated)</label>
                                    <Input
                                        value={editKeyServices}
                                        onChange={(e) => setEditKeyServices(e.target.value)}
                                        placeholder="e.g. rust-ingest, user-auth"
                                        disabled={!canCreateKeys}
                                    />
                                </div>
                            </div>

                            {/* Network & Warning constraints */}
                            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed IP Filters (Comma-separated)</label>
                                    <Input
                                        value={editKeyIPs}
                                        onChange={(e) => setEditKeyIPs(e.target.value)}
                                        placeholder="e.g. 192.168.1.0/24, 0.0.0.0/0"
                                        disabled={!canCreateKeys}
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Allowed Origins (Comma-separated)</label>
                                    <Input
                                        value={editKeyOrigins}
                                        onChange={(e) => setEditKeyOrigins(e.target.value)}
                                        placeholder="e.g. *.domain.com, *"
                                        disabled={!canCreateKeys}
                                    />
                                </div>
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Rotation warning (Days)</label>
                                    <Input
                                        type="number"
                                        value={editKeyWarnDays}
                                        onChange={(e) => setEditKeyWarnDays(Number(e.target.value))}
                                        min={1}
                                        max={365}
                                        disabled={!canCreateKeys}
                                    />
                                </div>
                            </div>

                            {/* Permissions Scope checkboxes */}
                            <div className="flex flex-wrap items-center gap-6 p-3 rounded-lg border border-border-color bg-glass-card/20">
                                <span className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Permissions Scope:</span>
                                <label className="flex items-center gap-2 text-xs font-semibold text-foreground cursor-pointer select-none">
                                    <input 
                                        type="checkbox" 
                                        checked={editKeyCanIngest} 
                                        onChange={(e) => setEditKeyCanIngest(e.target.checked)}
                                        disabled={!canCreateKeys}
                                        className="rounded bg-input-bg border-border-color text-cyan-500 focus:ring-cyan-500"
                                    />
                                    Ingest Analytics Data
                                </label>
                                <label className="flex items-center gap-2 text-xs font-semibold text-foreground cursor-pointer select-none">
                                    <input 
                                        type="checkbox" 
                                        checked={editKeyCanRead} 
                                        onChange={(e) => setEditKeyCanRead(e.target.checked)}
                                        disabled={!canCreateKeys}
                                        className="rounded bg-input-bg border-border-color text-cyan-500 focus:ring-cyan-500"
                                    />
                                    Read Telemetry Aggregations
                                </label>
                            </div>
                        </div>

                        {/* 3. META TIMELINES (Info Row) */}
                        <div className="p-3 rounded-xl border border-border-color bg-glass-card/5 grid grid-cols-1 sm:grid-cols-2 gap-3 text-[10px] font-semibold text-muted-foreground leading-normal">
                            <div className="space-y-0.5">
                                <span className="block text-[8px] uppercase font-bold tracking-wider leading-none">Created By Info</span>
                                <div className="text-foreground mt-0.5 font-bold">
                                    {selectedKeyForDetails.createdBy && typeof selectedKeyForDetails.createdBy === 'object' ? (
                                        <span>
                                            {selectedKeyForDetails.createdBy.username}{' '}
                                            <span className="text-[9px] font-normal text-muted-foreground">({selectedKeyForDetails.createdBy.email})</span>
                                        </span>
                                    ) : (
                                        <span>System Operator</span>
                                    )}
                                </div>
                                <span className="block text-[9px] font-medium text-muted-foreground/80">Issued: {new Date(selectedKeyForDetails.createdAt).toLocaleString()}</span>
                            </div>
                            <div className="space-y-0.5">
                                <span className="block text-[8px] uppercase font-bold tracking-wider leading-none">Expiration Schedule</span>
                                <div className="text-foreground mt-0.5 font-bold flex items-center gap-1">
                                    <Clock size={11} className="text-amber-400" />
                                    <span>Expires: {selectedKeyForDetails.expiresAt ? new Date(selectedKeyForDetails.expiresAt).toLocaleString() : 'Never (TTL Disabled)'}</span>
                                </div>
                                <span className="block text-[9px] font-medium text-muted-foreground/80">Warn alert days: {selectedKeyForDetails.security?.rotationWarningDays || 30}</span>
                            </div>
                        </div>

                        {/* 4. ACTIONS FOOTER DECK */}
                        <div className="pt-4 border-t border-border-color/30 flex flex-col sm:flex-row items-center justify-between gap-3 bg-zinc-950/20 -mx-6 -mb-6 p-4 rounded-b-2xl">
                            <Button 
                                type="button" 
                                variant="outline" 
                                size="sm" 
                                onClick={() => setSelectedKeyForDetails(null)}
                                className="w-full sm:w-auto text-xs cursor-pointer"
                            >
                                Cancel
                            </Button>

                            {canCreateKeys ? (
                                <div className="flex flex-wrap items-center justify-end gap-2 w-full sm:w-auto">
                                    {/* Activate / Deactivate status */}
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleToggleKeyStatus(
                                            selectedKeyForDetails.id || selectedKeyForDetails.keyId,
                                            selectedKeyForDetails.isActive
                                        )}
                                        className={cn(
                                            "text-xs gap-1.5 cursor-pointer border-transparent",
                                            selectedKeyForDetails.isActive
                                                ? "bg-orange-600/20 hover:bg-orange-600/35 text-orange-400 border border-orange-500/20"
                                                : "bg-emerald-600/20 hover:bg-emerald-600/35 text-emerald-400 border border-emerald-500/20"
                                        )}
                                    >
                                        <Power size={13} />
                                        {selectedKeyForDetails.isActive ? "Deactivate Key" : "Activate Key"}
                                    </Button>

                                    {/* Rotate token credentials */}
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleRotateKey(selectedKeyForDetails.id || selectedKeyForDetails.keyId)}
                                        className="text-xs gap-1.5 cursor-pointer border-amber-500/20 text-amber-400 hover:bg-amber-500/10"
                                    >
                                        <RefreshCw size={13} />
                                        Rotate Token
                                    </Button>

                                    {/* Delete credentials permanently */}
                                    <Button
                                        type="button"
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleDeleteKey(selectedKeyForDetails.id || selectedKeyForDetails.keyId)}
                                        className="text-xs gap-1.5 cursor-pointer border-rose-500/20 text-rose-400 hover:bg-rose-500/10"
                                    >
                                        <Trash2 size={13} />
                                        Delete Key
                                    </Button>

                                    {/* Save Edits */}
                                    <Button 
                                        type="submit" 
                                        size="sm" 
                                        isLoading={updateKeyMutation.isPending}
                                        className="text-xs gap-1.5 cursor-pointer"
                                    >
                                        <Save size={13} />
                                        Save Changes
                                    </Button>
                                </div>
                            ) : (
                                <span className="text-[10px] text-muted-foreground font-semibold">View Only Mode</span>
                            )}
                        </div>
                    </form>
                </div>
            )}

            {/* Generated Raw API Key Dialog modal popups */}
            {isKeyModalOpen && generatedKey && (
                <div className="fixed inset-0 bg-background/80 backdrop-blur-sm z-50 flex items-center justify-center p-4">
                    <div className="relative w-full max-w-md glass-panel p-6 rounded-xl shadow-2xl border border-cyan-500/20 bg-zinc-950 animate-in zoom-in-95 duration-200">
                        <div className="flex flex-col items-center text-center space-y-3">
                            <div className="flex items-center justify-center w-12 h-12 rounded-full bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
                                <KeyRound size={24} />
                            </div>
                            <h3 className="text-lg font-bold text-foreground">API Token Generated</h3>
                            <p className="text-xs text-muted-foreground">
                                Copy this token now. It will not be shown again for security reasons.
                            </p>
                        </div>

                        <div className="mt-4 p-3 rounded-lg border border-cyan-500/10 bg-zinc-900/60 flex items-center justify-between gap-4 font-mono text-xs text-cyan-400 select-all break-all leading-normal">
                            <span>{generatedKey}</span>
                            <button
                                onClick={() => handleCopyKey(generatedKey)}
                                className={cn(
                                    "p-2 rounded-lg transition-colors cursor-pointer shrink-0 border border-transparent hover:border-cyan-500/20 hover:bg-cyan-500/10",
                                    copied ? "text-emerald-400" : "text-cyan-400"
                                )}
                            >
                                {copied ? <Check size={14} /> : <ClipboardCopy size={14} />}
                            </button>
                        </div>

                        <div className="mt-6 flex justify-end">
                            <Button size="sm" onClick={() => {
                                setIsKeyModalOpen(false);
                                setGeneratedKey(null);
                                refetchKeys();
                            }}>
                                I Have Saved This Token
                            </Button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
