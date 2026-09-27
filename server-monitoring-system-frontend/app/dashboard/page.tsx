"use client";

import { useMemo, useState } from 'react';
import { notFound } from 'next/navigation';
import Link from 'next/link';
import { useDashboardQuery } from '@/hooks/use-dashboard-queries';
import { useAuth } from '@/contexts/auth-context';
import { MetricStrip, MetricItem } from '@/components/ui/metric-strip';
import { StatusBadge, HealthDot } from '@/components/ui/status-badge';
import { TelemetryTimeSeriesChart } from '@/components/charts/requests-latency-chart';
import { StatusBreakdown } from '@/components/charts/status-breakdown';
import { Button } from '@/components/ui/button';
import { 
    Activity, 
    AlertTriangle, 
    AlertCircle, 
    ArrowUpRight, 
    CheckCircle2, 
    RefreshCw, 
    ChevronDown,
    Server,
    ExternalLink
} from 'lucide-react';

export default function OverviewDashboardPage() {
    const { user, loading } = useAuth();
    const [timeRange, setTimeRange] = useState<'1h' | '24h' | '7d'>('24h');

    // Route guard: super_admin does not have access to client metrics overview
    if (!loading && user && user.role === 'super_admin') {
        notFound();
    }

    const { startTime, endTime } = useMemo(() => {
        const now = new Date();
        const durationHours = timeRange === '1h' ? 1 : timeRange === '7d' ? 168 : 24;
        return {
            startTime: new Date(now.getTime() - durationHours * 60 * 60 * 1000).toISOString(),
            endTime: now.toISOString(),
        };
    }, [timeRange]);

    const { data, isPending, error, refetch } = useDashboardQuery(
        user?.clientId ? { clientId: user.clientId, startTime, endTime } : undefined,
        { enabled: user?.role !== 'super_admin' }
    );


    const stats = data?.data?.stats ?? null;
    const topEndpoints = data?.data?.topEndpoints ?? [];
    const rawTimeSeries = data?.data?.recentActivity ?? [];

    // Calculate latency percentiles
    const { p95Latency, fastestLatency } = useMemo(() => {
        if (!rawTimeSeries.length) return { p95Latency: 0, fastestLatency: 0 };
        const latencies = rawTimeSeries
            .map(m => typeof m.maxLatency === 'string' ? parseFloat(m.maxLatency) : Number(m.maxLatency ?? 0))
            .filter(l => l > 0);
        
        if (!latencies.length) return { p95Latency: 0, fastestLatency: 0 };
        return {
            p95Latency: Math.max(...latencies),
            fastestLatency: Math.min(...latencies),
        };
    }, [rawTimeSeries]);

    // Diagnostic health calculation
    const systemHealth = useMemo(() => {
        if (!stats || stats.totalHits === 0) {
            return {
                status: 'healthy' as const,
                healthyCount: stats?.uniqueServices || 0,
                degradedCount: 0,
                offlineCount: 0,
                summary: "All monitored services operational. Ready for ingestion traffic.",
            };
        }

        const errorRate = stats.errorRate || 0;
        const avgLat = stats.avgLatency || 0;
        const degradedServices: string[] = [];

        topEndpoints.forEach(ep => {
            const epErr = typeof ep.errorRate === 'string' ? parseFloat(ep.errorRate) : Number(ep.errorRate || 0);
            if (epErr > 5) {
                degradedServices.push(`${ep.serviceName} (${ep.endpoint})`);
            }
        });

        if (errorRate >= 20 || degradedServices.length > 0) {
            return {
                status: 'degraded' as const,
                healthyCount: Math.max(0, (stats.uniqueServices || 1) - 1),
                degradedCount: 1,
                offlineCount: 0,
                summary: degradedServices.length > 0
                    ? `Error rate elevated on ${degradedServices[0]} (${errorRate.toFixed(1)}% failures).`
                    : `System error rate is elevated at ${errorRate.toFixed(1)}%.`,
            };
        }

        return {
            status: 'healthy' as const,
            healthyCount: stats.uniqueServices || 1,
            degradedCount: 0,
            offlineCount: 0,
            summary: "All monitored API endpoints operating within standard latency SLOs.",
        };
    }, [stats, topEndpoints]);

    // KPI Strip items
    const metricItems: MetricItem[] = useMemo(() => {
        if (!stats) {
            return [
                { label: 'REQUESTS', value: '0', subtext: '0 req/min avg' },
                { label: 'ERROR RATE', value: '0.0%', status: 'neutral', subtext: '0 failed requests' },
                { label: 'P95 LATENCY', value: '0 ms', status: 'neutral', subtext: 'within threshold' },
                { label: 'AVAILABILITY', value: '100%', status: 'healthy', subtext: 'No downtime' },
            ];
        }

        const errRate = Number(stats.errorRate) || 0;
        const total = stats.totalHits || 0;
        const p95 = p95Latency || stats.avgLatency || 0;
        const availability = total > 0 ? (stats.successHits / total) * 100 : 100;

        return [
            {
                label: 'REQUESTS',
                value: total > 999 ? `${(total / 1000).toFixed(1)}k` : total.toString(),
                subtext: `${stats.uniqueEndpoints || 0} active routes across ${stats.uniqueServices || 0} services`,
            },
            {
                label: 'ERROR RATE',
                value: `${errRate.toFixed(1)}%`,
                status: errRate === 0 ? 'healthy' : errRate > 15 ? 'critical' : 'warning',
                subtext: `${stats.errorHits || 0} failed of ${total} total requests`,
                trend: errRate > 0 ? `↑ ${errRate.toFixed(1)}%` : '— 0.0%',
            },
            {
                label: 'P95 LATENCY',
                value: `${Math.round(p95)} ms`,
                status: p95 > 500 ? 'warning' : 'neutral',
                subtext: `Avg latency ${Math.round(stats.avgLatency || 0)} ms`,
            },
            {
                label: 'AVAILABILITY',
                value: total > 0 ? `${availability.toFixed(2)}%` : '—',
                status: availability >= 99.5 ? 'healthy' : 'warning',
                subtext: total > 0 ? 'Last 24 hours window' : 'Insufficient telemetry data',
            },
        ];
    }, [stats, p95Latency]);

    // Attention Feed / Active Alerts
    const attentionAlerts = useMemo(() => {
        const alerts: Array<{ id: string; severity: 'critical' | 'warning' | 'info'; title: string; detail: string; service: string; time: string }> = [];

        topEndpoints.forEach((ep, idx) => {
            const errRate = typeof ep.errorRate === 'string' ? parseFloat(ep.errorRate) : Number(ep.errorRate || 0);
            const avgLat = typeof ep.avgLatency === 'string' ? parseFloat(ep.avgLatency) : Number(ep.avgLatency || 0);
            const errCount = Math.round((ep.totalHits * errRate) / 100);

            if (errRate >= 20) {
                alerts.push({
                    id: `err-${idx}`,
                    severity: 'critical',
                    title: `High Error Rate (${errRate.toFixed(1)}%)`,
                    detail: `${ep.serviceName} • ${ep.method} ${ep.endpoint} has ${errCount} failure(s) of ${ep.totalHits} hits.`,
                    service: ep.serviceName,
                    time: 'Active now',
                });
            } else if (avgLat >= 400) {
                alerts.push({
                    id: `lat-${idx}`,
                    severity: 'warning',
                    title: `Elevated Latency (${Math.round(avgLat)}ms)`,
                    detail: `${ep.serviceName} • ${ep.method} ${ep.endpoint} exceeded latency target.`,
                    service: ep.serviceName,
                    time: 'Recent',
                });
            }
        });

        return alerts;
    }, [topEndpoints]);

    if (isPending) {
        return (
            <div className="h-[60vh] flex flex-col items-center justify-center gap-2.5 text-zinc-400">
                <RefreshCw className="animate-spin text-[#4CB8D6] w-5 h-5" />
                <p className="text-xs font-medium">Loading telemetry overview...</p>
            </div>
        );
    }

    if (error) {
        return (
            <div className="surface-panel p-8 text-center max-w-md mx-auto my-12 space-y-4">
                <div className="w-10 h-10 rounded-full bg-[#E45865]/10 text-[#E45865] flex items-center justify-center mx-auto">
                    <AlertCircle size={20} />
                </div>
                <div className="space-y-1">
                    <h3 className="font-semibold text-sm text-zinc-100">Telemetry Stream Disconnected</h3>
                    <p className="text-xs text-zinc-400">Unable to load metrics from the analytics backend service.</p>
                </div>
                <Button variant="outline" size="sm" onClick={() => refetch()} className="text-xs h-8 gap-1.5 cursor-pointer">
                    <RefreshCw size={12} />
                    Retry Connection
                </Button>
            </div>
        );
    }

    return (
        <div className="space-y-6 max-w-7xl mx-auto pb-12 animate-in fade-in duration-150">
            {/* Page Header */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-2 border-b border-[#242932]">
                <div>
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                        Overview
                    </h1>
                    <p className="text-xs text-zinc-400 mt-0.5">
                        Operational health, throughput, and latency across your production environment.
                    </p>
                </div>

                {/* Time Range Selector */}
                <div className="flex items-center gap-2">
                    <div className="relative">
                        <select
                            value={timeRange}
                            onChange={(e: any) => setTimeRange(e.target.value)}
                            className="bg-[#111419] border border-[#242932] text-xs font-medium text-zinc-300 rounded px-2.5 py-1.5 pr-6 appearance-none outline-none hover:border-[#323946] focus:border-[#4CB8D6] transition-colors cursor-pointer select-none"
                        >
                            <option value="1h">Last 1 hour</option>
                            <option value="24h">Last 24 hours</option>
                            <option value="7d">Last 7 days</option>
                        </select>
                        <ChevronDown size={12} className="absolute right-2 top-2.5 text-zinc-400 pointer-events-none" />
                    </div>
                </div>
            </div>

            {/* System Health Diagnostic Module */}
            <div className="surface-panel p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                <div className="space-y-1">
                    <div className="flex items-center gap-3">
                        <span className="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">
                            System Health
                        </span>
                        <span className="text-zinc-600">•</span>
                        <div className="flex items-center gap-2.5 text-xs">
                            <span className="inline-flex items-center gap-1 text-zinc-300 font-mono">
                                <span className="w-2 h-2 rounded-full bg-[#48B982]" /> {systemHealth.healthyCount} Healthy
                            </span>
                            {systemHealth.degradedCount > 0 && (
                                <span className="inline-flex items-center gap-1 text-[#D99A3D] font-mono">
                                    <span className="w-2 h-2 rounded-full bg-[#D99A3D]" /> {systemHealth.degradedCount} Degraded
                                </span>
                            )}
                            <span className="inline-flex items-center gap-1 text-zinc-500 font-mono">
                                <span className="w-2 h-2 rounded-full bg-zinc-600" /> {systemHealth.offlineCount} Offline
                            </span>
                        </div>
                    </div>
                    <p className="text-xs text-zinc-300">
                        {systemHealth.summary}
                    </p>
                </div>

                <Link href="/dashboard/apis">
                    <Button variant="outline" size="sm" className="text-xs h-7 gap-1 shrink-0 cursor-pointer">
                        View API Routes
                        <ArrowUpRight size={12} />
                    </Button>
                </Link>
            </div>

            {/* Continuous Single-Strip KPI Row */}
            <MetricStrip items={metricItems} />

            {/* Telemetry Charts: Traffic & Latency */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
                <TelemetryTimeSeriesChart
                    data={rawTimeSeries}
                    type="requests"
                    title="Request Traffic & Error Count"
                />
                <TelemetryTimeSeriesChart
                    data={rawTimeSeries}
                    type="latency"
                    title="Latency Profile (p50 / p95)"
                />
            </div>

            {/* Requires Attention Section (Alerts Feed) */}
            {attentionAlerts.length > 0 && (
                <div className="surface-panel p-4 space-y-3">
                    <div className="flex items-center justify-between border-b border-[#242932] pb-2">
                        <div className="flex items-center gap-2">
                            <AlertTriangle size={14} className="text-[#D99A3D]" />
                            <span className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                                Requires Attention ({attentionAlerts.length})
                            </span>
                        </div>
                        <span className="text-[11px] text-zinc-500 font-mono">Telemetry SLO Engine</span>
                    </div>

                    <div className="divide-y divide-[#242932]">
                        {attentionAlerts.map((alert) => (
                            <div key={alert.id} className="py-2.5 flex items-start justify-between gap-4 text-xs">
                                <div className="space-y-0.5 min-w-0">
                                    <div className="flex items-center gap-2">
                                        <span className={`text-[10px] font-mono uppercase px-1.5 py-0.5 rounded font-bold ${
                                            alert.severity === 'critical' ? 'bg-[#E45865]/10 text-[#E45865] border border-[#E45865]/20' : 'bg-[#D99A3D]/10 text-[#D99A3D] border border-[#D99A3D]/20'
                                        }`}>
                                            {alert.severity}
                                        </span>
                                        <span className="font-semibold text-zinc-200">{alert.title}</span>
                                    </div>
                                    <p className="text-zinc-400 text-[11px] truncate">{alert.detail}</p>
                                </div>
                                <span className="text-zinc-500 font-mono text-[10px] shrink-0">{alert.time}</span>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            {/* Top Endpoints & Services Table */}
            <div className="surface-panel p-0 overflow-hidden">
                <div className="p-4 border-b border-[#242932] flex items-center justify-between">
                    <div>
                        <h3 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider">
                            Active Monitored Endpoints
                        </h3>
                        <p className="text-[11px] text-zinc-400 mt-0.5">
                            Highest throughput endpoints and failure distributions
                        </p>
                    </div>
                    <Link href="/dashboard/apis" className="text-xs text-[#4CB8D6] hover:underline flex items-center gap-1 font-medium">
                        View all routes
                        <ArrowUpRight size={12} />
                    </Link>
                </div>

                {topEndpoints.length === 0 ? (
                    <div className="text-center py-10 p-4">
                        <Server className="w-7 h-7 text-zinc-600 mx-auto mb-1.5" />
                        <p className="text-xs font-semibold text-zinc-300">No Endpoint Telemetry</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mx-auto mt-0.5">
                            Endpoints will populate automatically as your microservices emit HTTP trace events.
                        </p>
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-xs border-collapse">
                            <thead>
                                <tr className="border-b border-[#242932] text-zinc-400 text-[11px] uppercase tracking-wider bg-[#0E1014]/50">
                                    <th className="py-2.5 px-4 font-semibold">Status</th>
                                    <th className="py-2.5 px-4 font-semibold">Route</th>
                                    <th className="py-2.5 px-4 font-semibold">Service</th>
                                    <th className="py-2.5 px-4 font-semibold text-right">Requests</th>
                                    <th className="py-2.5 px-4 font-semibold text-right">Avg Latency</th>
                                    <th className="py-2.5 px-4 font-semibold text-right">Error Rate</th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[#242932]">
                                {topEndpoints.map((ep, idx) => {
                                    const errRate = typeof ep.errorRate === 'string' ? parseFloat(ep.errorRate) : Number(ep.errorRate || 0);
                                    const avgLat = typeof ep.avgLatency === 'string' ? parseFloat(ep.avgLatency) : Number(ep.avgLatency || 0);
                                    const isDegraded = errRate > 5 || avgLat > 400;

                                    return (
                                        <tr key={idx} className="hover:bg-[#181D24] transition-colors">
                                            <td className="py-3 px-4 whitespace-nowrap">
                                                <StatusBadge status={isDegraded ? 'degraded' : 'healthy'} />
                                            </td>
                                            <td className="py-3 px-4 font-mono font-medium text-zinc-200 whitespace-nowrap">
                                                <span className="text-zinc-500 font-bold mr-1.5">{ep.method}</span>
                                                {ep.endpoint}
                                            </td>
                                            <td className="py-3 px-4 text-zinc-400 whitespace-nowrap">
                                                {ep.serviceName}
                                            </td>
                                            <td className="py-3 px-4 text-right font-mono text-zinc-200">
                                                {ep.totalHits}
                                            </td>
                                            <td className="py-3 px-4 text-right font-mono text-zinc-300">
                                                {Math.round(avgLat)} ms
                                            </td>
                                            <td className={`py-3 px-4 text-right font-mono font-semibold ${errRate > 0 ? 'text-[#E45865]' : 'text-[#48B982]'}`}>
                                                {errRate.toFixed(1)}%
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>
        </div>
    );
}
