"use client";

import { useState, useEffect } from 'react';
import { notFound } from 'next/navigation';
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
    Mail, 
    Lock, 
    Eye, 
    EyeOff, 
    Shield, 
    UserMinus, 
    Building, 
    UserCheck,
    CheckCircle2
} from 'lucide-react';

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
    const createClientUserMutation = useCreateClientUserMutation(clientId);

    // Form inputs
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [role, setRole] = useState<'client_admin' | 'client_viewer'>('client_viewer');
    const [showPassword, setShowPassword] = useState(false);
    const [isSubmitting, setIsSubmitting] = useState(false);

    // Local storage of registered operators for this client
    const [operators, setOperators] = useState<ClientOperator[]>(() => {
        if (typeof window !== 'undefined') {
            try {
                const storageKey = `sm_operators_${clientId || 'default'}`;
                const saved = localStorage.getItem(storageKey);
                return saved ? JSON.parse(saved) : [];
            } catch {
                return [];
            }
        }
        return [];
    });

    // Update storage key when clientId loads
    useEffect(() => {
        if (clientId && typeof window !== 'undefined') {
            try {
                const storageKey = `sm_operators_${clientId}`;
                const saved = localStorage.getItem(storageKey);
                if (saved) {
                    setOperators(JSON.parse(saved));
                }
            } catch (e) {
                console.error(e);
            }
        }
    }, [clientId]);

    const handleOnboardOperator = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!clientId) {
            toast('No active client association found for this account', 'error');
            return;
        }

        if (!username.trim() || !email.trim() || !password.trim()) {
            toast('Username, email, and password are required', 'error');
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

            const newOperator: ClientOperator = {
                id: res.id,
                username: res.username || username.trim(),
                email: res.email || email.trim(),
                role: res.role || role,
                isActive: true,
                createdAt: new Date().toISOString(),
            };

            const updated = [newOperator, ...operators];
            setOperators(updated);
            if (typeof window !== 'undefined' && clientId) {
                try {
                    localStorage.setItem(`sm_operators_${clientId}`, JSON.stringify(updated));
                } catch (e) {
                    console.error(e);
                }
            }

            toast(`Operator '${newOperator.username}' onboarded successfully!`, 'success');
            setUsername('');
            setEmail('');
            setPassword('');
            setRole('client_viewer');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to onboard operator user', 'error');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleDeactivateOperator = async (operatorId: string) => {
        if (!confirm('Are you sure you want to deactivate this operator user?')) {
            return;
        }

        try {
            await authApi.deactivateUser(operatorId);
            const updated = operators.map(op => op.id === operatorId ? { ...op, isActive: false } : op);
            setOperators(updated);
            if (typeof window !== 'undefined' && clientId) {
                try {
                    localStorage.setItem(`sm_operators_${clientId}`, JSON.stringify(updated));
                } catch (e) {
                    console.error(e);
                }
            }
            toast('Operator account deactivated successfully', 'info');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to deactivate operator user', 'error');
        }
    };

    if (loading || (user && (user.role === 'super_admin' || user.role === 'client_viewer'))) {
        return null;
    }

    return (
        <div className="space-y-8 max-w-6xl mx-auto pb-12 animate-in fade-in duration-300">
            {/* Header */}
            <div className="border-b border-border-color/30 pb-5 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                        <Users className="text-cyan-400 w-7 h-7" />
                        Operator User Management
                    </h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Register and provision operator accounts for your organization to access monitoring streams.
                    </p>
                </div>
                <div className="flex items-center gap-2">
                    <Badge variant="outline" className="font-mono text-[10px] uppercase border-border-color bg-zinc-950/20 text-muted-foreground">
                        Tenant: {user?.clientId || 'N/A'}
                    </Badge>
                </div>
            </div>

            {/* Onboard Operator User Form */}
            <Card className="border-cyan-500/20 bg-glass-card/50 shadow-xl shadow-cyan-500/5">
                <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                        <UserPlus className="text-cyan-400 w-4 h-4" />
                        Register New Operator User
                    </CardTitle>
                    <CardDescription className="text-xs">
                        Configure user login credentials and assign administrative or read-only roles.
                    </CardDescription>
                </CardHeader>
                <CardContent>
                    <form onSubmit={handleOnboardOperator} className="space-y-4 max-w-2xl">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Username *
                                </label>
                                <Input
                                    value={username}
                                    onChange={(e) => setUsername(e.target.value)}
                                    placeholder="e.g. dev_operator"
                                    required
                                    className="text-xs"
                                />
                            </div>

                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Email Address *
                                </label>
                                <div className="relative">
                                    <Input
                                        type="email"
                                        value={email}
                                        onChange={(e) => setEmail(e.target.value)}
                                        placeholder="operator@company.com"
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
                                    Default Password *
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

                            <div className="space-y-1.5">
                                <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">
                                    Operator Role Access
                                </label>
                                <select
                                    value={role}
                                    onChange={(e) => setRole(e.target.value as 'client_admin' | 'client_viewer')}
                                    className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2.5 text-foreground h-9"
                                >
                                    <option value="client_viewer">Client Viewer (Read-only Analytics)</option>
                                    <option value="client_admin">Client Admin (Manage API Keys, Users & Analytics)</option>
                                </select>
                            </div>
                        </div>

                        <div className="flex justify-end pt-2">
                            <Button 
                                type="submit" 
                                className="text-xs h-9 gap-1.5 cursor-pointer px-5" 
                                isLoading={isSubmitting}
                            >
                                <UserPlus size={14} />
                                Register Operator
                            </Button>
                        </div>
                    </form>
                </CardContent>
            </Card>

            {/* Registered Operators List Table */}
            <Card>
                <CardHeader className="pb-3 border-b border-border-color/20">
                    <CardTitle className="text-base flex items-center gap-2">
                        <UserCheck className="text-emerald-400 w-4 h-4" />
                        Registered Operators
                    </CardTitle>
                    <CardDescription className="text-xs">
                        Review active operators with credentials to log into this tenant organization.
                    </CardDescription>
                </CardHeader>
                <CardContent className="p-0">
                    {operators.length === 0 ? (
                        <div className="text-center py-12 flex flex-col items-center justify-center p-6">
                            <Users className="w-10 h-10 text-muted-foreground/30 mb-2.5" />
                            <p className="text-sm font-semibold text-foreground">No Operators Registered Yet</p>
                            <p className="text-xs text-muted-foreground max-w-[300px] mt-1">
                                Use the form above to onboard team members and assign operator roles.
                            </p>
                        </div>
                    ) : (
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHead>Username</TableHead>
                                    <TableHead>Email Address</TableHead>
                                    <TableHead>Access Role</TableHead>
                                    <TableHead>Registered</TableHead>
                                    <TableHead>Status</TableHead>
                                    <TableHead className="text-right">Action</TableHead>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {operators.map((op) => (
                                    <TableRow key={op.id}>
                                        <TableCell className="font-semibold text-sm">
                                            {op.username}
                                        </TableCell>
                                        <TableCell className="text-xs text-muted-foreground">
                                            {op.email || 'N/A'}
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant="outline" className="text-[10px] uppercase font-mono">
                                                {op.role.replace('_', ' ')}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="text-xs text-muted-foreground">
                                            {new Date(op.createdAt).toLocaleDateString()}
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant={op.isActive ? "success" : "destructive"}>
                                                {op.isActive ? "Active" : "Deactivated"}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="text-right">
                                            {op.isActive && (
                                                <Button
                                                    variant="outline"
                                                    size="sm"
                                                    onClick={() => handleDeactivateOperator(op.id)}
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
