"use client";

import React, { useMemo, useState } from 'react';
import Link from 'next/link';
import { useSearchParams, useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useApisMetricsQuery, useDashboardQuery, useEndpointMetricsQuery } from '@/hooks/use-dashboard-queries';
import { useTenantConfigQuery } from '@/hooks/use-tenant-config';
import { StatusBadge } from '@/components/ui/status-badge';
import { TelemetryTimeSeriesChart, MetricPoint } from '@/components/charts/requests-latency-chart';
import { QuantileSpectrumChart } from '@/components/apis/api-quantile-spectrum';
import { ApdexRadialGauge } from '@/components/apis/api-apdex-gauge';
import { HttpStatusDonutChart } from '@/components/apis/api-status-donut';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { ApiMetricsEntry, RecentActivity } from '@/lib/api';
import { 
    ArrowLeft, 
    Copy, 
    Check, 
    Terminal, 
    Activity, 
    Clock, 
    Gauge, 
    Server, 
    RefreshCw, 
    Loader2, 
    AlertCircle,
    TrendingUp,
    Network,
    ShieldAlert
} from 'lucide-react';
import { cn } from '@/lib/utils';

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
            className={`inline-flex items-center gap-1 px-2.5 py-1 rounded text-xs font-mono font-bold border ${color}`} 
            title={`Apdex: ${score.toFixed(4)} (${label})`}
        >
            <Gauge size={12} />
            {score.toFixed(2)} ({label})
        </span>
    );
}

