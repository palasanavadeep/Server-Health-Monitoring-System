"use client";

import { useMemo } from 'react';
import { notFound } from 'next/navigation';
import { useDashboardQuery } from '@/hooks/use-dashboard-queries';
import { StatsGrid } from '@/components/dashboard/stats-grid';
import { TopEndpoints } from '@/components/dashboard/top-endpoints';
import { ApiHitsChart } from '@/components/charts/api-hits-chart';
import { StatusDistributionChart } from '@/components/charts/status-pie-chart';
import { Button } from '@/components/ui/button';
import { RefreshCw, AlertCircle, Sparkles } from 'lucide-react';
import { useAuth } from '@/contexts/auth-context';

export default function OverviewDashboardPage() {
    const { user, loading } = useAuth();

    // Route guard: super_admin does not have access to client metrics overview
    if (!loading && user && user.role === 'super_admin') {
        notFound();
    }

    const { data, isPending, error, refetch } = useDashboardQuery(
        user?.clientId ? { clientId: user.clientId } : undefined,
        { enabled: user?.role !== 'super_admin' }
    );

    const stats = data?.data?.stats ?? null;
    const topEndpoints = data?.data?.topEndpoints ?? [];
    const rawTimeSeries = data?.data?.recentActivity ?? [];

    // Extract Peak and Minimum latency from timeseries buckets
    const peakLatency = useMemo(() => {
        if (!rawTimeSeries.length) return 0;
        const latencies = rawTimeSeries.map(m => typeof m.maxLatency === 'string' ? parseFloat(m.maxLatency) : Number(m.maxLatency ?? 0));
        return Math.max(...latencies, 0);
    }, [rawTimeSeries]);

    const minLatency = useMemo(() => {
        if (!rawTimeSeries.length) return 0;
        const latencies = rawTimeSeries
            .map(m => typeof m.minLatency === 'string' ? parseFloat(m.minLatency) : Number(m.minLatency ?? 0))
            .filter(lat => lat > 0);
        return latencies.length ? Math.min(...latencies) : 0;
    }, [rawTimeSeries]);

    const statusData = useMemo(() => {
        if (!stats) return null;
        return {
            labels: ['Success (2xx)', 'Errors (4xx/5xx)'],
            values: [stats.successHits, stats.errorHits],
        };
    }, [stats]);

    if (isPending) {
        return (
            <div className="h-[60vh] flex flex-col items-center justify-center gap-3">
                <RefreshCw className="animate-spin text-cyan-500 w-8 h-8" />
                <p className="text-sm font-medium text-muted-foreground animate-pulse">Loading dashboard telemetry...</p>
            </div>
        );
    }

    if (error) {
        return (
            <div className="h-[60vh] flex flex-col items-center justify-center gap-4 text-center p-6 glass-panel rounded-2xl max-w-md mx-auto my-12 border-rose-500/20 shadow-xl shadow-rose-500/5">
                <div className="flex items-center justify-center w-12 h-12 rounded-full bg-rose-500/10 text-rose-500 border border-rose-500/20">
                    <AlertCircle size={24} />
                </div>
                <div className="space-y-1">
                    <h3 className="font-bold text-lg text-foreground">Sync Error</h3>
                    <p className="text-sm text-muted-foreground">Failed to connect to telemetry streaming service.</p>
                </div>
                <Button variant="outline" size="sm" onClick={() => refetch()} className="gap-2 cursor-pointer">
                    <RefreshCw size={13} />
                    Retry Connection
                </Button>
            </div>
        );
    }

    return (
        <div className="space-y-8 max-w-7xl mx-auto pb-12 animate-in fade-in duration-300">
            {/* Header Greeting */}
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-border-color/30 pb-6">
                <div>
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                        System Metrics
                        <Sparkles size={20} className="text-cyan-400 animate-pulse-slow" />
                    </h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Welcome back, <span className="font-bold text-foreground">{user?.username || 'Operator'}</span>. Monitor real-time API latency and service throughput.
                    </p>
                </div>
            </div>

            {/* Metrics Overview Grid */}
            <StatsGrid stats={stats} peakLatency={peakLatency} minLatency={minLatency} />

            {/* Visual Analytics charts */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                <ApiHitsChart stats={stats} />
                <StatusDistributionChart data={statusData} />
            </div>

            {/* Detailed Endpoints Performance metrics */}
            <TopEndpoints endpoints={topEndpoints} />
        </div>
    );
}
