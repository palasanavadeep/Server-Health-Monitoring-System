"use client";

import React, { useState, useEffect } from 'react';
import { Modal } from '@/components/ui/modal';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { useAuth } from '@/contexts/auth-context';
import { 
    useTenantConfigQuery, 
    useUpdateTenantConfigMutation, 
    useHistogramProfilesQuery 
} from '@/hooks/use-tenant-config';
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
    Lock
} from 'lucide-react';

interface TenantConfigModalProps {
    isOpen: boolean;
    onClose: () => void;
    clientId: string;
    clientName: string;
}

export function TenantConfigModal({
    isOpen,
    onClose,
    clientId,
    clientName,
}: TenantConfigModalProps) {
    const toast = useToast();
    const { user } = useAuth();
    const isSuperAdmin = user?.role === 'super_admin';
    const isClientAdmin = user?.role === 'client_admin';
    const canEdit = isSuperAdmin || (isClientAdmin && user?.clientId === clientId);

    // Queries & Mutations
    const { data: config, isPending: isLoadingConfig } = useTenantConfigQuery(clientId, {
        enabled: isOpen && !!clientId,
    });
    const { data: profiles = [] } = useHistogramProfilesQuery({
        enabled: isOpen,
    });
    const updateMutation = useUpdateTenantConfigMutation();

    // Form state
    const [apdexThresholdMs, setApdexThresholdMs] = useState(500);
    const [selectedProfile, setSelectedProfile] = useState('standard');
    const [dataRetentionDays, setDataRetentionDays] = useState(90);
    const [isQuotaUnlimited, setIsQuotaUnlimited] = useState(true);
    const [dailyIngestQuota, setDailyIngestQuota] = useState<number>(100_000);

    // Sync state when config loads or modal opens
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
    }, [config, isOpen]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!canEdit) {
            toast('Permission denied: Only Super Admins and Client Admins can update configuration', 'error');
            return;
        }
        try {
            await updateMutation.mutateAsync({
                clientId,
                data: {
                    apdexThresholdMs,
                    histogramProfile: selectedProfile,
                    dataRetentionDays,
                    dailyIngestQuota: isQuotaUnlimited ? null : dailyIngestQuota,
                },
            });
            toast(`Telemetry configuration updated for ${clientName}`, 'success');
            onClose();
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to update tenant configuration', 'error');
        }
    };

    return (
        <Modal
            isOpen={isOpen}
            onClose={onClose}
            title={`Tenant Telemetry Config: ${clientName}`}
            description="Fine-tune per-tenant metric interpretation, histogram profile lenses, data retention, and ingest quotas."
            maxWidth="max-w-2xl"
        >
            {isLoadingConfig ? (
                <div className="py-16 text-center text-zinc-400 space-y-3">
                    <Loader2 className="w-6 h-6 animate-spin mx-auto text-[#4CB8D6]" />
                    <p className="text-xs">Loading tenant configuration...</p>
                </div>
            ) : (
                <form onSubmit={handleSubmit} className="space-y-6">
                    {!canEdit && (
                        <div className="p-3 rounded-lg bg-[#D99A3D]/10 border border-[#D99A3D]/30 flex items-center gap-2.5 text-xs text-[#D99A3D]">
                            <ShieldAlert size={14} className="shrink-0" />
                            <span>Read-Only Mode: You do not have permission to modify configuration for this client.</span>
                        </div>
                    )}
                    {/* Apdex Target Configuration */}
                    <div className="space-y-2 p-4 rounded-lg bg-[#0E1014] border border-[#242932]">
                        <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                                <Gauge className="w-4 h-4 text-[#4CB8D6]" />
                                <label className="text-xs font-semibold text-zinc-200 uppercase tracking-wide">
                                    Apdex T Target (Latency Threshold)
                                </label>
                            </div>
                            <span className="font-mono text-xs font-bold text-[#4CB8D6]">
                                {apdexThresholdMs} ms
                            </span>
                        </div>
                        <p className="text-[11px] text-zinc-400 leading-relaxed">
                            Apdex measures user satisfaction. Requests ≤ <strong className="text-zinc-200">{apdexThresholdMs}ms</strong> are Satisfied.
                            Between <strong className="text-zinc-200">{apdexThresholdMs}ms</strong> and <strong className="text-zinc-200">{apdexThresholdMs * 4}ms</strong> (4T) are Tolerating.
                            Over <strong className="text-zinc-200">{apdexThresholdMs * 4}ms</strong> are Frustrated.
                        </p>
                        <div className="flex items-center gap-3 pt-2">
                            <input
                                type="range"
                                min={50}
                                max={3000}
                                step={50}
                                value={apdexThresholdMs}
                                onChange={(e) => setApdexThresholdMs(Number(e.target.value))}
                                className="w-full accent-[#4CB8D6] cursor-pointer"
                            />
                            <div className="flex items-center gap-1.5 shrink-0">
                                <input
                                    type="number"
                                    min={10}
                                    max={10000}
                                    value={apdexThresholdMs}
                                    onChange={(e) => setApdexThresholdMs(Math.max(10, Number(e.target.value)))}
                                    className="w-20 bg-[#111419] border border-[#242932] rounded px-2 py-1 text-xs font-mono text-zinc-200 text-right focus:outline-none focus:border-[#4CB8D6]"
                                />
                                <span className="text-xs text-zinc-500">ms</span>
                            </div>
                        </div>
                        <div className="flex gap-2 pt-1">
                            {[200, 500, 1000, 1500, 2000].map((preset) => (
                                <button
                                    key={preset}
                                    type="button"
                                    onClick={() => setApdexThresholdMs(preset)}
                                    className={`px-2 py-0.5 rounded text-[10px] font-mono border transition-colors cursor-pointer ${
                                        apdexThresholdMs === preset
                                            ? 'bg-[#4CB8D6]/10 text-[#4CB8D6] border-[#4CB8D6]/40'
                                            : 'bg-white/5 text-zinc-400 border-white/5 hover:text-zinc-200'
                                    }`}
                                >
                                    {preset}ms
                                </button>
                            ))}
                        </div>
                    </div>

                    {/* Histogram Profile Lens */}
                    <div className="space-y-2 p-4 rounded-lg bg-[#0E1014] border border-[#242932]">
                        <div className="flex items-center gap-2">
                            <Layers className="w-4 h-4 text-[#48B982]" />
                            <label className="text-xs font-semibold text-zinc-200 uppercase tracking-wide">
                                Histogram Percentile Profile
                            </label>
                        </div>
                        <p className="text-[11px] text-zinc-400 leading-relaxed">
                            Determines the resolution lens used to interpolate p50–p99 latency quantiles. All metrics are stored with 23 standard buckets.
                        </p>
                        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5 pt-2">
                            {profiles.length > 0 ? (
                                profiles.map((p) => {
                                    const isSelected = selectedProfile === p.name;
                                    return (
                                        <div
                                            key={p.name}
                                            onClick={() => setSelectedProfile(p.name)}
                                            className={`p-3 rounded-lg border cursor-pointer transition-all ${
                                                isSelected
                                                    ? 'bg-[#48B982]/10 border-[#48B982]/50 shadow-sm'
                                                    : 'bg-[#111419] border-[#242932] hover:border-zinc-700'
                                            }`}
                                        >
                                            <div className="flex items-center justify-between mb-1">
                                                <span className="text-xs font-semibold text-zinc-200 capitalize">
                                                    {p.name.replace('_', ' ')}
                                                </span>
                                                {isSelected && <Check size={13} className="text-[#48B982]" />}
                                            </div>
                                            <p className="text-[10px] text-zinc-400 line-clamp-2 leading-relaxed">
                                                {p.description}
                                            </p>
                                            <div className="mt-2 text-[10px] font-mono text-zinc-500">
                                                {p.bucketBounds?.length || 0} visible buckets
                                            </div>
                                        </div>
                                    );
                                })
                            ) : (
                                <div className="p-3 rounded bg-[#111419] border border-[#242932] text-xs text-zinc-400 col-span-3">
                                    Standard profile (23 cumulative buckets)
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Data Retention & Daily Quota */}
                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                        {/* Data Retention */}
                        <div className="p-4 rounded-lg bg-[#0E1014] border border-[#242932] space-y-2">
                            <div className="flex items-center gap-2">
                                <Clock className="w-4 h-4 text-[#D99A3D]" />
                                <label className="text-xs font-semibold text-zinc-200 uppercase tracking-wide">
                                    Data Retention
                                </label>
                            </div>
                            <p className="text-[11px] text-zinc-400">
                                Prune aggregated metric rows older than this threshold.
                            </p>
                            <div className="flex items-center gap-2 pt-1">
                                <input
                                    type="number"
                                    min={7}
                                    max={730}
                                    value={dataRetentionDays}
                                    onChange={(e) => setDataRetentionDays(Math.max(1, Number(e.target.value)))}
                                    className="w-24 bg-[#111419] border border-[#242932] rounded px-2.5 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                                />
                                <span className="text-xs text-zinc-400">days</span>
                            </div>
                            <div className="flex gap-1.5 pt-1">
                                {[30, 60, 90, 180, 365].map((d) => (
                                    <button
                                        key={d}
                                        type="button"
                                        onClick={() => setDataRetentionDays(d)}
                                        className={`px-1.5 py-0.5 rounded text-[10px] font-mono border transition-colors cursor-pointer ${
                                            dataRetentionDays === d
                                                ? 'bg-[#D99A3D]/10 text-[#D99A3D] border-[#D99A3D]/40'
                                                : 'bg-white/5 text-zinc-400 border-white/5 hover:text-zinc-200'
                                        }`}
                                    >
                                        {d}d
                                    </button>
                                ))}
                            </div>
                        </div>

                        {/* Daily Ingest Quota */}
                        <div className="p-4 rounded-lg bg-[#0E1014] border border-[#242932] space-y-3">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                    <Database className="w-4 h-4 text-[#A78BFA]" />
                                    <label className="text-xs font-semibold text-zinc-200 uppercase tracking-wide">
                                        Daily Ingest Quota
                                    </label>
                                </div>
                                <span className={`px-2 py-0.5 rounded text-[10px] font-mono font-semibold border ${
                                    isQuotaUnlimited
                                        ? 'bg-[#48B982]/10 text-[#48B982] border-[#48B982]/30'
                                        : 'bg-[#4CB8D6]/10 text-[#4CB8D6] border-[#4CB8D6]/30'
                                }`}>
                                    {isQuotaUnlimited ? '∞ Unlimited' : `${dailyIngestQuota.toLocaleString()} / day`}
                                </span>
                            </div>
                            <p className="text-[11px] text-zinc-400">
                                Maximum incoming telemetry events processed per 24-hour UTC window before rate limiting.
                            </p>
                            
                            {/* Mode Switcher */}
                            <div className="grid grid-cols-2 gap-1.5 p-1 bg-[#111419] border border-[#242932] rounded-lg max-w-xs">
                                <button
                                    type="button"
                                    onClick={() => setIsQuotaUnlimited(true)}
                                    className={`flex items-center justify-center gap-1.5 py-1.5 px-2 rounded text-xs font-semibold transition-all cursor-pointer ${
                                        isQuotaUnlimited
                                            ? 'bg-[#242932] text-zinc-100 shadow-sm border border-[#374151]'
                                            : 'text-zinc-400 hover:text-zinc-200'
                                    }`}
                                >
                                    <span>∞</span>
                                    <span>Unlimited</span>
                                </button>
                                <button
                                    type="button"
                                    onClick={() => setIsQuotaUnlimited(false)}
                                    className={`flex items-center justify-center gap-1.5 py-1.5 px-2 rounded text-xs font-semibold transition-all cursor-pointer ${
                                        !isQuotaUnlimited
                                            ? 'bg-[#242932] text-[#4CB8D6] shadow-sm border border-[#374151]'
                                            : 'text-zinc-400 hover:text-zinc-200'
                                    }`}
                                >
                                    <Sliders className="w-3 h-3" />
                                    <span>Custom Cap</span>
                                </button>
                            </div>

                            {!isQuotaUnlimited && (
                                <div className="space-y-2 pt-1">
                                    <div className="flex items-center gap-2">
                                        <div className="relative flex-1">
                                            <input
                                                type="number"
                                                min={1000}
                                                step={5000}
                                                value={dailyIngestQuota}
                                                onChange={(e) => setDailyIngestQuota(Math.max(100, Number(e.target.value)))}
                                                className="w-full bg-[#111419] border border-[#242932] rounded px-2.5 py-1.5 text-xs font-mono text-zinc-200 focus:outline-none focus:border-[#4CB8D6]"
                                                placeholder="e.g. 100000"
                                            />
                                            <span className="absolute right-2.5 top-1.5 text-[11px] text-zinc-500 font-mono pointer-events-none">/day</span>
                                        </div>
                                    </div>
                                    <div className="flex items-center gap-1 flex-wrap">
                                        {[50_000, 100_000, 250_000, 500_000, 1_000_000].map((preset) => (
                                            <button
                                                key={preset}
                                                type="button"
                                                onClick={() => setDailyIngestQuota(preset)}
                                                className={`px-2 py-0.5 rounded text-[10px] font-mono border transition-colors cursor-pointer ${
                                                    dailyIngestQuota === preset
                                                        ? 'bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/40 font-bold'
                                                        : 'bg-[#111419] text-zinc-400 border-[#242932] hover:text-zinc-200'
                                                }`}
                                            >
                                                {preset >= 1_000_000 ? `${preset / 1_000_000}M` : `${preset / 1_000}k`}
                                            </button>
                                        ))}
                                    </div>
                                </div>
                            )}
                        </div>
                    </div>

                    {/* Actions */}
                    <div className="flex items-center justify-end gap-3 pt-3 border-t border-[#242932]">
                        <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={onClose}
                            className="cursor-pointer"
                        >
                            Cancel
                        </Button>
                        {canEdit ? (
                            <Button
                                type="submit"
                                size="sm"
                                disabled={updateMutation.isPending}
                                className="bg-[#4CB8D6] hover:bg-[#3fa5c0] text-[#0A0C10] font-semibold cursor-pointer"
                            >
                                {updateMutation.isPending ? (
                                    <>
                                        <Loader2 className="w-3.5 h-3.5 animate-spin mr-1.5" />
                                        Saving...
                                    </>
                                ) : (
                                    'Save Configuration'
                                )}
                            </Button>
                        ) : (
                            <div className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-white/5 border border-white/10 text-zinc-400 text-xs font-medium">
                                <Lock size={12} />
                                View-Only Mode
                            </div>
                        )}
                    </div>
                </form>
            )}
        </Modal>
    );
}
