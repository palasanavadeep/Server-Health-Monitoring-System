"use client";

import { useState, useEffect } from 'react';
import Link from 'next/link';
import { useAuth } from '@/contexts/auth-context';
import { authApi } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { 
    User, 
    Save, 
    Shield, 
    Mail, 
    Lock,
    UserMinus,
    Building,
    KeyRound,
    Eye,
    EyeOff,
    Sliders,
    ArrowRight
} from 'lucide-react';
import { useTenantConfigQuery } from '@/hooks/use-tenant-config';

export default function SettingsPage() {
    const toast = useToast();
    const { user, refreshProfile, logout } = useAuth();

    const { data: tenantConfig, isPending: isTenantConfigLoading } = useTenantConfigQuery(user?.clientId, {
        enabled: !!user?.clientId,
    });

    // Profile updates form fields
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [updatingProfile, setUpdatingProfile] = useState(false);

    useEffect(() => {
        if (user) {
            setUsername(user.username || '');
            setEmail(user.email || '');
        }
    }, [user]);

    const handleUpdateProfile = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!username.trim()) return;

        setUpdatingProfile(true);
        try {
            await authApi.updateProfile({
                username: username.trim(),
                email: email.trim() ? email.trim() : undefined
            });
            toast('Profile updated successfully', 'success');
            await refreshProfile();
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to update profile', 'error');
        } finally {
            setUpdatingProfile(false);
        }
    };

    const handleDeactivateAccount = async () => {
        const targetUserId = user?.id || (user as any)?._id;
        if (!targetUserId) return;
        
        if (!confirm("Are you sure you want to deactivate your account? This will immediately terminate your session.")) {
            return;
        }

        try {
            await authApi.deactivateUser(targetUserId);
            toast('Account deactivated. Logging out...', 'info');
            await logout();
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to deactivate account', 'error');
        }
    };

    return (
        <div className="max-w-4xl mx-auto pb-16 space-y-6 animate-in fade-in duration-150">
            {/* Page Header */}
            <div className="pb-2 border-b border-[#242932]">
                <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                    Settings
                </h1>
                <p className="text-xs text-zinc-400 mt-0.5">
                    Manage your operator profile, active credentials, and workspace configurations.
                </p>
            </div>

            {/* Profile Settings */}
            <div className="surface-panel p-5 space-y-4">
                <div className="pb-3 border-b border-[#242932]">
                    <h2 className="text-sm font-semibold text-zinc-200">Operator Profile</h2>
                    <p className="text-xs text-zinc-400 mt-0.5">Update your display username and communication email address.</p>
                </div>

                <form onSubmit={handleUpdateProfile} className="space-y-4 max-w-md">
                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Username</label>
                        <input
                            type="text"
                            value={username}
                            onChange={(e) => setUsername(e.target.value)}
                            required
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                        />
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block">Email Address</label>
                        <input
                            type="email"
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                            placeholder="operator@company.com"
                            className="w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                        />
                    </div>

                    <div className="pt-2">
                        <Button
                            type="submit"
                            size="sm"
                            isLoading={updatingProfile}
                            className="text-xs h-8 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer"
                        >
                            <Save size={13} className="mr-1.5" />
                            Save Profile
                        </Button>
                    </div>
                </form>
            </div>

            {/* Workspace & Security Metadata */}
            <div className="surface-panel p-5 space-y-4">
                <div className="pb-3 border-b border-[#242932]">
                    <h2 className="text-sm font-semibold text-zinc-200">Security & Workspace Context</h2>
                    <p className="text-xs text-zinc-400 mt-0.5">Active session attributes and assigned role permissions.</p>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 text-xs">
                    <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                        <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Assigned Role</span>
                        <span className="text-sm font-mono font-bold text-zinc-200 mt-1 block">
                            {user?.role?.replace('_', ' ').toUpperCase() || 'USER'}
                        </span>
                    </div>

                    <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                        <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Tenant Workspace ID</span>
                        <span className="text-xs font-mono text-zinc-300 truncate mt-1 block">
                            {user?.clientId || 'Global / Super Admin'}
                        </span>
                    </div>

                    <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                        <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Session Status</span>
                        <span className="text-xs font-semibold text-[#48B982] mt-1 block">
                            ● Authenticated
                        </span>
                    </div>
                </div>
            </div>

            {/* Tenant Metric & Telemetry Settings */}
            <div className="surface-panel p-5 space-y-4">
                <div className="flex items-center justify-between pb-3 border-b border-[#242932]">
                    <div>
                        <h2 className="text-sm font-semibold text-zinc-200">
                            {user?.clientId ? "Workspace Telemetry & Metric Configuration" : "Tenant Telemetry Configurations"}
                        </h2>
                        <p className="text-xs text-zinc-400 mt-0.5">
                            {user?.clientId
                                ? "Active latency calculation thresholds, histogram lenses, and quota caps for your organization."
                                : "Manage per-tenant Apdex thresholds, quantile histogram profiles, and ingest limits."}
                        </p>
                    </div>
                    <Link href="/dashboard/client-config">
                        <Button
                            variant="outline"
                            size="sm"
                            className="h-7 px-2.5 text-xs gap-1.5 cursor-pointer hover:border-[#4CB8D6]"
                        >
                            <Sliders size={12} />
                            Manage Configuration
                            <ArrowRight size={11} className="ml-0.5 text-zinc-400" />
                        </Button>
                    </Link>
                </div>

                {user?.clientId && (
                    isTenantConfigLoading ? (
                        <div className="py-6 text-center text-xs text-zinc-500">Loading telemetry settings...</div>
                    ) : (
                        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                            <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                                <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Apdex Target (T)</span>
                                <span className="text-sm font-mono font-bold text-[#4CB8D6] mt-1 block">
                                    {tenantConfig?.apdexThresholdMs || 500} ms
                                </span>
                            </div>
                            <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                                <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Histogram Lens</span>
                                <span className="text-sm font-semibold text-zinc-200 capitalize mt-1 block">
                                    {tenantConfig?.histogramProfile?.name?.replace('_', ' ') || 'Standard'}
                                </span>
                            </div>
                            <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                                <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Data Retention</span>
                                <span className="text-sm font-mono font-bold text-zinc-200 mt-1 block">
                                    {tenantConfig?.dataRetentionDays || 90} days
                                </span>
                            </div>
                            <div className="p-3 rounded bg-[#0E1014] border border-[#242932]">
                                <span className="text-[10px] uppercase font-semibold text-zinc-400 block">Daily Quota</span>
                                <span className="text-sm font-mono font-bold text-zinc-200 mt-1 block">
                                    {tenantConfig?.dailyIngestQuota ? `${tenantConfig.dailyIngestQuota.toLocaleString()} /day` : 'Unlimited'}
                                </span>
                            </div>
                        </div>
                    )
                )}
            </div>

            {/* Danger Zone */}
            <div className="surface-panel p-5 border-[#E45865]/20 space-y-3">
                <div className="pb-2 border-b border-[#242932]">
                    <h2 className="text-sm font-semibold text-[#E45865]">Deactivate Account</h2>
                    <p className="text-xs text-zinc-400 mt-0.5">Suspending your operator account will immediately terminate this login session.</p>
                </div>

                <div className="flex items-center justify-between pt-1">
                    <span className="text-xs text-zinc-400">Suspend access credentials for this user ID.</span>
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={handleDeactivateAccount}
                        className="text-xs h-8 text-[#E45865] hover:bg-[#E45865]/10 hover:border-[#E45865]/30 cursor-pointer"
                    >
                        <UserMinus size={13} className="mr-1.5" />
                        Deactivate Operator Account
                    </Button>
                </div>
            </div>
        </div>
    );
}
