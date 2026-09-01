"use client";

import { useState, useEffect } from 'react';
import { notFound } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useCreateClientMutation } from '@/hooks/use-client-queries';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { useToast } from '@/contexts/toast-context';
import { 
    Building, 
    Plus, 
    Copy, 
    ShieldCheck, 
    Globe, 
    Mail, 
    FileText, 
    Check,
    Users,
    ArrowRight
} from 'lucide-react';
import Link from 'next/link';

interface TenantRecord {
    id: string;
    name: string;
    email?: string;
    website?: string;
    description?: string;
    createdAt: string;
    initialKey?: string;
}

export default function TenantsPage() {
    const toast = useToast();
    const { user, loading } = useAuth();
    const createClientMutation = useCreateClientMutation();

    // Route guard: only super_admin can view this page
    if (!loading && user && user.role !== 'super_admin') {
        notFound();
    }

    // Form inputs
    const [name, setName] = useState('');
    const [email, setEmail] = useState('');
    const [website, setWebsite] = useState('');
    const [description, setDescription] = useState('');

    // Local list of onboarded tenants for the session
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

    const [copiedId, setCopiedId] = useState<string | null>(null);

    const handleCreateTenant = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!name.trim() || !email.trim()) {
            toast('Tenant Name and Contact Email are required', 'error');
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
        <div className="space-y-8 max-w-6xl mx-auto pb-12 animate-in fade-in duration-300">
            {/* Header */}
            <div className="border-b border-border-color/30 pb-5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                        <Building className="text-cyan-400 w-7 h-7" />
                        Tenant Management
                    </h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Register new client tenant organizations and generate isolated telemetry environments.
                    </p>
                </div>
                <Badge variant="outline" className="font-mono text-[10px] uppercase border-cyan-500/30 text-cyan-400 bg-cyan-500/5 px-3 py-1 self-start sm:self-auto">
                    Super Admin Console
                </Badge>
            </div>

            {/* Create Tenant Form Card */}
            <Card className="border-cyan-500/20 bg-glass-card/50 shadow-xl shadow-cyan-500/5">
                <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                        <Plus className="text-cyan-400 w-4 h-4" />
                        Onboard New Tenant Organization
                    </CardTitle>
                    <CardDescription className="text-xs">
                        Fill in tenant details to provision an isolated organization and initial root API access key.
                    </CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleCreateTenant} className="space-y-4">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Organization / Company Name *
                                </label>
                                <Input
                                    value={name}
                                    onChange={(e) => setName(e.target.value)}
                                    placeholder="e.g. Acme Corporation"
                                    required
                                    className="text-xs"
                                />
                            </div>
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Primary Contact Email *
                                </label>
                                <div className="relative">
                                    <Input
                                        type="email"
                                        value={email}
                                        onChange={(e) => setEmail(e.target.value)}
                                        placeholder="admin@acme.com"
                                        required
                                        className="text-xs pl-8"
                                    />
                                    <Mail size={13} className="absolute left-2.5 top-3 text-muted-foreground" />
                                </div>
                            </div>
                        </div>

                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Website URL (Optional)
                                </label>
                                <div className="relative">
                                    <Input
                                        value={website}
                                        onChange={(e) => setWebsite(e.target.value)}
                                        placeholder="https://acme.com"
                                        className="text-xs pl-8"
                                    />
                                    <Globe size={13} className="absolute left-2.5 top-3 text-muted-foreground" />
                                </div>
                            </div>
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Organization Description (Optional)
                                </label>
                                <div className="relative">
                                    <Input
                                        value={description}
                                        onChange={(e) => setDescription(e.target.value)}
                                        placeholder="e.g. Fintech payment processing gateway"
                                        className="text-xs pl-8"
                                    />
                                    <FileText size={13} className="absolute left-2.5 top-3 text-muted-foreground" />
                                </div>
                            </div>
                        </div>

                        <div className="flex justify-end pt-2">
                            <Button 
                                type="submit" 
                                className="text-xs h-9 gap-1.5 cursor-pointer px-5" 
                                isLoading={createClientMutation.isPending}
                            >
                                <Plus size={14} />
                                Register Tenant
                            </Button>
                        </div>
                    </form>
                </CardContent>
            </Card>

            {/* Onboarded Tenants Table */}
            <Card>
                <CardHeader className="pb-3 border-b border-border-color/20 flex flex-row items-center justify-between">
                    <div>
                        <CardTitle className="text-base flex items-center gap-2">
                            <ShieldCheck className="text-emerald-400 w-4 h-4" />
                            Registered Tenants
                        </CardTitle>
                        <CardDescription className="text-xs">
                            Review registered tenant organizations and copy their Client IDs to provision tenant users.
                        </CardDescription>
                    </div>
                    <Link href="/dashboard/tenants/users">
                        <Button variant="secondary" size="sm" className="text-xs h-8 gap-1.5 cursor-pointer">
                            <Users size={13} />
                            Provision Tenant Users
                            <ArrowRight size={12} />
                        </Button>
                    </Link>
                </CardHeader>
                <CardContent className="p-0">
                    {tenants.length === 0 ? (
                        <div className="text-center py-12 flex flex-col items-center justify-center p-6">
                            <Building className="w-10 h-10 text-muted-foreground/30 mb-2.5" />
                            <p className="text-sm font-semibold text-foreground">No Tenants Registered Yet</p>
                            <p className="text-xs text-muted-foreground max-w-[320px] mt-1">
                                Use the form above to register your first tenant organization.
                            </p>
                        </div>
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Organization</TableHead>
                                    <TableHead>Client ID (UUID)</TableHead>
                                    <TableHead>Contact Email</TableHead>
                                    <TableHead>Website</TableHead>
                                    <TableHead>Created</TableHead>
                                    <TableHead className="text-right">Action</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {tenants.map((t) => (
                                    <TableRow key={t.id}>
                                        <TableCell className="font-semibold text-sm">
                                            {t.name}
                                            {t.description && (
                                                <span className="block text-[11px] text-muted-foreground font-normal mt-0.5">
                                                    {t.description}
                                                </span>
                                            )}
                                        </TableCell>
                                        <TableCell>
                                            <code className="text-xs font-mono bg-cyan-500/10 text-cyan-400 px-2 py-1 rounded border border-cyan-500/20">
                                                {t.id}
                                            </code>
                                        </TableCell>
                                        <TableCell className="text-xs text-muted-foreground">
                                            {t.email || 'N/A'}
                                        </TableCell>
                                        <TableCell className="text-xs text-muted-foreground">
                                            {t.website ? (
                                                <a href={t.website} target="_blank" rel="noreferrer" className="text-cyan-400 hover:underline">
                                                    {t.website}
                                                </a>
                                            ) : '—'}
                                        </TableCell>
                                        <TableCell className="text-xs text-muted-foreground">
                                            {new Date(t.createdAt).toLocaleDateString()}
                                        </TableCell>
                                        <TableCell className="text-right">
                                            <div className="flex items-center justify-end gap-2">
                                                <Button
                                                    variant="outline"
                                                    size="sm"
                                                    onClick={() => handleCopy(t.id, t.id)}
                                                    className="h-7 text-xs gap-1 cursor-pointer"
                                                >
                                                    {copiedId === t.id ? <Check size={12} className="text-emerald-400" /> : <Copy size={12} />}
                                                    Copy ID
                                                </Button>
                                                <Link href={`/dashboard/tenants/users?clientId=${t.id}`}>
                                                    <Button
                                                        variant="secondary"
                                                        size="sm"
                                                        className="h-7 text-xs gap-1 cursor-pointer"
                                                    >
                                                        <Users size={12} />
                                                        Add User
                                                    </Button>
                                                </Link>
                                            </div>
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