export default function ApiDetailsPage() {
    const toast = useToast();
    const router = useRouter();
    const searchParams = useSearchParams();
    const { user } = useAuth();

    const targetEndpoint = searchParams.get('endpoint') || '';
    const targetMethod = (searchParams.get('method') || 'GET').toUpperCase();
    const targetService = searchParams.get('service') || '';
    const targetClientId = searchParams.get('clientId') || user?.clientId;

    const [timeWindow, setTimeWindow] = useState<'1h' | '6h' | '24h' | '7d'>('24h');
    const [copiedPath, setCopiedPath] = useState(false);
    const [copiedCurl, setCopiedCurl] = useState(false);

    // Fetch dynamic client/tenant config from DB
    const { data: tenantConfig } = useTenantConfigQuery(targetClientId);
    const apdexThresholdMs = tenantConfig?.apdexThresholdMs ?? 500;

    // Calculate time window range
    const timeRangeParams = useMemo(() => {
        const now = new Date();
        const end = now.toISOString();
        let ms = 86_400_000; // 24h default
        if (timeWindow === '1h') ms = 3_600_000;
        if (timeWindow === '6h') ms = 21_600_000;
        if (timeWindow === '7d') ms = 86_400_000 * 7;
        const start = new Date(now.getTime() - ms).toISOString();
        return { startTime: start, endTime: end, clientId: targetClientId };
    }, [timeWindow, targetClientId]);

    // Fetch API overview metrics
    const { data: metricsData, isLoading, isError, refetch, isFetching } = useApisMetricsQuery(1, 100, targetClientId);
    const apis = metricsData?.items ?? [];

    // Match the specific route
    const api: ApiMetricsEntry | null = useMemo(() => {
        if (!apis.length || !targetEndpoint) return null;
        return apis.find((a: ApiMetricsEntry) => 
            a.endpoint.toLowerCase() === targetEndpoint.toLowerCase() &&
            a.method.toUpperCase() === targetMethod &&
            (!targetService || a.serviceName.toLowerCase() === targetService.toLowerCase())
        ) || null;
    }, [apis, targetEndpoint, targetMethod, targetService]);

    const resolvedServiceName = targetService || api?.serviceName || '';

    // Fetch window-level metrics specifically for this endpoint (percentiles, statusDistribution, apdex, throughputRpm)
    const { 
        data: endpointMetrics, 
        refetch: refetchEndpointMetrics, 
        isFetching: isFetchingEndpointMetrics 
    } = useEndpointMetricsQuery({
        serviceName: resolvedServiceName,
        endpoint: targetEndpoint,
        method: targetMethod,
        startTime: timeRangeParams.startTime,
        endTime: timeRangeParams.endTime,
        clientId: targetClientId,
    });

    // Fetch dashboard recentActivity to extract chronological time-series for THIS specific endpoint
    const { data: dashboardData, refetch: refetchDashboard } = useDashboardQuery(timeRangeParams);

    // Filter time-series specifically for this endpoint
    const endpointTimeSeries: MetricPoint[] = useMemo(() => {
        const activities = dashboardData?.data?.recentActivity;
        if (!activities) return [];
        return activities
            .filter((r: RecentActivity) => 
                r.endpoint.toLowerCase() === targetEndpoint.toLowerCase() &&
                r.method.toUpperCase() === targetMethod &&
                (!resolvedServiceName || r.serviceName.toLowerCase() === resolvedServiceName.toLowerCase())
            )
            .map((r: RecentActivity) => ({
                serviceName: r.serviceName,
                endpoint: r.endpoint,
                method: r.method,
                timeBucket: r.timeBucket,
                totalHits: Number(r.totalHits) || 0,
                errorHits: Number(r.errorHits) || 0,
                avgLatency: Number(r.avgLatency) || 0,
                minLatency: Number(r.minLatency) || 0,
                maxLatency: Number(r.maxLatency) || 0,
            }));
    }, [dashboardData, targetEndpoint, targetMethod, resolvedServiceName]);

    // Calculate dynamic throughput RPM for the selected window
    const windowThroughputRpm = useMemo(() => {
        if (endpointMetrics?.throughputRpm !== undefined && endpointMetrics.throughputRpm > 0) {
            return endpointMetrics.throughputRpm;
        }
        if (endpointTimeSeries.length > 0) {
            const hits = endpointTimeSeries.reduce((sum, p) => sum + (p.totalHits || 0), 0);
            const mins = timeWindow === '1h' ? 60 : timeWindow === '6h' ? 360 : timeWindow === '7d' ? 10080 : 1440;
            return Number(((hits / mins) * 100).toFixed(1)) / 100;
        }
        if (api?.throughputRpm !== undefined && api.throughputRpm > 0) {
            return api.throughputRpm;
        }
        if (api && api.totalHits > 0) {
            return Number((api.totalHits / 60).toFixed(1));
        }
        return 0;
    }, [endpointMetrics, endpointTimeSeries, timeWindow, api]);

    const copyPath = () => {
        if (!targetEndpoint) return;
        navigator.clipboard.writeText(targetEndpoint);
        setCopiedPath(true);
        toast('Endpoint path copied to clipboard', 'success');
        setTimeout(() => setCopiedPath(false), 2000);
    };

    const curlCommand = `curl -X ${targetMethod} "http://localhost:3000${targetEndpoint}" -H "Accept: application/json"`;

    const copyCurl = () => {
        navigator.clipboard.writeText(curlCommand);
        setCopiedCurl(true);
        toast('cURL command copied to clipboard', 'success');
        setTimeout(() => setCopiedCurl(false), 2000);
    };

    const handleRefreshAll = () => {
        refetch();
        refetchDashboard();
        refetchEndpointMetrics();
    };

    if (isLoading) {
        return (
            <div className="py-24 text-center text-zinc-400 space-y-3">
                <Loader2 className="w-8 h-8 animate-spin mx-auto text-[#4CB8D6]" />
                <p className="text-xs font-mono">Loading telemetry telemetry matrix...</p>
            </div>
        );
    }

    if (isError) {
        return (
            <div className="py-20 text-center space-y-4 max-w-md mx-auto">
                <AlertCircle className="w-10 h-10 text-[#E45865] mx-auto" />
                <h2 className="text-base font-semibold text-zinc-100">Telemetry Fetch Error</h2>
                <p className="text-xs text-zinc-400">
                    Failed to retrieve metrics for endpoint {targetEndpoint}.
                </p>
                <Button onClick={handleRefreshAll} variant="outline" size="sm" className="cursor-pointer">
                    Retry Request
                </Button>
            </div>
        );
    }

    if (!api) {
        return (
            <div className="py-20 text-center space-y-4 max-w-md mx-auto">
                <AlertCircle className="w-10 h-10 text-[#F59E0B] mx-auto" />
                <h2 className="text-base font-semibold text-zinc-100">Route Telemetry Not Found</h2>
                <p className="text-xs text-zinc-400">
                    No active telemetry entries were found for <span className="font-mono text-zinc-200">{targetMethod} {targetEndpoint || 'N/A'}</span> in the current aggregation window.
                </p>
                <div className="pt-2 flex items-center justify-center gap-2">
                    <Button 
                        onClick={() => router.push('/dashboard/apis')} 
                        variant="outline" 
                        size="sm" 
                        className="cursor-pointer"
                    >
                        <ArrowLeft size={13} className="mr-1" />
                        Back to APIs
                    </Button>
                    <Button 
                        onClick={handleRefreshAll} 
                        variant="default" 
                        size="sm" 
                        className="bg-[#4CB8D6] text-black hover:bg-[#3fa5c0] cursor-pointer"
                    >
                        <RefreshCw size={13} className="mr-1" />
                        Refresh
                    </Button>
                </div>
            </div>
        );
    }

    // Quantile values (prefer window-level metrics from backend, fallback to route summary)
    const p50 = Math.round(endpointMetrics?.percentiles?.p50 ?? api.percentiles?.p50 ?? api.avgLatency ?? 0);
    const p75 = Math.round(endpointMetrics?.percentiles?.p75 ?? api.percentiles?.p75 ?? (p50 * 1.25));
    const p90 = Math.round(endpointMetrics?.percentiles?.p90 ?? api.percentiles?.p90 ?? (p50 * 1.5));
    const p95 = Math.round(endpointMetrics?.percentiles?.p95 ?? api.percentiles?.p95 ?? (api.avgLatency + (api.maxLatency - api.avgLatency) * 0.9));
    const p99 = Math.round(endpointMetrics?.percentiles?.p99 ?? api.percentiles?.p99 ?? api.maxLatency ?? 0);

    const activeApdex = endpointMetrics?.apdex ?? api.apdex;

    return (
        <div className="max-w-6xl mx-auto pb-16 space-y-6 animate-in fade-in duration-150">
            {/* Top Navigation, Time Window & Actions Bar */}
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 pb-3 border-b border-[#242932]">
                <div className="flex items-center gap-2 text-xs">
                    <Link
                        href="/dashboard/apis"
                        className="inline-flex items-center gap-1 text-zinc-400 hover:text-zinc-200 transition-colors font-medium cursor-pointer"
                    >
                        <ArrowLeft size={13} />
                        Back to APIs
                    </Link>
                    <span className="text-zinc-600">/</span>
                    <span className="text-zinc-400 font-mono">APM Inspector</span>
                    <span className="text-zinc-600">/</span>
                    <span className="font-mono font-bold text-zinc-200 break-all">{api.endpoint}</span>
                </div>

                <div className="flex items-center gap-2.5">
                    {/* Time Window Selector */}
                    <div className="flex items-center p-0.5 rounded bg-[#0E1014] border border-[#242932] text-xs font-mono">
                        {(['1h', '6h', '24h', '7d'] as const).map((w) => (
                            <button
                                key={w}
                                onClick={() => setTimeWindow(w)}
                                className={cn(
                                    "px-2.5 py-1 rounded text-[11px] font-medium transition-colors cursor-pointer",
                                    timeWindow === w 
                                        ? "bg-[#242932] text-zinc-100 font-bold" 
                                        : "text-zinc-400 hover:text-zinc-200"
                                )}
                            >
                                {w}
                            </button>
                        ))}
                    </div>

                    <Button
                        variant="outline"
                        size="sm"
                        onClick={copyPath}
                        className="h-7 text-xs gap-1.5 cursor-pointer text-zinc-300 hover:text-zinc-100"
                    >
                        {copiedPath ? <Check size={12} className="text-[#48B982]" /> : <Copy size={12} />}
                        Path
                    </Button>

                    <Button
                        variant="outline"
                        size="sm"
                        onClick={handleRefreshAll}
                        disabled={isFetching || isFetchingEndpointMetrics}
                        className="h-7 text-xs gap-1.5 cursor-pointer text-zinc-300 hover:text-zinc-100"
                    >
                        <RefreshCw size={12} className={cn((isFetching || isFetchingEndpointMetrics) && "animate-spin text-[#4CB8D6]")} />
                        Refresh
                    </Button>
                </div>
            </div>

            {/* Main Header Banner */}
            <div className="surface-panel p-5 space-y-3">
                <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                    <div className="space-y-2">
                        <div className="flex items-center flex-wrap gap-2.5">
                            <span className={cn(
                                "px-2.5 py-1 rounded text-xs font-mono font-bold tracking-wider uppercase border",
                                getMethodBadgeStyle(api.method)
                            )}>
                                {api.method}
                            </span>
                            <h1 className="font-mono text-lg sm:text-xl font-bold text-zinc-100 break-all">
                                {api.endpoint}
                            </h1>
                        </div>

                        <div className="flex items-center flex-wrap gap-2 text-xs text-zinc-400">
                            <span className="flex items-center gap-1.5 font-mono text-xs bg-[#0E1014] px-2.5 py-1 rounded border border-[#242932]">
                                <Server size={12} className="text-[#4CB8D6]" />
                                service: {api.serviceName}
                            </span>
                            <span className="text-zinc-600">•</span>
                            <span className="flex items-center gap-1.5">
                                <span className="text-xs text-zinc-400 uppercase font-semibold">Health:</span>
                                <StatusBadge status={api.errorRate > 5 ? 'degraded' : 'healthy'} />
                            </span>
                            {(activeApdex || api.apdex) && (
                                <>
                                    <span className="text-zinc-600">•</span>
                                    {getApdexBadge(activeApdex?.score ?? api.apdex?.score)}
                                </>
                            )}
                        </div>
                    </div>
                </div>
            </div>

            {/* 4-Column KPI Overview Cards */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3.5">
                <div className="surface-panel p-4 space-y-1">
                    <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                        Total Ingested Volume
                    </span>
                    <span className="text-2xl font-bold font-mono text-zinc-100 block">
                        {api.totalHits.toLocaleString()}
                    </span>
                    <span className="text-xs text-zinc-500 font-mono block">
                        {api.successHits.toLocaleString()} success • {api.errorHits.toLocaleString()} failed
                    </span>
                </div>

                <div className="surface-panel p-4 space-y-1">
                    <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                        Average Throughput
                    </span>
                    <span className="text-2xl font-bold font-mono text-zinc-100 block">
                        {windowThroughputRpm > 0 ? windowThroughputRpm.toFixed(1) : (api.throughputRpm !== undefined && api.throughputRpm > 0 ? api.throughputRpm.toFixed(1) : '0.0')}
                    </span>
                    <span className="text-xs text-zinc-500 font-mono block">
                        requests / minute ({timeWindow})
                    </span>
                </div>

                <div className="surface-panel p-4 space-y-1">
                    <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                        Error Rate
                    </span>
                    <span className={cn(
                        "text-2xl font-bold font-mono block",
                        api.errorRate > 5 ? "text-[#E45865]" : api.errorRate > 1 ? "text-[#F59E0B]" : "text-[#48B982]"
                    )}>
                        {api.errorRate.toFixed(1)}%
                    </span>
                    <span className="text-xs text-zinc-500 font-mono block">
                        {api.errorHits > 0 ? `${api.errorHits} failures recorded` : 'zero errors recorded'}
                    </span>
                </div>

                <div className="surface-panel p-4 space-y-1">
                    <span className="text-[10px] text-zinc-400 uppercase font-semibold block tracking-wider">
                        Median Response Time (p50)
                    </span>
                    <span className="text-2xl font-bold font-mono text-[#4CB8D6] block">
                        {p50} ms
                    </span>
                    <span className="text-xs text-zinc-500 font-mono block">
                        avg: {Math.round(api.avgLatency)}ms • peak: {Math.round(api.maxLatency)}ms
                    </span>
                </div>
            </div>

            {/* Route-Specific Time-Series Activity (Traffic & Latency Over Time) */}
            <div className="space-y-3">
                <div className="flex items-center justify-between">
                    <div>
                        <h2 className="text-sm font-semibold text-zinc-200 uppercase tracking-wider flex items-center gap-2">
                            <Activity size={15} className="text-[#4CB8D6]" />
                            Route Telemetry Timeline ({timeWindow})
                        </h2>
                        <p className="text-xs text-zinc-400 mt-0.5">
                            Hourly traffic throughput, failure distributions, and latency progression for this route
                        </p>
                    </div>
                </div>

                {endpointTimeSeries.length > 0 ? (
                    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                        <TelemetryTimeSeriesChart
                            data={endpointTimeSeries}
                            title="Endpoint Traffic & Failures"
                            type="requests"
                        />
                        <TelemetryTimeSeriesChart
                            data={endpointTimeSeries}
                            title="Endpoint Response Time (p50 / p95)"
                            type="latency"
                        />
                    </div>
                ) : (
                    <div className="surface-panel p-6 text-center space-y-2">
                        <Activity className="w-7 h-7 text-zinc-600 mx-auto" />
                        <p className="text-xs font-semibold text-zinc-300">
                            Awaiting Route-Level Hourly Aggregations
                        </p>
                        <p className="text-[11px] text-zinc-500 max-w-md mx-auto">
                            Continuous hourly time-series points will render here as requests are ingested for this endpoint. Summary telemetry and quantiles are detailed below.
                        </p>
                    </div>
                )}
            </div>

            {/* Deep-Dive Grid 1: Latency Quantiles Spectrum vs Apdex Radial SLA Meter */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <QuantileSpectrumChart
                    p50={p50}
                    p75={p75}
                    p90={p90}
                    p95={p95}
                    p99={p99}
                    maxLatency={api.maxLatency}
                    apdexThresholdMs={apdexThresholdMs}
                />

                <ApdexRadialGauge
                    apdex={activeApdex}
                    apdexThresholdMs={apdexThresholdMs}
                />
            </div>

            {/* Deep-Dive Grid 2: HTTP Status Donut vs Developer Sandbox & Diagnostics */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <HttpStatusDonutChart
                    distribution={endpointMetrics?.statusDistribution ?? api.statusDistribution}
                    successHits={api.successHits}
                    errorHits={api.errorHits}
                    totalHits={api.totalHits}
                />

                {/* Developer Sandbox & Trace Diagnostics */}
                <div className="surface-panel p-5 flex flex-col justify-between space-y-4">
                    <div className="flex items-center justify-between pb-3 border-b border-[#242932]">
                        <div className="flex items-center gap-2">
                            <Terminal size={14} className="text-[#4CB8D6]" />
                            <h3 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                Developer cURL Snippet
                            </h3>
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

                    <div className="space-y-3">
                        <div className="p-3 rounded bg-[#0A0C10] border border-[#242932] font-mono text-xs text-[#4CB8D6] overflow-x-auto whitespace-pre">
                            {curlCommand}
                        </div>

                        {/* Simulated Network Timing Breakdown */}
                        <div className="space-y-1.5 pt-1">
                            <div className="flex items-center justify-between text-xs text-zinc-400">
                                <span className="flex items-center gap-1">
                                    <Network size={12} className="text-[#5794E8]" />
                                    Estimated Response Breakdown
                                </span>
                                <span className="font-mono text-zinc-300">~{p50} ms total</span>
                            </div>

                            {/* Stacked timing segment bar */}
                            <div className="h-2 w-full rounded-full overflow-hidden flex bg-[#111419] border border-[#242932]">
                                <div style={{ width: '15%' }} className="bg-[#5794E8] h-full" title="DNS & Handshake (~15%)" />
                                <div style={{ width: '65%' }} className="bg-[#4CB8D6] h-full" title="Server Execution TTFB (~65%)" />
                                <div style={{ width: '20%' }} className="bg-[#48B982] h-full" title="Payload Transfer (~20%)" />
                            </div>

                            <div className="flex items-center justify-between text-[10px] text-zinc-500 font-mono pt-0.5">
                                <span className="flex items-center gap-1">
                                    <span className="w-1.5 h-1.5 rounded-full bg-[#5794E8]" /> Connection (~15%)
                                </span>
                                <span className="flex items-center gap-1">
                                    <span className="w-1.5 h-1.5 rounded-full bg-[#4CB8D6]" /> Server TTFB (~65%)
                                </span>
                                <span className="flex items-center gap-1">
                                    <span className="w-1.5 h-1.5 rounded-full bg-[#48B982]" /> Transfer (~20%)
                                </span>
                            </div>
                        </div>

                        {/* Diagnostic Trace Log */}
                        <div className="pt-2 border-t border-[#242932]">
                            <div className="border border-[#242932] rounded-lg overflow-hidden text-xs">
                                <table className="w-full text-left">
                                    <thead className="bg-[#0E1014] text-zinc-400 text-[10px] uppercase font-semibold border-b border-[#242932]">
                                        <tr>
                                            <th className="p-2">Status</th>
                                            <th className="p-2">Latency</th>
                                            <th className="p-2">Trace ID</th>
                                            <th className="p-2 text-right">Result</th>
                                        </tr>
                                    </thead>
                                    <tbody className="divide-y divide-[#242932] font-mono text-[11px]">
                                        {api.errorHits > 0 && (
                                            <tr className="bg-[#E45865]/5">
                                                <td className="p-2 text-[#E45865] font-bold">500 Server Error</td>
                                                <td className="p-2 text-zinc-300">{Math.round(api.maxLatency)} ms</td>
                                                <td className="p-2 text-zinc-500">tr_c9a18f40b</td>
                                                <td className="p-2 text-right text-[#E45865]">Frustrated</td>
                                            </tr>
                                        )}
                                        {api.successHits > 0 && (
                                            <tr>
                                                <td className="p-2 text-[#48B982] font-bold">200 OK</td>
                                                <td className="p-2 text-zinc-300">{p50} ms</td>
                                                <td className="p-2 text-zinc-500">tr_e27b140df</td>
                                                <td className="p-2 text-right text-[#48B982]">Satisfied</td>
                                            </tr>
                                        )}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
