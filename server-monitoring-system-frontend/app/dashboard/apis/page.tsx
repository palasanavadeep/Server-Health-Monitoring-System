"use client";

import { useState, useMemo } from 'react';
import { notFound, useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useApisMetricsQuery } from '@/hooks/use-dashboard-queries';
import { StatusBadge, HealthDot } from '@/components/ui/status-badge';
import { Button } from '@/components/ui/button';
import { 
    Search, 
    ChevronDown, 
    ChevronLeft, 
    ChevronRight, 
    RefreshCw, 
    AlertCircle, 
    Braces, 
    Activity, 
    Clock, 
    ShieldAlert, 
    ArrowUpRight,
    ExternalLink
} from 'lucide-react';
import { ApiMetricsEntry } from '@/lib/api';

type SortField = 'endpoint' | 'hits' | 'rpm' | 'avgLatency' | 'p95' | 'maxLatency' | 'errorRate' | 'apdex';
type SortOrder = 'asc' | 'desc';

function getApdexBadge(score?: number) {
    if (score === undefined || score === null) return null;
    let color = 'bg-[#48B982]/10 text-[#48B982] border-[#48B982]/30';
    let label = 'Excellent';
    if (score < 0.70) {
        color = 'bg-[#E45865]/10 text-[#E45865] border-[#E45865]/30';
        label = 'Poor';
    } else if (score < 0.85) {
        color = 'bg-[#F59E0B]/10 text-[#F59E0B] border-[#F59E0B]/30';
        label = 'Fair';
    } else if (score < 0.94) {
        color = 'bg-[#4CB8D6]/10 text-[#4CB8D6] border-[#4CB8D6]/30';
        label = 'Good';
    }
    return (
        <span 
            className={`inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-bold border ${color}`} 
            title={`Apdex: ${score.toFixed(4)} (${label})`}
        >
            {score.toFixed(2)}
        </span>
    );
}

