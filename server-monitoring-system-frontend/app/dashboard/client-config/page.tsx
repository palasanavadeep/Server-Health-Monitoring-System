"use client";

import React, { useState, useEffect, useMemo } from 'react';
import { useSearchParams, useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useToast } from '@/contexts/toast-context';
import { 
    useTenantConfigQuery, 
    useUpdateTenantConfigMutation, 
    useHistogramProfilesQuery 
} from '@/hooks/use-tenant-config';
import { useClientsQuery } from '@/hooks/use-client-queries';
import { Button } from '@/components/ui/button';
import { 
    Gauge, 
    Clock, 
    Database, 
    Sliders, 
    Layers, 
    ShieldCheck, 
    ShieldAlert,
    Loader2, 
    Check, 
    Info,
    Building,
    Save,
    RotateCcw,
    Lock,
    AlertCircle,
    ChevronDown
} from 'lucide-react';
import { cn } from '@/lib/utils';

export default function ClientConfigPage() {
    const toast = useToast();
    const router = useRouter();
    const searchParams = useSearchParams();
    const { user, loading: authLoading } = useAuth();

    const isSuperAdmin = user?.role === 'super_admin';
    const isClientAdmin = user?.role === 'client_admin';
    const isClientViewer = user?.role === 'client_viewer';
    const canEdit = isSuperAdmin || isClientAdmin;

    // For super admins: load all client organizations to allow tenant switching
    const { data: clients = [], isPending: isLoadingClients } = useClientsQuery(isSuperAdmin);

    // Determine currently selected client ID
    const paramClientId = searchParams.get('clientId');
    const [selectedClientId, setSelectedClientId] = useState<string>('');

    useEffect(() => {
        if (isSuperAdmin) {
            if (paramClientId) {
                setSelectedClientId(paramClientId);
            } else if (clients.length > 0 && !selectedClientId) {
                setSelectedClientId(clients[0].id);
            }
        } else if (user?.clientId) {
            setSelectedClientId(user.clientId);
        }
    }, [isSuperAdmin, paramClientId, clients, user?.clientId, selectedClientId]);

    const handleClientChange = (newClientId: string) => {
        setSelectedClientId(newClientId);
        router.push(`/dashboard/client-config?clientId=${newClientId}`);
    };

    const effectiveClientId = isSuperAdmin ? selectedClientId : (user?.clientId || '');

    // Active selected client metadata (for super admin)
    const activeClient = useMemo(() => {
        if (isSuperAdmin && clients.length > 0) {
            return clients.find(c => c.id === effectiveClientId) || null;
        }
        return null;
    }, [isSuperAdmin, clients, effectiveClientId]);

    // Query telemetry configuration and histogram profiles
    const { 
        data: config, 
        isPending: isLoadingConfig, 
        refetch: refetchConfig 
    } = useTenantConfigQuery(effectiveClientId, {
        enabled: !!effectiveClientId,
    });

    const { data: profiles = [], isPending: isLoadingProfiles } = useHistogramProfilesQuery();
    const updateMutation = useUpdateTenantConfigMutation();

    // Local form state
    const [apdexThresholdMs, setApdexThresholdMs] = useState(500);
    const [selectedProfile, setSelectedProfile] = useState('standard');
    const [dataRetentionDays, setDataRetentionDays] = useState(90);
    const [isQuotaUnlimited, setIsQuotaUnlimited] = useState(true);
    const [dailyIngestQuota, setDailyIngestQuota] = useState<number>(100_000);

    // Sync form state when config loads or target client changes
    useEffect(() => {
        if (config) {
            setApdexThresholdMs(config.apdexThresholdMs || 500);
            setSelectedProfile(config.histogramProfile?.name || 'standard');
            setDataRetentionDays(config.dataRetentionDays || 90);
            if (config.dailyIngestQuota !== null && config.dailyIngestQuota !== undefined) {
                setIsQuotaUnlimited(false);
                setDailyIngestQuota(config.dailyIngestQuota);
            } else {
                setIsQuotaUnlimited(true);
                setDailyIngestQuota(100_000);
            }
        }
    }, [config]);

    // Check if user has uncommitted changes
    const hasChanges = useMemo(() => {
        if (!config) return false;
        const currentQuota = isQuotaUnlimited ? null : dailyIngestQuota;
        const serverQuota = config.dailyIngestQuota ?? null;

        return (
            apdexThresholdMs !== (config.apdexThresholdMs || 500) ||
            selectedProfile !== (config.histogramProfile?.name || 'standard') ||
            dataRetentionDays !== (config.dataRetentionDays || 90) ||
            currentQuota !== serverQuota
        );
    }, [config, apdexThresholdMs, selectedProfile, dataRetentionDays, isQuotaUnlimited, dailyIngestQuota]);

    const handleReset = () => {
        if (config) {
            setApdexThresholdMs(config.apdexThresholdMs || 500);
            setSelectedProfile(config.histogramProfile?.name || 'standard');
            setDataRetentionDays(config.dataRetentionDays || 90);
            if (config.dailyIngestQuota !== null && config.dailyIngestQuota !== undefined) {
                setIsQuotaUnlimited(false);
                setDailyIngestQuota(config.dailyIngestQuota);
            } else {
                setIsQuotaUnlimited(true);
                setDailyIngestQuota(100_000);
            }
            toast('Configuration changes reverted', 'info');
        }
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        if (!canEdit) {
            toast('Permission denied: Client viewers cannot update configuration', 'error');
            return;
        }

        if (!effectiveClientId) {
            toast('No tenant organization selected', 'error');
            return;
        }

        try {
            await updateMutation.mutateAsync({
                clientId: effectiveClientId,
                data: {
                    apdexThresholdMs,
                    histogramProfile: selectedProfile,
                    dataRetentionDays,
                    dailyIngestQuota: isQuotaUnlimited ? null : dailyIngestQuota,
                },
            });
            toast('Telemetry configuration updated successfully', 'success');
            await refetchConfig();
        } catch (err: any) {
            toast(
                err.response?.data?.message || err.message || 'Failed to update configuration',
                'error'
            );
        }
    };

    if (authLoading) {
        return (
            <div className="py-24 text-center text-zinc-400 space-y-3">
                <Loader2 className="w-7 h-7 animate-spin mx-auto text-[#4CB8D6]" />
                <p className="text-xs">Authenticating operator session...</p>
            </div>
        );
    }

    return (
        <div className="max-w-5xl mx-auto pb-16 space-y-6 animate-in fade-in duration-150">
            {/* Page Header & Breadcrumbs */}
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 pb-3 border-b border-[#242932]">
                <div>
                    <div className="flex items-center gap-2">
                        <Sliders className="w-5 h-5 text-[#4CB8D6]" />
                        <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                            Client Telemetry Configuration
                        </h1>
                    </div>
                    <p className="text-xs text-zinc-400 mt-1">
                        Fine-tune per-tenant metric interpretation, Apdex T latency satisfaction thresholds, quantile histogram profiles, data retention, and ingest limits.
                    </p>
                </div>

                {/* Super Admin Tenant Selector */}
                {isSuperAdmin && (
                    <div className="flex items-center gap-2">
                        <Building className="w-4 h-4 text-zinc-400" />
                        <div className="relative">
                            <select
                                value={selectedClientId}
                                onChange={(e) => handleClientChange(e.target.value)}
                                disabled={isLoadingClients}
                                className="bg-[#0E1014] border border-[#242932] rounded-lg px-3 py-1.5 text-xs text-zinc-200 focus:outline-none focus:border-[#4CB8D6] pr-8 appearance-none cursor-pointer"
                            >
                                {clients.map((c) => (
                                    <option key={c.id} value={c.id}>
                                        {c.name} ({c.id.substring(0, 8)}...)
                                    </option>
                                ))}
                            </select>
                            <ChevronDown className="w-3.5 h-3.5 text-zinc-400 absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
                        </div>
                    </div>
                )}
            </div>

            {/* RBAC Status Notice */}
            {isClientViewer ? (
                <div className="p-4 rounded-lg bg-[#D99A3D]/10 border border-[#D99A3D]/30 flex items-start gap-3 text-xs">
                    <ShieldAlert className="w-5 h-5 text-[#D99A3D] shrink-0 mt-0.5" />
                    <div>
                        <h4 className="font-semibold text-[#D99A3D]">Read-Only Mode (Client Viewer)</h4>
                        <p className="text-zinc-300 mt-0.5 leading-relaxed">
                            You have read-only permissions for this client workspace. You can inspect active telemetry configurations, but modifications are restricted to <strong>Client Administrators</strong> and <strong>Super Administrators</strong>.
                        </p>
                    </div>
                </div>
            ) : isClientAdmin ? (
                <div className="p-3.5 rounded-lg bg-[#48B982]/10 border border-[#48B982]/30 flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2.5">
                        <ShieldCheck className="w-4 h-4 text-[#48B982] shrink-0" />
                        <span className="text-zinc-200">
                            Logged in as <strong>Client Administrator</strong>. You have authorization to update configuration for your organization.
                        </span>
                    </div>
                    <span className="font-mono text-[11px] text-zinc-400 hidden sm:inline">
                        Client ID: {user?.clientId}
                    </span>
                </div>
            ) : isSuperAdmin && activeClient ? (
                <div className="p-3.5 rounded-lg bg-[#4CB8D6]/10 border border-[#4CB8D6]/30 flex items-center justify-between text-xs">
                    <div className="flex items-center gap-2.5">
                        <Building className="w-4 h-4 text-[#4CB8D6] shrink-0" />
                        <span className="text-zinc-200">
                            Super Admin mode: configuring organization <strong>{activeClient.name}</strong> ({activeClient.email || 'No email'})
                        </span>
                    </div>
                    <span className="font-mono text-[11px] text-zinc-400 hidden sm:inline">
                        ID: {activeClient.id}
                    </span>
                </div>
            ) : null}

            {isLoadingConfig ? (
                <div className="py-20 text-center text-zinc-400 space-y-3 surface-panel p-8">
                    <Loader2 className="w-7 h-7 animate-spin mx-auto text-[#4CB8D6]" />
                    <p className="text-xs">Loading client telemetry settings...</p>
                </div>
            ) : !effectiveClientId ? (
                <div className="py-16 text-center text-zinc-400 space-y-3 surface-panel p-8">
                    <AlertCircle className="w-8 h-8 mx-auto text-zinc-500" />
                    <p className="text-xs">No client organization selected or available.</p>
                </div>
            ) : (
                <form onSubmit={handleSubmit} className="space-y-6">
                    {/* Apdex T Latency Target */}
                    <div className="surface-panel p-5 space-y-3">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <Gauge className="w-4 h-4 text-[#4CB8D6]" />
                                <h2 className="text-sm font-semibold text-zinc-200 uppercase tracking-wide">
                                    Apdex T Target (Latency Threshold)
                                </h2>
                            </div>
                            <span className="font-mono text-sm font-bold text-[#4CB8D6] bg-[#4CB8D6]/10 px-2.5 py-0.5 rounded border border-[#4CB8D6]/30">
                                {apdexThresholdMs} ms
                            </span>
                        </div>

                        <p className="text-xs text-zinc-400 leading-relaxed max-w-3xl">
                            The Apdex (Application Performance Index) metric standardizes user satisfaction. Requests completing in ≤ <strong className="text-zinc-200">{apdexThresholdMs}ms</strong> are categorized as <em>Satisfied</em>. Responses between <strong className="text-zinc-200">{apdexThresholdMs}ms</strong> and <strong className="text-zinc-200">{apdexThresholdMs * 4}ms</strong> (4T) are <em>Tolerating</em>. Any response taking longer than <strong className="text-zinc-200">{apdexThresholdMs * 4}ms</strong> or failing with a 5xx error is categorized as <em>Frustrated</em>.
                        </p>

                        <div className="flex items-center gap-4 pt-2">
                            <input
                                type="range"
                                min={50}
                                max={3000}
                                step={50}
                                value={apdexThresholdMs}
                                onChange={(e) => setApdexThresholdMs(Number(e.target.value))}
                                disabled={!canEdit}
                                className={cn(
                                    "w-full accent-[#4CB8D6]",
                                    canEdit ? "cursor-pointer" : "cursor-not-allowed opacity-60"
                                )}
                            />
                            <div className="flex items-center gap-1.5 shrink-0">
                                <input
                                    type="number"
                                    min={10}
                                    max={10000}
                                    value={apdexThresholdMs}
                                    onChange={(e) => setApdexThresholdMs(Math.max(10, Number(e.target.value)))}
                                    disabled={!canEdit}
                                    className={cn(
                                        "w-24 bg-[#0E1014] border border-[#242932] rounded px-2.5 py-1.5 text-xs font-mono text-zinc-200 text-right focus:outline-none focus:border-[#4CB8D6]",
                                        !canEdit && "cursor-not-allowed opacity-60"
                                    )}
                                />
                                <span className="text-xs text-zinc-500 font-mono">ms</span>
                            </div>
                        </div>

                        {/* Quick presets */}
                        <div className="flex items-center gap-2 pt-1">
                            <span className="text-[11px] text-zinc-500 font-medium">Quick Presets:</span>
                            {[200, 500, 1000, 1500, 2000].map((preset) => (
                                <button
                                    key={preset}
                                    type="button"
                                    onClick={() => canEdit && setApdexThresholdMs(preset)}
                                    disabled={!canEdit}
                                    className={cn(
                                        "px-2.5 py-1 rounded text-xs font-mono border transition-colors",
                                        apdexThresholdMs === preset
                                            ? "bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/40 font-semibold"
                                            : "bg-[#0E1014] text-zinc-400 border-[#242932] hover:text-zinc-200",
                                        !canEdit && "cursor-not-allowed opacity-60"
                                    )}
                                >
                                    {preset}ms
                                </button>
                            ))}
                        </div>
                    </div>

                    {/* Histogram Quantile Profiles */}
                    <div className="surface-panel p-5 space-y-3">
                        <div className="flex items-center gap-2">
                            <Layers className="w-4 h-4 text-[#48B982]" />
                            <h2 className="text-sm font-semibold text-zinc-200 uppercase tracking-wide">
                                Histogram Percentile Profile (Quantile Lens)
                            </h2>
                        </div>

                        <p className="text-xs text-zinc-400 leading-relaxed max-w-3xl">
                            Select the interpolation profile used to calculate p50, p90, p95, and p99 response times. Standard profiling retains 23 cumulative duration boundaries to ensure zero client-side latency calculation drift.
                        </p>

                        <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2">
                            {profiles.length > 0 ? (
                                profiles.map((p) => {
                                    const isSelected = selectedProfile === p.name;
                                    return (
                                        <div
                                            key={p.name}
                                            onClick={() => canEdit && setSelectedProfile(p.name)}
                                            className={cn(
                                                "p-4 rounded-lg border transition-all relative",
                                                isSelected
                                                    ? "bg-[#48B982]/10 border-[#48B982]/60 shadow-sm"
                                                    : "bg-[#0E1014] border-[#242932] hover:border-zinc-700",
                                                canEdit ? "cursor-pointer" : "cursor-not-allowed opacity-75"
                                            )}
                                        >
                                            <div className="flex items-center justify-between mb-1.5">
                                                <span className="text-xs font-semibold text-zinc-200 capitalize">
                                                    {p.name.replace('_', ' ')}
                                                </span>
                                                {isSelected && (
                                                    <span className="flex items-center gap-1 text-[11px] text-[#48B982] font-semibold">
                                                        <Check size={13} /> Active
                                                    </span>
                                                )}
                                            </div>
                                            <p className="text-[11px] text-zinc-400 leading-relaxed">
                                                {p.description}
                                            </p>
                                            <div className="mt-3 text-[10px] font-mono text-zinc-500 border-t border-[#242932] pt-2">
                                                Resolution: {p.bucketBounds?.length || 0} discrete bounds
                                            </div>
                                        </div>
                                    );
                                })
                            ) : (
                                <div className="p-4 rounded bg-[#0E1014] border border-[#242932] text-xs text-zinc-400 col-span-3">
                                    Standard Profile (23 default cumulative histogram bounds)
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Retention & Ingest Quota Row */}
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        {/* Data Retention */}
                        <div className="surface-panel p-5 space-y-3">
                            <div className="flex items-center gap-2">
                                <Clock className="w-4 h-4 text-[#D99A3D]" />
                                <h2 className="text-sm font-semibold text-zinc-200 uppercase tracking-wide">
                                    Data Retention Window
                                </h2>
                            </div>

                            <p className="text-xs text-zinc-400 leading-relaxed">
                                Metric aggregations older than this threshold are pruned by the background maintenance cycle.
                            </p>

                            <div className="flex items-center gap-2 pt-1">
                                <input
                                    type="number"
                                    min={7}
                                    max={730}
                                    value={dataRetentionDays}
                                    onChange={(e) => setDataRetentionDays(Math.max(1, Number(e.target.value)))}
                                    disabled={!canEdit}
                                    className={cn(
                                        "w-28 bg-[#0E1014] border border-[#242932] rounded px-3 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6]",
                                        !canEdit && "cursor-not-allowed opacity-60"
                                    )}
                                />
                                <span className="text-xs text-zinc-400 font-mono">days</span>
                            </div>

                            <div className="flex items-center gap-1.5 pt-1">
                                <span className="text-[11px] text-zinc-500 font-medium">Presets:</span>
                                {[30, 60, 90, 180, 365].map((d) => (
                                    <button
                                        key={d}
                                        type="button"
                                        onClick={() => canEdit && setDataRetentionDays(d)}
                                        disabled={!canEdit}
                                        className={cn(
                                            "px-2 py-0.5 rounded text-xs font-mono border transition-colors",
                                            dataRetentionDays === d
                                                ? "bg-[#D99A3D]/15 text-[#D99A3D] border-[#D99A3D]/40 font-semibold"
                                                : "bg-[#0E1014] text-zinc-400 border-[#242932] hover:text-zinc-200",
                                            !canEdit && "cursor-not-allowed opacity-60"
                                        )}
                                    >
                                        {d}d
                                    </button>
                                ))}
                            </div>
                        </div>

                        {/* Daily Ingest Quota */}
                        <div className="surface-panel p-5 space-y-4">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                    <Database className="w-4 h-4 text-[#A78BFA]" />
                                    <h2 className="text-sm font-semibold text-zinc-200 uppercase tracking-wide">
                                        Daily Ingest Quota
                                    </h2>
                                </div>
                                <span className={cn(
                                    "px-2 py-0.5 rounded text-[11px] font-mono font-semibold border",
                                    isQuotaUnlimited
                                        ? "bg-[#48B982]/10 text-[#48B982] border-[#48B982]/30"
                                        : "bg-[#4CB8D6]/10 text-[#4CB8D6] border-[#4CB8D6]/30"
                                )}>
                                    {isQuotaUnlimited ? "∞ Unlimited" : `${dailyIngestQuota.toLocaleString()} / day`}
                                </span>
                            </div>

                            <p className="text-xs text-zinc-400 leading-relaxed">
                                Maximum incoming telemetry events processed per 24-hour UTC window before rate limiting.
                            </p>

                            {/* Mode Switcher: Unlimited vs Custom Cap */}
                            <div className="grid grid-cols-2 gap-2 p-1 bg-[#0E1014] border border-[#242932] rounded-lg max-w-sm">
                                <button
                                    type="button"
                                    onClick={() => canEdit && setIsQuotaUnlimited(true)}
                                    disabled={!canEdit}
                                    className={cn(
                                        "flex items-center justify-center gap-2 py-2 px-3 rounded text-xs font-semibold transition-all cursor-pointer",
                                        isQuotaUnlimited
                                            ? "bg-[#242932] text-zinc-100 shadow-sm border border-[#374151]"
                                            : "text-zinc-400 hover:text-zinc-200 hover:bg-[#181D24]",
                                        !canEdit && "cursor-not-allowed opacity-60"
                                    )}
                                >
                                    <span className="text-sm leading-none">∞</span>
                                    <span>Unlimited Quota</span>
                                </button>

                                <button
                                    type="button"
                                    onClick={() => canEdit && setIsQuotaUnlimited(false)}
                                    disabled={!canEdit}
                                    className={cn(
                                        "flex items-center justify-center gap-2 py-2 px-3 rounded text-xs font-semibold transition-all cursor-pointer",
                                        !isQuotaUnlimited
                                            ? "bg-[#242932] text-[#4CB8D6] shadow-sm border border-[#374151]"
                                            : "text-zinc-400 hover:text-zinc-200 hover:bg-[#181D24]",
                                        !canEdit && "cursor-not-allowed opacity-60"
                                    )}
                                >
                                    <Sliders className="w-3.5 h-3.5" />
                                    <span>Custom Cap</span>
                                </button>
                            </div>

                            {/* Quota details / inputs */}
                            {isQuotaUnlimited ? (
                                <div className="p-3 rounded bg-[#0E1014]/60 border border-[#242932] text-xs text-zinc-400 flex items-center gap-2">
                                    <Check className="w-4 h-4 text-[#48B982] shrink-0" />
                                    <span>
                                        Ingestion has no volume restrictions. All telemetry payloads are accepted without daily caps.
                                    </span>
                                </div>
                            ) : (
                                <div className="space-y-3 pt-1">
                                    <div className="flex items-center gap-2 flex-wrap">
                                        <div className="relative flex-1 min-w-[200px] max-w-xs">
                                            <input
                                                type="number"
                                                min={1000}
                                                step={5000}
                                                value={dailyIngestQuota}
                                                onChange={(e) => setDailyIngestQuota(Math.max(100, Number(e.target.value)))}
                                                disabled={!canEdit}
                                                placeholder="e.g. 100000"
                                                className={cn(
                                                    "w-full bg-[#0E1014] border border-[#242932] rounded px-3 py-2 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6]",
                                                    !canEdit && "cursor-not-allowed opacity-60"
                                                )}
                                            />
                                            <span className="absolute right-3 top-2 text-xs text-zinc-500 font-mono pointer-events-none">
                                                /day
                                            </span>
                                        </div>

                                        {/* Presets */}
                                        <div className="flex items-center gap-1.5 flex-wrap">
                                            {[50_000, 100_000, 250_000, 500_000, 1_000_000].map((preset) => (
                                                <button
                                                    key={preset}
                                                    type="button"
                                                    disabled={!canEdit}
                                                    onClick={() => canEdit && setDailyIngestQuota(preset)}
                                                    className={cn(
                                                        "px-2.5 py-1.5 rounded text-[11px] font-mono border transition-colors cursor-pointer",
                                                        dailyIngestQuota === preset
                                                            ? "bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/40 font-bold"
                                                            : "bg-[#0E1014] text-zinc-400 border-[#242932] hover:text-zinc-200 hover:border-zinc-700",
                                                        !canEdit && "cursor-not-allowed opacity-60"
                                                    )}
                                                >
                                                    {preset >= 1_000_000 ? `${preset / 1_000_000}M` : `${preset / 1_000}k`}
                                                </button>
                                            ))}
                                        </div>
                                    </div>
                                    <p className="text-[11px] text-zinc-500">
                                        Payloads exceeding {dailyIngestQuota.toLocaleString()} events/day will receive HTTP 429 Too Many Requests until the UTC midnight reset.
                                    </p>
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Actions Bar */}
                    <div className="flex items-center justify-between pt-4 border-t border-[#242932]">
                        <div className="flex items-center gap-2">
                            {hasChanges && canEdit && (
                                <span className="text-xs text-[#D99A3D] flex items-center gap-1.5">
                                    <span className="w-2 h-2 rounded-full bg-[#D99A3D] animate-pulse" />
                                    Unsaved modifications
                                </span>
                            )}
                        </div>

                        <div className="flex items-center gap-3">
                            {canEdit && (
                                <Button
                                    type="button"
                                    variant="outline"
                                    size="sm"
                                    onClick={handleReset}
                                    disabled={!hasChanges || updateMutation.isPending}
                                    className="cursor-pointer text-xs"
                                >
                                    <RotateCcw size={13} className="mr-1.5" />
                                    Reset Changes
                                </Button>
                            )}

                            {canEdit ? (
                                <Button
                                    type="submit"
                                    size="sm"
                                    disabled={!hasChanges || updateMutation.isPending}
                                    className="bg-[#4CB8D6] hover:bg-[#3fa5c0] text-[#0A0C10] font-semibold text-xs cursor-pointer"
                                >
                                    {updateMutation.isPending ? (
                                        <>
                                            <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
                                            Saving Configuration...
                                        </>
                                    ) : (
                                        <>
                                            <Save size={13} className="mr-1.5" />
                                            Save Configuration
                                        </>
                                    )}
                                </Button>
                            ) : (
                                <div className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-white/5 border border-white/10 text-zinc-400 text-xs font-medium cursor-not-allowed">
                                    <Lock size={12} />
                                    View-Only Mode (Updates Disabled)
                                </div>
                            )}
                        </div>
                    </div>
                </form>
            )}
        </div>
    );
}
