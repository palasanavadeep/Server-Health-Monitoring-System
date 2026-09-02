"use client";

import { useState } from 'react';
import { notFound } from 'next/navigation';
import Link from 'next/link';
import { useAuth } from '@/contexts/auth-context';
import { useCreateClientMutation } from '@/hooks/use-client-queries';
import { StatusBadge } from '@/components/ui/status-badge';
import { Modal } from '@/components/ui/modal';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { 
    Building, 
    Plus, 
    Copy, 
    Check, 
    Users, 
    ArrowUpRight,
    Search
} from 'lucide-react';

interface TenantRecord {
    id: string;
    name: string;
    email?: string;
    website?: string;
    description?: string;
    createdAt: string;
}

export default function TenantsPage() {
    const toast = useToast();
    const { user, loading } = useAuth();
    const createClientMutation = useCreateClientMutation();

    // Route guard: only super_admin can view this page
    if (!loading && user && user.role !== 'super_admin') {
        notFound();
    }

    const [isRegisterModalOpen, setIsRegisterModalOpen] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [copiedId, setCopiedId] = useState<string | null>(null);

    // Form inputs
    const [name, setName] = useState('');
    const [email, setEmail] = useState('');
    const [website, setWebsite] = useState('');
    const [description, setDescription] = useState('');

    // Local storage of onboarded tenants
    const [tenants, setTenants] = useState<TenantRecord[]>(() => {
        if (typeof window !== 'undefined') {
            try {
                const saved = localStorage.getItem('sm_onboarded_tenants');
                return saved ? JSON.parse(saved) : [];
            } catch {
                return [];
            }
        }
        return [];
    });

    const filteredTenants = tenants.filter(t => 
        !searchQuery.trim() ||
        t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        t.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (t.email && t.email.toLowerCase().includes(searchQuery.toLowerCase()))
    );

    const handleCreateTenant = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!name.trim() || !email.trim()) {
            toast('Name and email are required', 'error');
            return;
        }

        try {
            const res = await createClientMutation.mutateAsync({
                name: name.trim(),
                email: email.trim(),
                description: description.trim() || undefined,
                website: website.trim() || undefined,
            });

            const newTenant: TenantRecord = {
                id: res.id,
                name: res.name,
                email: res.email,
                website: res.website,
                description: res.description,
                createdAt: res.createdAt || new Date().toISOString(),
            };

            const updated = [newTenant, ...tenants];
            setTenants(updated);
            if (typeof window !== 'undefined') {
                try {
                    localStorage.setItem('sm_onboarded_tenants', JSON.stringify(updated));
                } catch (e) {
                    console.error(e);
                }
            }

            toast(`Tenant organization '${res.name}' created successfully!`, 'success');
            setName('');
            setEmail('');
            setWebsite('');
            setDescription('');
            setIsRegisterModalOpen(false);
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to onboard tenant', 'error');
        }
    };

    const handleCopy = (text: string, id: string) => {
        navigator.clipboard.writeText(text);
        setCopiedId(id);
        toast('Client ID copied to clipboard', 'success');
        setTimeout(() => setCopiedId(null), 2000);
    };

    if (loading || (user && user.role !== 'super_admin')) {
        return null;
    }

    return (
        <div className="space-y-6 max-w-7xl mx-auto pb-12 animate-in fade-in duration-150">
            {/* Page Header */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pb-2 border-b border-[#242932]">
                <div>
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                        Tenants
                    </h1>
                    <p className="text-xs text-zinc-400 mt-0.5">
                        Provision isolated tenant organizations and telemetry environments.
                    </p>
                </div>

                <div className="flex items-center gap-2">
                    <Link href="/dashboard/tenants/users">
                        <Button variant="outline" size="sm" className="text-xs h-8 gap-1.5 cursor-pointer">
                            <Users size={13} />
                            Tenant Users
                        </Button>
                    </Link>
                    <Button
                        size="sm"
                        onClick={() => setIsRegisterModalOpen(true)}
                        className="text-xs h-8 gap-1.5 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                    >
                        <Plus size={14} />
                        Register Tenant
                    </Button>
                </div>
            </div>

            {/* Flat Toolbar */}
            <div className="flex items-center justify-between gap-4 text-xs">
                <div className="relative max-w-sm w-full">
                    <Search size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                    <input
                        type="text"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        placeholder="Search tenants by name or ID..."
                        className="w-full bg-[#111419] border border-[#242932] rounded px-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                    />
                </div>

                <span className="text-xs text-zinc-500 font-mono">
                    {filteredTenants.length} {filteredTenants.length === 1 ? 'tenant' : 'tenants'}
                </span>
            </div>

            {/* Dense Flat Data Table */}
            <div className="surface-panel overflow-hidden">
                {filteredTenants.length === 0 ? (
                    <div className="text-center py-12 p-6">
                        <Building className="w-8 h-8 text-zinc-600 mx-auto mb-2" />
                        <p className="text-xs font-semibold text-zinc-300">No Tenants Registered Yet</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mx-auto mt-0.5">
                            Register a client organization to provision isolated telemetry workspaces.
                        </p>
                        {!searchQuery && (
                            <Button
                                size="sm"
                                onClick={() => setIsRegisterModalOpen(true)}
                                className="mt-3 text-xs h-7 gap-1 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                            >
                                <Plus size={12} />
                                Register Tenant
                            </Button>
                        )}
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-xs border-collapse">
                            <thead>
                                <tr className="border-b border-[#242932] text-zinc-400 text-[11px] uppercase tracking-wider bg-[#0E1014]/60 select-none">
                                    <th className="py-2.5 px-4 font-semibold">Organization</th>
                                    <th className="py-2.5 px-4 font-semibold">Client ID (UUID)</th>
                                    <th className="py-2.5 px-4 font-semibold">Contact Email</th>
                                    <th className="py-2.5 px-4 font-semibold">Registered</th>
                                    <th className="py-2.5 px-4 font-semibold text-right w-36">Actions</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[#242932]">
                                {filteredTenants.map((t) => (
                                    <tr key={t.id} className="hover:bg-[#181D24] transition-colors">
                                        <td className="py-3 px-4 font-medium text-zinc-200 whitespace-nowrap">
                                            <span>{t.name}</span>
                                            {t.description && (
                                                <span className="block text-[11px] text-zinc-500 font-normal">
                                                    {t.description}
                                                </span>
                                            )}
                                        </td>
                                        <td className="py-3 px-4 font-mono text-zinc-400 whitespace-nowrap">
                                            {t.id}
                                        </td>
                                        <td className="py-3 px-4 text-zinc-400 whitespace-nowrap">
                                            {t.email || '—'}
                                        </td>
                                        <td className="py-3 px-4 text-zinc-500 whitespace-nowrap font-mono text-[11px]">
                                            {new Date(t.createdAt).toLocaleDateString()}
                                        </td>
                                        <td className="py-3 px-4 text-right whitespace-nowrap">
                                            <div className="flex items-center justify-end gap-1.5">
                                                <Button
                                                    variant="outline"
                                                    size="sm"
                                                    onClick={() => handleCopy(t.id, t.id)}
                                                    className="h-7 px-2 text-xs gap-1 cursor-pointer"
                                                >
                                                    {copiedId === t.id ? <Check size={12} className="text-[#48B982]" /> : <Copy size={12} />}
                                                    Copy ID
                                                </Button>
                                                <Link href={`/dashboard/tenants/users?clientId=${t.id}`}>
                                                    <Button
                                                        variant="secondary"
                                                        size="sm"
                                                        className="h-7 px-2 text-xs gap-1 cursor-pointer"
                                                    >
                                                        <Users size={12} />
                                                        Users
                                                    </Button>
                                                </Link>
                                            </div>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>

            {/* Register Tenant Modal */}
            <Modal
                isOpen={isRegisterModalOpen}
                onClose={() => setIsRegisterModalOpen(false)}
                title="Register Tenant Organization"
                description="Create an isolated tenant environment and initial root API key."
                maxWidth="max-w-lg"
            >
                <form onSubmit={handleCreateTenant} className="space-y-4">
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                        <div className="space-y-1">
                            <label className="text-xs text-zinc-300 block">Organization Name *</label>
                            <input
                                type="text"
                                value={name}
                                onChange={(e) => setName(e.target.value)}
                                placeholder="e.g. Acme Corp"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs text-zinc-300 block">Contact Email *</label>
                            <input
                                type="email"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                placeholder="admin@acme.com"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                        </div>
                    </div>

                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                        <div className="space-y-1">
                            <label className="text-xs text-zinc-300 block">Website URL (Optional)</label>
                            <input
                                type="text"
                                value={website}
                                onChange={(e) => setWebsite(e.target.value)}
                                placeholder="https://acme.com"
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                        </div>

                        <div className="space-y-1">
                            <label className="text-xs text-zinc-300 block">Description (Optional)</label>
                            <input
                                type="text"
                                value={description}
                                onChange={(e) => setDescription(e.target.value)}
                                placeholder="Fintech cloud gateway"
                                className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                            />
                        </div>
                    </div>

                    <div className="flex items-center justify-end gap-2 pt-4 border-t border-[#242932]">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setIsRegisterModalOpen(false)}
                            className="text-xs h-8 cursor-pointer"
                        >
                            Cancel
                        </Button>
                        <Button
                            type="submit"
                            size="sm"
                            isLoading={createClientMutation.isPending}
                            className="text-xs h-8 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer"
                        >
                            Register Tenant
                        </Button>
                    </div>
                </form>
            </Modal>
        </div>
    );
}