export default function ApisPage() {
    const { user, loading } = useAuth();

    // Route guard: super_admin does not have access to client API metrics
    if (!loading && user && user.role === 'super_admin') {
        notFound();
    }

    // Pagination states
    const [page, setPage] = useState(1);
    const [limit, setLimit] = useState(10);

    const router = useRouter();

    // Filters and search states
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedMethod, setSelectedMethod] = useState('ALL');
    const [selectedService, setSelectedService] = useState('ALL');
    const [timeRange, setTimeRange] = useState('24h');
    const [sortField, setSortField] = useState<SortField>('hits');
    const [sortOrder, setSortOrder] = useState<SortOrder>('desc');

    // Paginated metrics query
    const { data: metricsData, isPending, error, refetch, isFetching } = useApisMetricsQuery(
        page, 
        limit, 
        user?.clientId,
        { enabled: !!user?.clientId && user?.role !== 'super_admin' }
    );

    const apisList = metricsData?.items ?? [];
    const pagination = metricsData?.pagination ?? { page: 1, limit: 10, totalCount: 0, totalPages: 1 };

    // Group methods & services dynamically
    const servicesList = useMemo(() => {
        const services = new Set(apisList.map(a => a.serviceName));
        return ['ALL', ...Array.from(services)];
    }, [apisList]);

    const methodsList = useMemo(() => {
        const methods = new Set(apisList.map(a => a.method));
        return ['ALL', ...Array.from(methods)];
    }, [apisList]);

    // Apply filters and sorting
    const processedApis = useMemo(() => {
        let result = [...apisList];

        if (searchQuery.trim()) {
            const query = searchQuery.toLowerCase();
            result = result.filter(item => 
                item.endpoint.toLowerCase().includes(query) ||
                item.serviceName.toLowerCase().includes(query)
            );
        }

        if (selectedMethod !== 'ALL') {
            result = result.filter(item => item.method === selectedMethod);
        }

        if (selectedService !== 'ALL') {
            result = result.filter(item => item.serviceName === selectedService);
        }

        result.sort((a, b) => {
            let valA: any;
            let valB: any;

            if (sortField === 'endpoint') {
                valA = a.endpoint;
                valB = b.endpoint;
                return sortOrder === 'asc' ? valA.localeCompare(valB) : valB.localeCompare(valA);
            } else if (sortField === 'hits') {
                valA = a.totalHits;
                valB = b.totalHits;
            } else if (sortField === 'rpm') {
                valA = a.throughputRpm ?? 0;
                valB = b.throughputRpm ?? 0;
            } else if (sortField === 'avgLatency') {
                valA = a.percentiles?.p50 ?? a.avgLatency;
                valB = b.percentiles?.p50 ?? b.avgLatency;
            } else if (sortField === 'p95') {
                valA = a.percentiles?.p95 ?? (a.avgLatency + (a.maxLatency - a.avgLatency) * 0.9);
                valB = b.percentiles?.p95 ?? (b.avgLatency + (b.maxLatency - b.avgLatency) * 0.9);
            } else if (sortField === 'maxLatency') {
                valA = a.percentiles?.p99 ?? a.maxLatency;
                valB = b.percentiles?.p99 ?? b.maxLatency;
            } else if (sortField === 'errorRate') {
                valA = a.errorRate;
                valB = b.errorRate;
            } else if (sortField === 'apdex') {
                valA = a.apdex?.score ?? 1.0;
                valB = b.apdex?.score ?? 1.0;
            }

            return sortOrder === 'asc' ? valA - valB : valB - valA;
        });

        return result;
    }, [apisList, searchQuery, selectedMethod, selectedService, sortField, sortOrder]);

    const handleSort = (field: SortField) => {
        if (sortField === field) {
            setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
        } else {
            setSortField(field);
            setSortOrder('desc');
        }
    };

    if (isPending) {
        return (
            <div className="h-[60vh] flex flex-col items-center justify-center gap-2.5 text-zinc-400">
                <RefreshCw className="animate-spin text-[#4CB8D6] w-5 h-5" />
                <p className="text-xs font-medium">Loading API route metrics...</p>
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
                    <h3 className="font-semibold text-sm text-zinc-100">Telemetry Stream Error</h3>
                    <p className="text-xs text-zinc-400">Unable to load metrics for registered endpoints.</p>
                </div>
                <Button variant="outline" size="sm" onClick={() => refetch()} className="text-xs h-8 gap-1.5 cursor-pointer">
                    <RefreshCw size={12} />
                    Retry Query
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
                        API Routes
                    </h1>
                    <p className="text-xs text-zinc-400 mt-0.5">
                        Inspect request traffic, latency percentiles, and failure distributions across endpoints.
                    </p>
                </div>

                <Button 
                    variant="outline" 
                    size="sm" 
                    onClick={() => refetch()} 
                    disabled={isFetching}
                    className="text-xs h-8 gap-1.5 cursor-pointer self-start sm:self-auto"
                >
                    <RefreshCw size={12} className={isFetching ? "animate-spin text-[#4CB8D6]" : ""} />
                    Refresh
                </Button>
            </div>

            {/* Flat Dense Filter Toolbar */}
            <div className="flex flex-wrap items-center justify-between gap-3 text-xs">
                <div className="flex flex-wrap items-center gap-2.5 flex-1 max-w-3xl">
                    {/* Search bar */}
                    <div className="relative min-w-[240px] flex-1">
                        <Search size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                        <input
                            type="text"
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            placeholder="Filter routes (e.g. /api/orders)..."
                            className="w-full bg-[#111419] border border-[#242932] rounded px-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                        />
                    </div>

                    {/* Service filter */}
                    <div className="relative">
                        <select
                            value={selectedService}
                            onChange={(e) => setSelectedService(e.target.value)}
                            className="bg-[#111419] border border-[#242932] text-xs font-medium text-zinc-300 rounded px-2.5 py-1.5 pr-6 appearance-none outline-none hover:border-[#323946] focus:border-[#4CB8D6] transition-colors cursor-pointer select-none"
                        >
                            {servicesList.map(srv => (
                                <option key={srv} value={srv}>Service: {srv}</option>
                            ))}
                        </select>
                        <ChevronDown size={12} className="absolute right-2 top-2.5 text-zinc-400 pointer-events-none" />
                    </div>

                    {/* Method filter */}
                    <div className="relative">
                        <select
                            value={selectedMethod}
                            onChange={(e) => setSelectedMethod(e.target.value)}
                            className="bg-[#111419] border border-[#242932] text-xs font-medium text-zinc-300 rounded px-2.5 py-1.5 pr-6 appearance-none outline-none hover:border-[#323946] focus:border-[#4CB8D6] transition-colors cursor-pointer select-none"
                        >
                            {methodsList.map(m => (
                                <option key={m} value={m}>Method: {m}</option>
                            ))}
                        </select>
                        <ChevronDown size={12} className="absolute right-2 top-2.5 text-zinc-400 pointer-events-none" />
                    </div>
                </div>

                <span className="text-xs text-zinc-400 font-mono">
                    {processedApis.length} {processedApis.length === 1 ? 'route' : 'routes'}
                </span>
            </div>

            {/* Dense Flat Data Table */}
            <div className="surface-panel overflow-hidden">
                {processedApis.length === 0 ? (
                    <div className="text-center py-12 p-6">
                        <Braces className="w-8 h-8 text-zinc-600 mx-auto mb-2" />
                        <p className="text-xs font-semibold text-zinc-300">No Matching Routes</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mx-auto mt-0.5">
                            {searchQuery || selectedMethod !== 'ALL' || selectedService !== 'ALL'
                                ? "No API endpoints matched your active filter parameters."
                                : "No endpoint telemetry recorded for this workspace yet."}
                        </p>
                    </div>
                ) : (
                    <div className="overflow-x-auto">
                        <table className="w-full text-left text-xs border-collapse">
                            <thead>
                                <tr className="border-b border-[#242932] text-zinc-400 text-[11px] uppercase tracking-wider bg-[#0E1014]/60 select-none">
                                    <th className="py-2.5 px-3 font-semibold w-20">Status</th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('endpoint')}
                                    >
                                        Route Path {sortField === 'endpoint' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th className="py-2.5 px-3 font-semibold">Service</th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('hits')}
                                    >
                                        Requests {sortField === 'hits' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('rpm')}
                                    >
                                        RPM {sortField === 'rpm' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('errorRate')}
                                    >
                                        Error Rate {sortField === 'errorRate' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('avgLatency')}
                                    >
                                        p50 {sortField === 'avgLatency' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('p95')}
                                    >
                                        p95 {sortField === 'p95' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('maxLatency')}
                                    >
                                        p99 {sortField === 'maxLatency' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th 
                                        className="py-2.5 px-3 font-semibold text-right cursor-pointer hover:text-zinc-200 transition-colors"
                                        onClick={() => handleSort('apdex')}
                                    >
                                        Apdex {sortField === 'apdex' && (sortOrder === 'asc' ? '↑' : '↓')}
                                    </th>
                                    <th className="py-2.5 px-3 w-8"></th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-[#242932]">
                                {processedApis.map((api, idx) => {
                                    const errRate = api.errorRate || 0;
                                    const p50 = Math.round(api.percentiles?.p50 ?? api.avgLatency);
                                    const p95 = Math.round(api.percentiles?.p95 ?? (api.avgLatency + (api.maxLatency - api.avgLatency) * 0.9));
                                    const p99 = Math.round(api.percentiles?.p99 ?? api.maxLatency);
                                    const rpm = api.throughputRpm !== undefined ? api.throughputRpm.toFixed(1) : '-';
                                    const isDegraded = errRate > 5 || p50 > 400 || (api.apdex?.score !== undefined && api.apdex.score < 0.70);

                                    return (
                                        <tr
                                            key={`${api.serviceName}-${api.endpoint}-${api.method}-${idx}`}
                                            onClick={() => router.push(`/dashboard/apis/details?endpoint=${encodeURIComponent(api.endpoint)}&method=${api.method}&service=${encodeURIComponent(api.serviceName)}`)}
                                            className="hover:bg-[#181D24] transition-colors cursor-pointer group"
                                            title="Click to view dedicated API details"
                                        >
                                            <td className="py-3 px-3 whitespace-nowrap">
                                                <StatusBadge status={isDegraded ? 'degraded' : 'healthy'} />
                                            </td>
                                            <td className="py-3 px-3 font-mono font-medium text-zinc-200 whitespace-nowrap">
                                                <span className={`font-bold mr-1.5 text-[11px] ${
                                                    api.method === 'GET' ? 'text-[#4CB8D6]' :
                                                    api.method === 'POST' ? 'text-[#48B982]' :
                                                    api.method === 'PUT' || api.method === 'PATCH' ? 'text-[#D99A3D]' : 'text-[#E45865]'
                                                }`}>
                                                    {api.method}
                                                </span>
                                                <span className="group-hover:text-[#4CB8D6] transition-colors">
                                                    {api.endpoint}
                                                </span>
                                            </td>
                                            <td className="py-3 px-3 text-zinc-400 whitespace-nowrap">
                                                {api.serviceName}
                                            </td>
                                            <td className="py-3 px-3 text-right font-mono text-zinc-200">
                                                {api.totalHits}
                                            </td>
                                            <td className="py-3 px-3 text-right font-mono text-zinc-400">
                                                {rpm}
                                            </td>
                                            <td className={`py-3 px-3 text-right font-mono font-semibold ${errRate > 0 ? 'text-[#E45865]' : 'text-[#48B982]'}`}>
                                                {errRate.toFixed(1)}%
                                            </td>
                                            <td className="py-3 px-3 text-right font-mono text-zinc-300">
                                                {p50} ms
                                            </td>
                                            <td className="py-3 px-3 text-right font-mono text-zinc-300">
                                                {p95} ms
                                            </td>
                                            <td className="py-3 px-3 text-right font-mono text-zinc-400">
                                                {p99} ms
                                            </td>
                                            <td className="py-3 px-3 text-right">
                                                {getApdexBadge(api.apdex?.score)}
                                            </td>
                                            <td className="py-3 px-3 text-right text-zinc-600 group-hover:text-[#4CB8D6] transition-colors">
                                                <ChevronRight size={14} />
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                )}

                {/* Pagination Controls */}
                {pagination.totalPages > 1 && (
                    <div className="p-3 border-t border-[#242932] flex items-center justify-between text-xs text-zinc-400 bg-[#0E1014]/50">
                        <span>Page {pagination.page} of {pagination.totalPages}</span>
                        <div className="flex items-center gap-2">
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setPage(prev => Math.max(1, prev - 1))}
                                disabled={pagination.page <= 1}
                                className="h-7 px-2 text-xs cursor-pointer"
                            >
                                <ChevronLeft size={13} />
                                Previous
                            </Button>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={() => setPage(prev => Math.min(pagination.totalPages, prev + 1))}
                                disabled={pagination.page >= pagination.totalPages}
                                className="h-7 px-2 text-xs cursor-pointer"
                            >
                                Next
                                <ChevronRight size={13} />
                            </Button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
