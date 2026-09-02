"use client";

import { useState, useEffect, useMemo } from 'react';
import { notFound, useSearchParams } from 'next/navigation';
import Link from 'next/link';
import { useAuth } from '@/contexts/auth-context';
import { useCreateClientUserMutation } from '@/hooks/use-client-queries';
import { authApi } from '@/lib/api';
import { StatusBadge } from '@/components/ui/status-badge';
import { Modal } from '@/components/ui/modal';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { 
    Users, 
    UserPlus, 
    Search, 
    ChevronDown, 
    UserMinus, 
    UserCheck,
    Building, 
    ArrowLeft,
    Eye,
    EyeOff
} from 'lucide-react';

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

    const [isProvisionModalOpen, setIsProvisionModalOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [targetClientId, setTargetClientId] = useState('');
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [role, setRole] = useState<'client_admin' | 'client_viewer'>('client_admin');
    const [showPassword, setShowPassword] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    // Saved tenants list
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
            setIsProvisionModalOpen(true);
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

    const filteredUsers = useMemo(() => {
        if (!searchQuery.trim()) return provisionedUsers;
        const q = searchQuery.toLowerCase();
        return provisionedUsers.filter(u => 
            u.username.toLowerCase().includes(q) ||
            u.clientId.toLowerCase().includes(q) ||
            (u.email && u.email.toLowerCase().includes(q))
        );
    }, [provisionedUsers, searchQuery]);

    const handleCreateUser = async (e: React.FormEvent) => {
        e.preventDefault();
        const cid = targetClientId.trim();
        if (!cid) {
            toast('Please enter or select a Tenant Client ID', 'error');
            return;
        }

        if (!username.trim() || !email.trim() || !password.trim()) {
            toast('All fields are required', 'error');
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

            toast(`User '${newUser.username}' provisioned in tenant!`, 'success');
            setUsername('');
            setEmail('');
            setPassword('');
            setIsProvisionModalOpen(false);
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to provision tenant user', 'error');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleToggleUserStatus = async (userId: string, currentActive: boolean) => {
        const actionName = currentActive ? 'deactivate' : 'activate';
        if (!confirm(`Are you sure you want to ${actionName} this tenant user account?`)) {
            return;
        }

        try {
            if (currentActive) {
                await authApi.deactivateUser(userId);
                toast('User deactivated successfully', 'info');
            } else {
                await authApi.activateUser(userId);
                toast('User activated successfully', 'success');
            }
            const updated = provisionedUsers.map(u => u.id === userId ? { ...u, isActive: !currentActive } : u);
            setProvisionedUsers(updated);
            if (typeof window !== 'undefined') {
                try {
                    localStorage.setItem('sm_provisioned_tenant_users', JSON.stringify(updated));
                } catch (e) {
                    console.error(e);
                }
            }
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || `Failed to ${actionName} user`, 'error');
        }
    };

    if (loading || (user && user.role !== 'super_admin')) {
        return null;
    }

    return (
        <div className="space-y-6 max-w-7xl mx-auto pb-12 animate-in fade-in duration-150">
            {/* Page Header */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#242932]">
                <div>
                    <div className="flex items-center gap-1.5 mb-1">
                        <Link href="/dashboard/tenants" className="text-xs text-zinc-400 hover:text-[#4CB8D6] flex items-center gap-1">
                            <ArrowLeft size={12} />
                            Tenants
                        </Link>
                    </div>
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                        Tenant Users
                    </h1>
                    <p className="text-xs text-zinc-400 mt-0.5">
                        Provision administrative and viewer user credentials across tenant organizations.
                    </p>
                </div>

                <Button
                    size="sm"
                    onClick={() => setIsProvisionModalOpen(true)}
                    className="text-xs h-8 gap-1.5 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold self-start sm:self-auto"
                >
                    <UserPlus size={14} />
                    Provision User
                </Button>
            </div>

            {/* Flat Toolbar */}
            <div className="flex items-center justify-between gap-4 text-xs">
                <div className="relative max-w-sm w-full">
                    <Search size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                    <input
                        type="text"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        placeholder="Search users by name, email, or tenant ID..."
                        className="w-full bg-[#111419] border border-[#242932] rounded px-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                    />
                </div>

                <span className="text-xs text-zinc-500 font-mono">
                    {filteredUsers.length} {filteredUsers.length === 1 ? 'user' : 'users'}
                </span>
            </div>

            {/* Dense Flat Data Table */}
            <div className="surface-panel overflow-hidden">
                {filteredUsers.length === 0 ? (
                    <div className="text-center py-12 p-6">
                        <Users className="w-8 h-8 text-zinc-600 mx-auto mb-2" />
                        <p className="text-xs font-semibold text-zinc-300">No Users Provisioned Yet</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mx-auto mt-0.5">
                            Provision an operator or admin user inside any registered client tenant.
                        </p>
                        {!searchQuery && (
                            <Button
                                size="sm"
                                onClick={() => setIsProvisionModalOpen(true)}
                                className="mt-3 text-xs h-7 gap-1 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                            >
                                <UserPlus size={12} />
                                Provision User
                            </Button>
                        )}
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-xs border-collapse">
                            <thead>
                                <tr className="border-b border-[#242932] text-zinc-400 text-[11px] uppercase tracking-wider bg-[#0E1014]/60 select-none">
                                    <th className="py-2.5 px-4 font-semibold w-24">Status</th>
                                    <th className="py-2.5 px-4 font-semibold">User</th>
                                    <th className="py-2.5 px-4 font-semibold">Tenant Client ID</th>
                                    <th className="py-2.5 px-4 font-semibold">Email</th>
                                    <th className="py-2.5 px-4 font-semibold">Role</th>
                                    <th className="py-2.5 px-4 font-semibold text-right w-28">Action</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[#242932]">
                                {filteredUsers.map((u) => (
                                    <tr key={u.id} className="hover:bg-[#181D24] transition-colors">
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <StatusBadge status={u.isActive ? 'healthy' : 'offline'} label={u.isActive ? 'Active' : 'Disabled'} />
                                        </td>
                                        <td className="py-3 px-4 font-medium text-zinc-200 whitespace-nowrap">
                                            {u.username}
                                        </td>
                                        <td className="py-3 px-4 font-mono text-zinc-400 whitespace-nowrap">
                                            {u.clientId.substring(0, 10)}...
                                        </td>
                                        <td className="py-3 px-4 text-zinc-400 whitespace-nowrap">
                                            {u.email || '—'}
                                        </td>
                                        <td className="py-3 px-4 whitespace-nowrap">
                                            <span className="text-[10px] font-mono uppercase bg-[#181D24] text-zinc-300 px-1.5 py-0.5 rounded border border-[#242932]">
                                                {u.role.replace('_', ' ')}
                                            </span>
                                        </td>
                                        <td className="py-3 px-4 text-right whitespace-nowrap">
                                            <Button
                                                variant="outline"
                                                size="sm"
                                                onClick={() => handleToggleUserStatus(u.id, !!u.isActive)}
                                                className={`h-7 px-2 text-xs cursor-pointer ${
                                                    u.isActive
                                                        ? "text-[#E45865] hover:bg-[#E45865]/10 hover:border-[#E45865]/30"
                                                        : "text-[#48B982] hover:bg-[#48B982]/10 hover:border-[#48B982]/30"
                                                }`}
                                            >
                                                {u.isActive ? <UserMinus size={12} /> : <UserCheck size={12} />}
                                                {u.isActive ? 'Deactivate' : 'Activate'}
                                            </Button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>

            {/* Provision User Modal */}
            <Modal
                isOpen={isProvisionModalOpen}
                onClose={() => setIsProvisionModalOpen(false)}
                title="Provision Tenant User"
                description="Assign credentials and access permissions to a tenant organization."
                maxWidth="max-w-md"
            >
                <form onSubmit={handleCreateUser} className="space-y-4">
                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Target Client ID *</label>
                        <input
                            type="text"
                            value={targetClientId}
                            onChange={(e) => setTargetClientId(e.target.value)}
                            placeholder="Enter tenant UUID"
                            required
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 font-mono focus:outline-none focus:border-[#4CB8D6]"
                        />
                    </div>

                    {savedTenants.length > 0 && (
                        <div className="space-y-1">
                            <label className="text-[11px] text-zinc-400 block">Or select from registered tenants</label>
                            <select
                                value={targetClientId}
                                onChange={(e) => setTargetClientId(e.target.value)}
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            >
                                <option value="">-- Select Tenant --</option>
                                {savedTenants.map((t) => (
                                    <option key={t.id} value={t.id}>{t.name} ({t.id.substring(0, 8)}...)</option>
                                ))}
                            </select>
                        </div>
                    )}

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Username *</label>
                        <input
                            type="text"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            placeholder="e.g. acme_admin"
                            required
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                        />
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Email Address *</label>
                        <input
                            type="email"
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                            placeholder="admin@acme.com"
                            required
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                        />
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Initial Password *</label>
                        <div className="relative">
                            <input
                                type={showPassword ? "text" : "password"}
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                placeholder="Min. 8 characters"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 pr-8 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                            <button
                                type="button"
                                onClick={() => setShowPassword(!showPassword)}
                                className="absolute right-2.5 top-2 text-zinc-500 hover:text-zinc-300 cursor-pointer"
                            >
                                {showPassword ? <EyeOff size={13} /> : <Eye size={13} />}
                            </button>
                        </div>
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Role Access</label>
                        <select
                            value={role}
                            onChange={(e: any) => setRole(e.target.value)}
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                        >
                            <option value="client_admin">Client Admin</option>
                            <option value="client_viewer">Client Viewer</option>
                        </select>
                    </div>

                    <div className="flex items-center justify-end gap-2 pt-4 border-t border-[#242932]">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setIsProvisionModalOpen(false)}
                            className="text-xs h-8 cursor-pointer"
                        >
                            Cancel
                        </Button>
                        <Button
                            type="submit"
                            size="sm"
                            isLoading={isSubmitting}
                            className="text-xs h-8 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer"
                        >
                            Provision User
                        </Button>
                    </div>
                </form>
            </Modal>
        </div>
    );
}
