"use client";

import { useState, useMemo } from 'react';
import { notFound } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useCreateClientUserMutation, useClientUsersQuery } from '@/hooks/use-client-queries';
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
    Mail, 
    Lock, 
    Eye, 
    EyeOff, 
    Shield, 
    Check
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface ClientOperator {
    id: string;
    username: string;
    email?: string;
    role: string;
    isActive: boolean;
    createdAt: string;
}

export default function OperatorsPage() {
    const toast = useToast();
    const { user, loading } = useAuth();

    // Route guard: only client_admin can access operator management
    if (!loading && user && user.role !== 'client_admin') {
        notFound();
    }

    const clientId = user?.clientId || '';
    const { data: serverUsers = [], isLoading: loadingUsers, refetch } = useClientUsersQuery(clientId);
    const createClientUserMutation = useCreateClientUserMutation(clientId);

    // Modal state
    const [isInviteModalOpen, setIsInviteModalOpen] = useState(false);

    // Search and filters
    const [searchQuery, setSearchQuery] = useState('');
    const [roleFilter, setRoleFilter] = useState('ALL');

    // Invite form state
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [role, setRole] = useState<'client_admin' | 'client_viewer'>('client_viewer');
    const [showPassword, setShowPassword] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    const filteredOperators = useMemo(() => {
        return serverUsers.filter(op => {
            const matchesSearch = !searchQuery.trim() || 
                op.username.toLowerCase().includes(searchQuery.toLowerCase()) ||
                (op.email && op.email.toLowerCase().includes(searchQuery.toLowerCase()));
            const matchesRole = roleFilter === 'ALL' || op.role === roleFilter;
            return matchesSearch && matchesRole;
        });
    }, [serverUsers, searchQuery, roleFilter]);

    const handleInviteOperator = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!clientId) {
            toast('No active workspace association found', 'error');
            return;
        }

        if (!username.trim() || !email.trim() || !password.trim()) {
            toast('Please fill in all required fields', 'error');
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

            toast(`Operator '${res.username || username.trim()}' registered successfully!`, 'success');
            setUsername('');
            setEmail('');
            setPassword('');
            setRole('client_viewer');
            setIsInviteModalOpen(false);
            refetch();
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to register operator', 'error');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleToggleUserStatus = async (operatorId: string, currentActive: boolean) => {
        const actionName = currentActive ? 'deactivate' : 'activate';
        if (!confirm(`Are you sure you want to ${actionName} this operator account?`)) {
            return;
        }

        try {
            if (currentActive) {
                await authApi.deactivateUser(operatorId);
                toast('Operator account deactivated', 'info');
            } else {
                await authApi.activateUser(operatorId);
                toast('Operator account activated', 'success');
            }
            refetch();
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || `Failed to ${actionName} operator`, 'error');
        }
    };

    if (loading || (user && user.role !== 'client_admin')) {
        return null;
    }

    return (
        <div className="space-y-6 max-w-7xl mx-auto pb-12 animate-in fade-in duration-150">
            {/* Page Header */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#242932]">
                <div>
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                        Operators
                    </h1>
                    <p className="text-xs text-zinc-400 mt-0.5">
                        Manage team members and configure role-based access for this workspace.
                    </p>
                </div>

                <Button
                    size="sm"
                    onClick={() => setIsInviteModalOpen(true)}
                    className="text-xs h-8 gap-1.5 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold self-start sm:self-auto"
                >
                    <UserPlus size={14} />
                    Invite Operator
                </Button>
            </div>

            {/* Flat Toolbar */}
            <div className="flex flex-wrap items-center justify-between gap-4 text-xs">
                <div className="flex items-center gap-3 flex-1 max-w-md">
                    <div className="relative flex-1">
                        <Search size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                        <input
                            type="text"
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            placeholder="Filter operators by name or email..."
                            className="w-full bg-[#111419] border border-[#242932] rounded px-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                        />
                    </div>

                    <div className="relative">
                        <select
                            value={roleFilter}
                            onChange={(e) => setRoleFilter(e.target.value)}
                            className="bg-[#111419] border border-[#242932] text-xs font-medium text-zinc-300 rounded px-2.5 py-1.5 pr-6 appearance-none outline-none hover:border-[#323946] focus:border-[#4CB8D6] transition-colors cursor-pointer select-none"
                        >
                            <option value="ALL">Role: All</option>
                            <option value="client_admin">Admin</option>
                            <option value="client_viewer">Viewer</option>
                        </select>
                        <ChevronDown size={12} className="absolute right-2 top-2.5 text-zinc-400 pointer-events-none" />
                    </div>
                </div>

                <span className="text-xs text-zinc-500 font-mono">
                    {filteredOperators.length} {filteredOperators.length === 1 ? 'user' : 'users'}
                </span>
            </div>

            {/* Dense Flat Data Table */}
            <div className="surface-panel overflow-hidden">
                {filteredOperators.length === 0 ? (
                    <div className="text-center py-12 p-6">
                        <Users className="w-8 h-8 text-zinc-600 mx-auto mb-2" />
                        <p className="text-xs font-semibold text-zinc-300">No Operators Registered Yet</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mx-auto mt-0.5">
                            {searchQuery ? "No operators matched your search filter." : "Invite team members to collaborate and monitor telemetry streams."}
                        </p>
                        {!searchQuery && (
                            <Button 
                                size="sm" 
                                onClick={() => setIsInviteModalOpen(true)}
                                className="mt-3 text-xs h-7 gap-1 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                            >
                                <UserPlus size={12} />
                                Invite Operator
                            </Button>
                        )}
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-xs border-collapse">
                            <thead>
                                <tr className="border-b border-[#242932] text-zinc-400 text-[11px] uppercase tracking-wider bg-[#0E1014]/60 select-none">
                                    <th className="py-2.5 px-4 font-semibold w-24">Status</th>
                                    <th className="py-2.5 px-4 font-semibold">Operator</th>
                                    <th className="py-2.5 px-4 font-semibold">Email</th>
                                    <th className="py-2.5 px-4 font-semibold">Role</th>
                                    <th className="py-2.5 px-4 font-semibold">Registered</th>
                                    <th className="py-2.5 px-4 font-semibold text-right w-28">Action</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[#242932]">
                                {filteredOperators.map((op, idx) => {
                                    const isCurrentUser = user?.id === op.id;
                                    const isAdmin = user?.role === 'client_admin' || user?.role === 'super_admin';
                                    const canToggle = isAdmin || isCurrentUser;

                                    return (
                                        <tr key={op.id || idx} className="hover:bg-[#181D24] transition-colors">
                                            <td className="py-3 px-4 whitespace-nowrap">
                                                <StatusBadge status={op.isActive ? 'healthy' : 'offline'} label={op.isActive ? 'Active' : 'Disabled'} />
                                            </td>
                                            <td className="py-3 px-4 font-medium text-zinc-200 whitespace-nowrap">
                                                <span>{op.username}</span>
                                                {isCurrentUser && (
                                                    <span className="ml-1.5 text-[9px] font-mono text-[#4CB8D6] bg-[#4CB8D6]/10 px-1 py-0.2 rounded border border-[#4CB8D6]/20">
                                                        You
                                                    </span>
                                                )}
                                            </td>
                                            <td className="py-3 px-4 text-zinc-400 whitespace-nowrap">
                                                {op.email || '—'}
                                            </td>
                                            <td className="py-3 px-4 whitespace-nowrap">
                                                <span className="text-[10px] font-mono uppercase bg-[#181D24] text-zinc-300 px-1.5 py-0.5 rounded border border-[#242932]">
                                                    {op.role === 'client_admin' ? 'Admin' : 'Viewer'}
                                                </span>
                                            </td>
                                            <td className="py-3 px-4 text-zinc-500 whitespace-nowrap font-mono text-[11px]">
                                                {op.createdAt ? new Date(op.createdAt).toLocaleDateString() : '—'}
                                            </td>
                                            <td className="py-3 px-4 text-right whitespace-nowrap">
                                                {canToggle ? (
                                                    <Button
                                                        variant="outline"
                                                        size="sm"
                                                        onClick={() => handleToggleUserStatus(op.id, !!op.isActive)}
                                                        className={cn(
                                                            "h-7 px-2 text-xs cursor-pointer",
                                                            op.isActive
                                                                ? "text-[#E45865] hover:bg-[#E45865]/10 hover:border-[#E45865]/30"
                                                                : "text-[#48B982] hover:bg-[#48B982]/10 hover:border-[#48B982]/30"
                                                        )}
                                                    >
                                                        {op.isActive ? <UserMinus size={12} /> : <UserCheck size={12} />}
                                                        {op.isActive ? 'Deactivate' : 'Activate'}
                                                    </Button>
                                                ) : (
                                                    <span className="text-[10px] text-zinc-600 font-mono">
                                                        {op.isActive ? 'Active' : 'Disabled'}
                                                    </span>
                                                )}
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>

            {/* Invite Operator Modal */}
            <Modal
                isOpen={isInviteModalOpen}
                onClose={() => setIsInviteModalOpen(false)}
                title="Invite Operator"
                description="Create credentials and assign an administrative or read-only role."
                maxWidth="max-w-md"
            >
                <form onSubmit={handleInviteOperator} className="space-y-4">
                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Username *</label>
                        <input
                            type="text"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            placeholder="e.g. alex_dev"
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
                            placeholder="alex@company.com"
                            required
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                        />
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Password *</label>
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
                            <option value="client_viewer">Viewer (Read-only Analytics)</option>
                            <option value="client_admin">Admin (Manage API Keys, Users & Analytics)</option>
                        </select>
                    </div>

                    <div className="flex items-center justify-end gap-2 pt-4 border-t border-[#242932]">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setIsInviteModalOpen(false)}
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
                            Invite Operator
                        </Button>
                    </div>
                </form>
            </Modal>
        </div>
    );
}
