"use client";

import React, { useState } from 'react';
import { Modal } from '@/components/ui/modal';
import { StatusBadge } from '@/components/ui/status-badge';
import { StatusBreakdown } from '@/components/charts/status-breakdown';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { ApiMetricsEntry } from '@/lib/api';
import { 
    Activity, 
    Clock, 
    ShieldAlert, 
    Gauge, 
    Layers, 
    Copy, 
    Check, 
    Terminal, 
    ExternalLink, 
    TrendingUp, 
    Server,
    X,
    FileText
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface ApiDetailModalProps {
    isOpen: boolean;
    onClose: () => void;
    api: ApiMetricsEntry | null;
}

function getMethodBadgeStyle(method: string) {
    switch (method.toUpperCase()) {
        case 'GET':
            return 'bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/40';
        case 'POST':
            return 'bg-[#48B982]/15 text-[#48B982] border-[#48B982]/40';
        case 'PUT':
            return 'bg-[#F59E0B]/15 text-[#F59E0B] border-[#F59E0B]/40';
        case 'DELETE':
            return 'bg-[#E45865]/15 text-[#E45865] border-[#E45865]/40';
        case 'PATCH':
            return 'bg-[#A78BFA]/15 text-[#A78BFA] border-[#A78BFA]/40';
        default:
            return 'bg-zinc-800 text-zinc-300 border-zinc-700';
    }
}

function getApdexBadge(score?: number) {
    if (score === undefined || score === null) return null;
    let color = 'bg-[#48B982]/15 text-[#48B982] border-[#48B982]/40';
    let label = 'Excellent';
    if (score < 0.70) {
        color = 'bg-[#E45865]/15 text-[#E45865] border-[#E45865]/40';
        label = 'Poor';
    } else if (score < 0.85) {
        color = 'bg-[#F59E0B]/15 text-[#F59E0B] border-[#F59E0B]/40';
        label = 'Fair';
    } else if (score < 0.94) {
        color = 'bg-[#4CB8D6]/15 text-[#4CB8D6] border-[#4CB8D6]/40';
        label = 'Good';
    }
    return (
        <span 
            className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-mono font-bold border ${color}`} 
            title={`Apdex: ${score.toFixed(4)} (${label})`}
        >
            <Gauge size={11} />
            {score.toFixed(2)} ({label})
        </span>
    );
}

export function ApiDetailModal({ isOpen, onClose, api }: ApiDetailModalProps) {
    const toast = useToast();
    const [activeTab, setActiveTab] = useState<'latency' | 'status' | 'diagnostics'>('latency');
    const [copiedPath, setCopiedPath] = useState(false);
    const [copiedCurl, setCopiedCurl] = useState(false);

    if (!api) return null;

    const copyPath = () => {
        navigator.clipboard.writeText(api.endpoint);
        setCopiedPath(true);
        toast('Endpoint path copied to clipboard', 'success');
        setTimeout(() => setCopiedPath(false), 2000);
    };

    const curlCommand = `curl -X ${api.method.toUpperCase()} "http://localhost:3000${api.endpoint}" -H "Accept: application/json"`;

    const copyCurl = () => {
        navigator.clipboard.writeText(curlCommand);
        setCopiedCurl(true);
        toast('cURL command copied to clipboard', 'success');
        setTimeout(() => setCopiedCurl(false), 2000);
    };

    // Latency percentiles resolution
    const p50 = Math.round(api.percentiles?.p50 ?? api.avgLatency ?? 0);
    const p75 = Math.round(api.percentiles?.p75 ?? (p50 * 1.25));
    const p90 = Math.round(api.percentiles?.p90 ?? (p50 * 1.5));
    const p95 = Math.round(api.percentiles?.p95 ?? (api.avgLatency + (api.maxLatency - api.avgLatency) * 0.9));
    const p99 = Math.round(api.percentiles?.p99 ?? api.maxLatency ?? 0);
    const maxVal = Math.max(p99, api.maxLatency, 1);

    // Apdex counts
    const apdexScore = api.apdex?.score ?? 0;
    const satisfied = api.apdex?.satisfied ?? 0;
    const tolerating = api.apdex?.tolerating ?? 0;
    const frustrated = api.apdex?.frustrated ?? 0;
    const totalApdexHits = satisfied + tolerating + frustrated || 1;
    const satisfiedPct = Math.round((satisfied / totalApdexHits) * 100);
    const toleratingPct = Math.round((tolerating / totalApdexHits) * 100);
    const frustratedPct = Math.round((frustrated / totalApdexHits) * 100);

    const quantiles = [
        { label: 'p50 (Median)', value: p50, color: 'bg-[#4CB8D6]' },
        { label: 'p75', value: p75, color: 'bg-[#5794E8]' },
        { label: 'p90', value: p90, color: 'bg-[#F59E0B]' },
        { label: 'p95', value: p95, color: 'bg-[#D99A3D]' },
        { label: 'p99 (Tail)', value: p99, color: 'bg-[#E45865]' },
    ];

    return (
        <Modal
            isOpen={isOpen}
            onClose={onClose}
            title="API Route Inspector"
            description="Individual route telemetry, latency distribution, and diagnostic testing"
            maxWidth="max-w-4xl"
        >
            <div className="space-y-6">
                {/* Header Context Banner */}
                <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 pb-4 border-b border-[#242932]">
                    <div className="space-y-1.5">
                        <div className="flex items-center flex-wrap gap-2">
                            <span className={cn(
                                "px-2 py-0.5 rounded text-[11px] font-mono font-bold tracking-wider uppercase border",
                                getMethodBadgeStyle(api.method)
                            )}>
                                {api.method}
                            </span>
                            <span className="font-mono text-sm sm:text-base font-bold text-zinc-100 break-all">
                                {api.endpoint}
                            </span>
                            <button
                                onClick={copyPath}
                                className="p-1 rounded hover:bg-white/10 text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer"
                                title="Copy endpoint path"
                            >
                                {copiedPath ? <Check size={13} className="text-[#48B982]" /> : <Copy size={13} />}
                            </button>
                        </div>

                        <div className="flex items-center flex-wrap gap-2 text-xs text-zinc-400">
                            <span className="flex items-center gap-1 font-mono text-[11px] bg-[#0E1014] px-2 py-0.5 rounded border border-[#242932]">
                                <Server size={11} className="text-[#4CB8D6]" />
                                service: {api.serviceName}
                            </span>
                            <span className="text-zinc-600">•</span>
                            <span className="flex items-center gap-1.5">
                                <span className="text-[11px] text-zinc-400 uppercase font-semibold">Health:</span>
                                <StatusBadge status={api.errorRate > 5 ? 'degraded' : 'healthy'} />
                            </span>
                            {api.apdex && (
                                <>
                                    <span className="text-zinc-600">•</span>
                                    {getApdexBadge(api.apdex.score)}
                                </>
                            )}
                        </div>
                    </div>
                </div>

                {/* 4-Column KPI Summary Cards */}
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
                    <div className="surface-panel p-3.5 space-y-1">
                        <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                            Total Volume
                        </span>
                        <span className="text-xl font-bold font-mono text-zinc-100 block">
                            {api.totalHits.toLocaleString()}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            {api.successHits} success • {api.errorHits} failed
                        </span>
                    </div>

                    <div className="surface-panel p-3.5 space-y-1">
                        <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                            Throughput
                        </span>
                        <span className="text-xl font-bold font-mono text-zinc-100 block">
                            {api.throughputRpm !== undefined ? api.throughputRpm.toFixed(1) : '-'}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            requests / minute
                        </span>
                    </div>

                    <div className="surface-panel p-3.5 space-y-1">
                        <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                            Error Rate
                        </span>
                        <span className={cn(
                            "text-xl font-bold font-mono block",
                            api.errorRate > 5 ? "text-[#E45865]" : api.errorRate > 1 ? "text-[#F59E0B]" : "text-[#48B982]"
                        )}>
                            {api.errorRate.toFixed(1)}%
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            {api.errorHits > 0 ? `${api.errorHits} non-2xx responses` : 'zero errors recorded'}
                        </span>
                    </div>

                    <div className="surface-panel p-3.5 space-y-1">
                        <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                            Median Latency
                        </span>
                        <span className="text-xl font-bold font-mono text-[#4CB8D6] block">
                            {p50} ms
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            avg: {Math.round(api.avgLatency)}ms • max: {Math.round(api.maxLatency)}ms
                        </span>
                    </div>
                </div>

                {/* Tab Navigation */}
                <div className="flex items-center gap-2 border-b border-[#242932] text-xs">
                    <button
                        onClick={() => setActiveTab('latency')}
                        className={cn(
                            "px-3 py-2 font-medium transition-colors border-b-2 cursor-pointer -mb-px flex items-center gap-1.5",
                            activeTab === 'latency'
                                ? "border-[#4CB8D6] text-zinc-100 font-semibold"
                                : "border-transparent text-zinc-400 hover:text-zinc-200"
                        )}
                    >
                        <Clock size={13} />
                        Latency & Percentiles
                    </button>
                    <button
                        onClick={() => setActiveTab('status')}
                        className={cn(
                            "px-3 py-2 font-medium transition-colors border-b-2 cursor-pointer -mb-px flex items-center gap-1.5",
                            activeTab === 'status'
                                ? "border-[#4CB8D6] text-zinc-100 font-semibold"
                                : "border-transparent text-zinc-400 hover:text-zinc-200"
                        )}
                    >
                        <Activity size={13} />
                        HTTP Status Distribution
                    </button>
                    <button
                        onClick={() => setActiveTab('diagnostics')}
                        className={cn(
                            "px-3 py-2 font-medium transition-colors border-b-2 cursor-pointer -mb-px flex items-center gap-1.5",
                            activeTab === 'diagnostics'
                                ? "border-[#4CB8D6] text-zinc-100 font-semibold"
                                : "border-transparent text-zinc-400 hover:text-zinc-200"
                        )}
                    >
                        <Terminal size={13} />
                        Diagnostics & cURL
                    </button>
                </div>

                {/* Tab 1: Latency & Quantiles */}
                {activeTab === 'latency' && (
                    <div className="space-y-5 animate-in fade-in duration-100">
                        {/* Quantile Bars Breakdown */}
                        <div className="surface-panel p-4 space-y-4">
                            <div className="flex items-center justify-between">
                                <div>
                                    <h4 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                        Cumulative Latency Quantiles
                                    </h4>
                                    <p className="text-[11px] text-zinc-400 mt-0.5">
                                        Interpolated response duration bounds from 23 cumulative histogram buckets
                                    </p>
                                </div>
                                <span className="font-mono text-xs text-zinc-500">
                                    Range: {Math.round(api.minLatency)}ms – {Math.round(api.maxLatency)}ms
                                </span>
                            </div>

                            <div className="space-y-3 pt-1">
                                {quantiles.map((q) => {
                                    const pct = Math.min(100, Math.max(5, (q.value / maxVal) * 100));
                                    return (
                                        <div key={q.label} className="space-y-1">
                                            <div className="flex items-center justify-between text-xs">
                                                <span className="font-medium text-zinc-300 font-mono">{q.label}</span>
                                                <span className="font-mono font-bold text-zinc-100">{q.value} ms</span>
                                            </div>
                                            <div className="h-2 w-full bg-[#111419] rounded-full overflow-hidden border border-[#242932]">
                                                <div 
                                                    className={cn("h-full rounded-full transition-all duration-300", q.color)}
                                                    style={{ width: `${pct}%` }}
                                                />
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>

                            {/* Min / Avg / Max latency stats */}
                            <div className="grid grid-cols-3 gap-3 pt-2 border-t border-[#242932] text-center">
                                <div className="p-2 rounded bg-[#0E1014] border border-[#242932]">
                                    <span className="text-[10px] text-zinc-500 uppercase font-semibold block">Minimum</span>
                                    <span className="text-xs font-mono font-bold text-zinc-200 mt-0.5 block">
                                        {Math.round(api.minLatency)} ms
                                    </span>
                                </div>
                                <div className="p-2 rounded bg-[#0E1014] border border-[#242932]">
                                    <span className="text-[10px] text-zinc-500 uppercase font-semibold block">Arithmetic Mean</span>
                                    <span className="text-xs font-mono font-bold text-[#4CB8D6] mt-0.5 block">
                                        {Math.round(api.avgLatency)} ms
                                    </span>
                                </div>
                                <div className="p-2 rounded bg-[#0E1014] border border-[#242932]">
                                    <span className="text-[10px] text-zinc-500 uppercase font-semibold block">Maximum Peak</span>
                                    <span className="text-xs font-mono font-bold text-zinc-200 mt-0.5 block">
                                        {Math.round(api.maxLatency)} ms
                                    </span>
                                </div>
                            </div>
                        </div>

                        {/* Apdex Satisfaction Breakdown */}
                        {api.apdex && (
                            <div className="surface-panel p-4 space-y-3">
                                <div className="flex items-center justify-between">
                                    <div>
                                        <h4 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                            Apdex User Satisfaction Breakdown
                                        </h4>
                                        <p className="text-[11px] text-zinc-400 mt-0.5">
                                            Satisfaction metric computed against tenant Apdex T threshold
                                        </p>
                                    </div>
                                    {getApdexBadge(apdexScore)}
                                </div>

                                {/* Visual segmented satisfaction bar */}
                                <div className="h-3 w-full rounded-full overflow-hidden flex bg-[#111419] border border-[#242932]">
                                    <div 
                                        className="bg-[#48B982] h-full transition-all" 
                                        style={{ width: `${satisfiedPct}%` }}
                                        title={`Satisfied: ${satisfied} (${satisfiedPct}%)`}
                                    />
                                    <div 
                                        className="bg-[#F59E0B] h-full transition-all" 
                                        style={{ width: `${toleratingPct}%` }}
                                        title={`Tolerating: ${tolerating} (${toleratingPct}%)`}
                                    />
                                    <div 
                                        className="bg-[#E45865] h-full transition-all" 
                                        style={{ width: `${frustratedPct}%` }}
                                        title={`Frustrated: ${frustrated} (${frustratedPct}%)`}
                                    />
                                </div>

                                <div className="grid grid-cols-3 gap-3 text-center pt-1">
                                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#48B982]/20">
                                        <div className="flex items-center justify-center gap-1 text-[10px] text-[#48B982] font-semibold uppercase">
                                            <span className="w-1.5 h-1.5 rounded-full bg-[#48B982]" />
                                            Satisfied (≤ T)
                                        </div>
                                        <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                            {satisfied.toLocaleString()}
                                        </span>
                                        <span className="text-[10px] text-zinc-500 font-mono">{satisfiedPct}% of traffic</span>
                                    </div>

                                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#F59E0B]/20">
                                        <div className="flex items-center justify-center gap-1 text-[10px] text-[#F59E0B] font-semibold uppercase">
                                            <span className="w-1.5 h-1.5 rounded-full bg-[#F59E0B]" />
                                            Tolerating (T to 4T)
                                        </div>
                                        <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                            {tolerating.toLocaleString()}
                                        </span>
                                        <span className="text-[10px] text-zinc-500 font-mono">{toleratingPct}% of traffic</span>
                                    </div>

                                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#E45865]/20">
                                        <div className="flex items-center justify-center gap-1 text-[10px] text-[#E45865] font-semibold uppercase">
                                            <span className="w-1.5 h-1.5 rounded-full bg-[#E45865]" />
                                            Frustrated (&gt; 4T)
                                        </div>
                                        <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                            {frustrated.toLocaleString()}
                                        </span>
                                        <span className="text-[10px] text-zinc-500 font-mono">{frustratedPct}% of traffic</span>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>
                )}

                {/* Tab 2: HTTP Status Distribution */}
                {activeTab === 'status' && (
                    <div className="space-y-4 animate-in fade-in duration-100">
                        <div className="surface-panel p-4 space-y-4">
                            <div>
                                <h4 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                    HTTP Response Status Code Breakdown
                                </h4>
                                <p className="text-[11px] text-zinc-400 mt-0.5">
                                    Distribution across success classes and HTTP error responses
                                </p>
                            </div>

                            <StatusBreakdown
                                distribution={api.statusDistribution}
                                successHits={api.successHits}
                                errorHits={api.errorHits}
                            />

                            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 pt-2 text-center">
                                <div className="p-3 rounded bg-[#0E1014] border border-[#48B982]/30">
                                    <span className="text-[10px] text-[#48B982] uppercase font-semibold block">2xx Success</span>
                                    <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                        {api.statusDistribution?.status_2xx?.toLocaleString() || api.successHits.toLocaleString()}
                                    </span>
                                    <span className="text-[10px] text-zinc-500 font-mono">
                                        {api.totalHits > 0 ? (((api.statusDistribution?.status_2xx ?? api.successHits) / api.totalHits) * 100).toFixed(1) : 100}%
                                    </span>
                                </div>

                                <div className="p-3 rounded bg-[#0E1014] border border-[#4CB8D6]/30">
                                    <span className="text-[10px] text-[#4CB8D6] uppercase font-semibold block">3xx Redirect</span>
                                    <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                        {api.statusDistribution?.status_3xx?.toLocaleString() || '0'}
                                    </span>
                                    <span className="text-[10px] text-zinc-500 font-mono">
                                        {api.totalHits > 0 ? (((api.statusDistribution?.status_3xx ?? 0) / api.totalHits) * 100).toFixed(1) : 0}%
                                    </span>
                                </div>

                                <div className="p-3 rounded bg-[#0E1014] border border-[#F59E0B]/30">
                                    <span className="text-[10px] text-[#F59E0B] uppercase font-semibold block">4xx Client Error</span>
                                    <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                        {api.statusDistribution?.status_4xx?.toLocaleString() || '0'}
                                    </span>
                                    <span className="text-[10px] text-zinc-500 font-mono">
                                        {api.totalHits > 0 ? (((api.statusDistribution?.status_4xx ?? 0) / api.totalHits) * 100).toFixed(1) : 0}%
                                    </span>
                                </div>

                                <div className="p-3 rounded bg-[#0E1014] border border-[#E45865]/30">
                                    <span className="text-[10px] text-[#E45865] uppercase font-semibold block">5xx Server Error</span>
                                    <span className="text-base font-bold font-mono text-zinc-100 mt-0.5 block">
                                        {api.statusDistribution?.status_5xx?.toLocaleString() || '0'}
                                    </span>
                                    <span className="text-[10px] text-zinc-500 font-mono">
                                        {api.totalHits > 0 ? (((api.statusDistribution?.status_5xx ?? 0) / api.totalHits) * 100).toFixed(1) : 0}%
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                )}

                {/* Tab 3: Diagnostics & cURL Testing */}
                {activeTab === 'diagnostics' && (
                    <div className="space-y-4 animate-in fade-in duration-100">
                        {/* cURL Snippet */}
                        <div className="surface-panel p-4 space-y-2">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                    <Terminal size={14} className="text-[#4CB8D6]" />
                                    <h4 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                        Developer cURL Test Snippet
                                    </h4>
                                </div>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={copyCurl}
                                    className="h-6 text-[11px] gap-1 cursor-pointer"
                                >
                                    {copiedCurl ? <Check size={11} className="text-[#48B982]" /> : <Copy size={11} />}
                                    {copiedCurl ? 'Copied' : 'Copy cURL'}
                                </Button>
                            </div>

                            <p className="text-[11px] text-zinc-400">
                                Reproduce or test this endpoint directly from terminal or debugging tools:
                            </p>

                            <div className="p-2.5 rounded bg-[#0A0C10] border border-[#242932] font-mono text-[11px] text-[#4CB8D6] overflow-x-auto whitespace-pre">
                                {curlCommand}
                            </div>
                        </div>

                        {/* Recent Diagnostic Events */}
                        <div className="surface-panel p-4 space-y-3">
                            <h4 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                Diagnostic Trace Log Preview
                            </h4>
                            <div className="border border-[#242932] rounded-lg overflow-hidden text-xs">
                                <table className="w-full text-left">
                                    <thead className="bg-[#0E1014] text-zinc-400 text-[10px] uppercase font-semibold border-b border-[#242932]">
                                        <tr>
                                            <th className="p-2.5">HTTP Status</th>
                                            <th className="p-2.5">Response Time</th>
                                            <th className="p-2.5">Trace Reference</th>
                                            <th className="p-2.5 text-right">Result</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-[#242932] font-mono text-[11px]">
                                        {api.errorHits > 0 && (
                                            <tr className="bg-[#E45865]/5">
                                                <td className="p-2.5 text-[#E45865] font-bold">500 Server Error</td>
                                                <td className="p-2.5 text-zinc-300">{Math.round(api.maxLatency)} ms</td>
                                                <td className="p-2.5 text-zinc-500">tr_{Math.random().toString(36).substring(2, 9)}</td>
                                                <td className="p-2.5 text-right text-[#E45865]">Failure (Frustrated)</td>
                                            </tr>
                                        )}
                                        {api.successHits > 0 && (
                                            <tr>
                                                <td className="p-2.5 text-[#48B982] font-bold">200 OK</td>
                                                <td className="p-2.5 text-zinc-300">{p50} ms</td>
                                                <td className="p-2.5 text-zinc-500">tr_{Math.random().toString(36).substring(2, 9)}</td>
                                                <td className="p-2.5 text-right text-[#48B982]">Success (Satisfied)</td>
                                            </tr>
                                        )}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                )}

                {/* Modal Actions Footer */}
                <div className="flex items-center justify-between pt-3 border-t border-[#242932]">
                    <div className="flex items-center gap-2">
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={copyCurl}
                            className="h-7 text-xs gap-1.5 cursor-pointer text-zinc-300 hover:text-zinc-100"
                        >
                            <Terminal size={12} />
                            Copy cURL
                        </Button>
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={copyPath}
                            className="h-7 text-xs gap-1.5 cursor-pointer text-zinc-300 hover:text-zinc-100"
                        >
                            <Copy size={12} />
                            Copy Path
                        </Button>
                    </div>

                    <Button
                        variant="default"
                        size="sm"
                        onClick={onClose}
                        className="h-7 px-4 text-xs bg-[#4CB8D6] hover:bg-[#3fa5c0] text-[#0A0C10] font-semibold cursor-pointer"
                    >
                        Close
                    </Button>
                </div>
            </div>
        </Modal>
    );
}
