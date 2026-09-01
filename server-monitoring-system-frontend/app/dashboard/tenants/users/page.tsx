"use client";

import { useState, useEffect } from 'react';
import { notFound, useSearchParams } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useCreateClientUserMutation } from '@/hooks/use-client-queries';
import { authApi } from '@/lib/api';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useToast } from '@/contexts/toast-context';
import { 
    Users, 
    UserPlus, 
    Building, 
    Mail, 
    Lock, 
    Eye, 
    EyeOff, 
    Shield, 
    UserMinus, 
    ArrowLeft
} from 'lucide-react';
import Link from 'next/link';

interface ProvisionedUser {
    id: string;
    clientId: string;
    username: string;
    email?: string;
    role: string;
    isActive: boolean;
    createdAt: string;
}

export default function TenantUsersPage() {
    const searchParams = useSearchParams();
    const toast = useToast();
    const { user, loading } = useAuth();

    // Route guard: only super_admin can view this page
    if (!loading && user && user.role !== 'super_admin') {
        notFound();
    }

    // Pre-populate clientId from query params if passed from /dashboard/tenants
    const [targetClientId, setTargetClientId] = useState('');
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [role, setRole] = useState<'client_admin' | 'client_viewer'>('client_admin');
    const [showPassword, setShowPassword] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    // Saved tenants from local storage to provide a quick dropdown
    const [savedTenants, setSavedTenants] = useState<Array<{ id: string; name: string }>>([]);

    // Provisioned users list
    const [provisionedUsers, setProvisionedUsers] = useState<ProvisionedUser[]>(() => {
        if (typeof window !== 'undefined') {
            try {
                const saved = localStorage.getItem('sm_provisioned_tenant_users');
                return saved ? JSON.parse(saved) : [];
            } catch {
                return [];
            }
        }
        return [];
    });

    useEffect(() => {
        const queryCid = searchParams.get('clientId');
        if (queryCid) {
            setTargetClientId(queryCid);
        }

        if (typeof window !== 'undefined') {
            try {
                const tenantsRaw = localStorage.getItem('sm_onboarded_tenants');
                if (tenantsRaw) {
                    const parsed = JSON.parse(tenantsRaw);
                    setSavedTenants(parsed.map((t: any) => ({ id: t.id, name: t.name })));
                }
            } catch (e) {
                console.error(e);
            }
        }
    }, [searchParams]);

    const createClientUserMutation = useCreateClientUserMutation(targetClientId);

    const handleCreateUser = async (e: React.FormEvent) => {
        e.preventDefault();
        const cid = targetClientId.trim();
        if (!cid) {
            toast('Please enter or select a Target Tenant Client ID', 'error');
            return;
        }

        if (!username.trim() || !email.trim() || !password.trim()) {
            toast('All fields (Username, Email, Password) are required', 'error');
            return;
        }

        setIsSubmitting(true);
        try {
            const res = await createClientUserMutation.mutateAsync({
                username: username.trim(),
                email: email.trim(),
                password: password.trim(),
                role,
            });

            const newUser: ProvisionedUser = {
                id: res.id,
                clientId: cid,
                username: res.username || username.trim(),
                email: res.email || email.trim(),
                role: res.role || role,
                isActive: true,
                createdAt: new Date().toISOString(),
            };

            const updated = [newUser, ...provisionedUsers];
            setProvisionedUsers(updated);
            if (typeof window !== 'undefined') {
                try {
                    localStorage.setItem('sm_provisioned_tenant_users', JSON.stringify(updated));
                } catch (e) {
                    console.error(e);
                }
            }

            toast(`User '${newUser.username}' successfully registered in tenant!`, 'success');
            setUsername('');
            setEmail('');
            setPassword('');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to create user in tenant', 'error');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleDeactivateUser = async (userId: string) => {
        if (!confirm('Are you sure you want to deactivate this tenant user account?')) {
            return;
        }

        try {
            await authApi.deactivateUser(userId);
            const updated = provisionedUsers.map(u => u.id === userId ? { ...u, isActive: false } : u);
            setProvisionedUsers(updated);
            if (typeof window !== 'undefined') {
                try {
                    localStorage.setItem('sm_provisioned_tenant_users', JSON.stringify(updated));
                } catch (e) {
                    console.error(e);
                }
            }
            toast('User deactivated successfully', 'info');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to deactivate user', 'error');
        }
    };

    if (loading || (user && user.role !== 'super_admin')) {
        return null;
    }

    return (
        <div className="space-y-8 max-w-6xl mx-auto pb-12 animate-in fade-in duration-300">
            {/* Header */}
            <div className="border-b border-border-color/30 pb-5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
                    <div className="flex items-center gap-2 mb-1">
                        <Link href="/dashboard/tenants" className="text-xs text-muted-foreground hover:text-cyan-400 flex items-center gap-1">
                            <ArrowLeft size={12} />
                            Back to Tenants
                        </Link>
                    </div>
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                        <Users className="text-cyan-400 w-7 h-7" />
                        Tenant User Provisioning
                    </h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Create administrative and viewer user accounts inside any tenant organization by Client ID.
                    </p>
                </div>
                <Badge variant="outline" className="font-mono text-[10px] uppercase border-cyan-500/30 text-cyan-400 bg-cyan-500/5 px-3 py-1 self-start sm:self-auto">
                    Super Admin Console
                </Badge>
            </div>

            {/* Create Tenant User Form */}
            <Card className="border-cyan-500/20 bg-glass-card/50 shadow-xl shadow-cyan-500/5">
                <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                        <UserPlus className="text-cyan-400 w-4 h-4" />
                        Provision User in Tenant
                    </CardTitle>
                    <CardDescription className="text-xs">
                        Specify target Client ID and credentials to grant tenant operator access.
                    </CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleCreateUser} className="space-y-4">
                        {/* Target Client ID */}
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Target Tenant Client ID (UUID) *
                                </label>
                                <div className="relative">
                                    <Input
                                        value={targetClientId}
                                        onChange={(e) => setTargetClientId(e.target.value)}
                                        placeholder="Paste or enter Client ID"
                                        required
                                        className="font-mono text-xs pl-8"
                                    />
                                    <Building size={13} className="absolute left-2.5 top-3 text-muted-foreground" />
                                </div>
                            </div>

                            {savedTenants.length > 0 && (
                                <div className="space-y-1.5">
                                    <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                        Or Select From Registered Tenants
                                    </label>
                                    <select
                                        value={targetClientId}
                                        onChange={(e) => setTargetClientId(e.target.value)}
                                        className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground h-9"
                                    >
                                        <option value="">-- Choose Registered Tenant --</option>
                                        {savedTenants.map((t) => (
                                            <option key={t.id} value={t.id}>
                                                {t.name} ({t.id.substring(0, 8)}...)
                                            </option>
                                        ))}
                                    </select>
                                </div>
                            )}
                        </div>

                        {/* User Credentials */}
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Operator Username *
                                </label>
                                <Input
                                    value={username}
                                    onChange={(e) => setUsername(e.target.value)}
                                    placeholder="e.g. acme_operator"
                                    required
                                    className="text-xs"
                                />
                            </div>

                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Operator Email Address *
                                </label>
                                <div className="relative">
                                    <Input
                                        type="email"
                                        value={email}
                                        onChange={(e) => setEmail(e.target.value)}
                                        placeholder="operator@acme.com"
                                        required
                                        className="text-xs pl-8"
                                    />
                                    <Mail size={13} className="absolute left-2.5 top-3 text-muted-foreground" />
                                </div>
                            </div>

                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Initial Password *
                                </label>
                                <div className="relative">
                                    <Input
                                        type={showPassword ? "text" : "password"}
                                        value={password}
                                        onChange={(e) => setPassword(e.target.value)}
                                        placeholder="Min. 8 characters"
                                        required
                                        className="text-xs pl-8 pr-10"
                                    />
                                    <Lock size={13} className="absolute left-2.5 top-3 text-muted-foreground" />
                                    <button
                                        type="button"
                                        onClick={() => setShowPassword(!showPassword)}
                                        className="absolute inset-y-0 right-0 pr-3 flex items-center text-muted-foreground hover:text-foreground cursor-pointer"
                                    >
                                        {showPassword ? <EyeOff size={14} /> : <Eye size={14} />}
                                    </button>
                                </div>
                            </div>
                        </div>

                        {/* Role selection */}
                        <div className="space-y-1.5 max-w-sm">
                            <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                Role Permission Access
                            </label>
                            <select
                                value={role}
                                onChange={(e) => setRole(e.target.value as 'client_admin' | 'client_viewer')}
                                className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground h-9"
                            >
                                <option value="client_admin">Client Admin (Manage API Keys, Users & Analytics)</option>
                                <option value="client_viewer">Client Viewer (Read-only Telemetry Analytics)</option>
                            </select>
                        </div>

                        <div className="flex justify-end pt-2">
                            <Button
                                type="submit"
                                className="text-xs h-9 gap-1.5 cursor-pointer px-5"
                                isLoading={isSubmitting}
                            >
                                <UserPlus size={14} />
                                Register User in Tenant
                            </Button>
                        </div>
                    </form>
                </CardContent>
            </Card>

            {/* Provisioned Users Table */}
            <Card>
                <CardHeader className="pb-3 border-b border-border-color/20">
                    <CardTitle className="text-base flex items-center gap-2">
                        <Shield className="text-cyan-400 w-4 h-4" />
                        Registered Tenant Users
                    </CardTitle>
                    <CardDescription className="text-xs">
                        Review users provisioned into tenant organizations across your infrastructure.
                    </CardDescription>
                </CardHeader>
                <CardContent className="p-0">
                    {provisionedUsers.length === 0 ? (
                        <div className="text-center py-12 flex flex-col items-center justify-center p-6">
                            <Users className="w-10 h-10 text-muted-foreground/30 mb-2.5" />
                            <p className="text-sm font-semibold text-foreground">No Users Provisioned Yet</p>
                            <p className="text-xs text-muted-foreground max-w-[320px] mt-1">
                                Use the form above to register an administrative or viewer user in a tenant.
                            </p>
                        </div>
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Username</TableHead>
                                    <TableHead>Tenant ID</TableHead>
                                    <TableHead>Email</TableHead>
                                    <TableHead>Role</TableHead>
                                    <TableHead>Status</TableHead>
                                    <TableHead className="text-right">Action</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {provisionedUsers.map((u) => (
                                    <TableRow key={u.id}>
                                        <TableCell className="font-semibold text-sm">
                                            {u.username}
                                        </TableCell>
                                        <TableCell>
                                            <code className="text-xs font-mono bg-zinc-900 text-zinc-300 px-2 py-1 rounded border border-border-color">
                                                {u.clientId.substring(0, 10)}...
                                            </code>
                                        </TableCell>
                                        <TableCell className="text-xs text-muted-foreground">
                                            {u.email || 'N/A'}
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant="outline" className="text-[10px] uppercase font-mono">
                                                {u.role.replace('_', ' ')}
                                            </Badge>
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant={u.isActive ? "success" : "destructive"}>
                                                {u.isActive ? "Active" : "Deactivated"}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="text-right">
                                            {u.isActive && (
                                                <Button
                                                    variant="outline"
                                                    size="sm"
                                                    onClick={() => handleDeactivateUser(u.id)}
                                                    className="border-rose-500/20 text-rose-400 hover:bg-rose-500/10 h-7 text-xs gap-1 cursor-pointer"
                                                >
                                                    <UserMinus size={12} />
                                                    Deactivate
                                                </Button>
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
    );
}
