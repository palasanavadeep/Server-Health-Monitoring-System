"use client";

import { useState, useMemo } from 'react';
import { notFound } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { useApisMetricsQuery } from '@/hooks/use-dashboard-queries';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { 
    Braces, 
    Search, 
    ArrowUpDown, 
    Activity, 
    RefreshCw, 
    AlertCircle, 
    Server, 
    ActivitySquare, 
    Gauge,
    ChevronLeft,
    ChevronRight
} from 'lucide-react';
import { cn, getLatencyColorClass } from '@/lib/utils';

type SortField = 'endpoint' | 'hits' | 'avgLatency' | 'maxLatency' | 'errorRate';
type SortOrder = 'asc' | 'desc';

interface AggregatedApi {
    endpoint: string;
    method: string;
    serviceName: string;
    totalHits: number;
    errorHits: number;
    avgLatency: number;
    minLatency: number;
    maxLatency: number;
    errorRate: number;
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

    // Filters and search states
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedMethod, setSelectedMethod] = useState('ALL');
    const [selectedService, setSelectedService] = useState('ALL');
    const [sortField, setSortField] = useState<SortField>('hits');
    const [sortOrder, setSortOrder] = useState<SortOrder>('desc');

    // Paginated metrics query (only enabled for client users who have a clientId)
    const { data: metricsData, isPending, error, refetch, isFetching } = useApisMetricsQuery(
        page, 
        limit, 
        user?.clientId,
        { enabled: !!user?.clientId && user?.role !== 'super_admin' }
    );

    const apisList = metricsData?.items ?? [];
    const pagination = metricsData?.pagination ?? { page: 1, limit: 10, totalCount: 0, totalPages: 1 };

    // Group methods & services dynamically from current page items
    const servicesList = useMemo(() => {
        const services = new Set(apisList.map(a => a.serviceName));
        return ['ALL', ...Array.from(services)];
    }, [apisList]);

    const methodsList = useMemo(() => {
        const methods = new Set(apisList.map(a => a.method));
        return ['ALL', ...Array.from(methods)];
    }, [apisList]);

    // Apply filters and sorting locally on page items
    const processedApis = useMemo(() => {
        let result = [...apisList];

        // 1. Text Search Filter
        if (searchQuery.trim()) {
            const query = searchQuery.toLowerCase();
            result = result.filter(item => 
                item.endpoint.toLowerCase().includes(query) ||
                item.serviceName.toLowerCase().includes(query)
            );
        }

        // 2. Method Category Filter
        if (selectedMethod !== 'ALL') {
            result = result.filter(item => item.method === selectedMethod);
        }

        // 3. Service Scope Filter
        if (selectedService !== 'ALL') {
            result = result.filter(item => item.serviceName === selectedService);
        }

        // 4. Multi-column Sort
        result.sort((a, b) => {
            let valA = 0;
            let valB = 0;

            if (sortField === 'endpoint') {
                return sortOrder === 'asc' 
                    ? a.endpoint.localeCompare(b.endpoint) 
                    : b.endpoint.localeCompare(a.endpoint);
            } else if (sortField === 'hits') {
                valA = a.totalHits;
                valB = b.totalHits;
            } else if (sortField === 'avgLatency') {
                valA = a.avgLatency;
                valB = b.avgLatency;
            } else if (sortField === 'maxLatency') {
                valA = a.maxLatency;
                valB = b.maxLatency;
            } else if (sortField === 'errorRate') {
                valA = a.errorRate;
                valB = b.errorRate;
            }

            return sortOrder === 'asc' ? valA - valB : valB - valA;
        });

        return result;
    }, [apisList, searchQuery, selectedMethod, selectedService, sortField, sortOrder]);

    // Find the maximum latency value on the page to bound horizontal bar charts
    const maxGlobalLatency = useMemo(() => {
        if (!apisList.length) return 0;
        return Math.max(...apisList.map(a => a.maxLatency), 0);
    }, [apisList]);

    const handleSort = (field: SortField) => {
        if (sortField === field) {
            setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
        } else {
            setSortField(field);
            setSortOrder('desc');
        }
    };

    const getMethodColor = (method: string) => {
        switch (method.toUpperCase()) {
            case 'GET': return 'success';
            case 'POST': return 'info';
            case 'PUT': return 'warning';
            case 'DELETE': return 'destructive';
            default: return 'secondary';
        }
    };

    if (isPending) {
        return (
            <div className="h-[60vh] flex flex-col items-center justify-center gap-3">
                <RefreshCw className="animate-spin text-cyan-500 w-8 h-8" />
                <p className="text-sm font-medium text-muted-foreground animate-pulse font-mono">Fetching active api statistics...</p>
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
                    <h3 className="font-bold text-lg text-foreground">Query Error</h3>
                    <p className="text-sm text-muted-foreground">Failed to connect to microservice metrics aggregators.</p>
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
            {/* Header Title */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 border-b border-border-color/30 pb-6">
                <div>
                    <h1 className="text-3xl font-extrabold tracking-tight text-foreground flex items-center gap-2">
                        Registered API Routes
                    </h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Review latencies spreads, load distributions, and failure rates across ingestion channels.
                    </p>
                </div>
                <Button 
                    variant="secondary" 
                    size="sm" 
                    onClick={() => refetch()} 
                    disabled={isFetching}
                    className="gap-1.5 self-start sm:self-auto cursor-pointer"
                >
                    <RefreshCw size={12} className={isFetching ? "animate-spin text-cyan-400" : ""} />
                    Refresh Logs
                </Button>
            </div>

            {/* Filter controls cards */}
            <Card className="border-cyan-500/5 bg-glass-card/30">
                <CardContent className="p-5 space-y-4">
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                        {/* Search Input */}
                        <div className="relative">
                            <span className="absolute inset-y-0 left-0 pl-3 flex items-center text-muted-foreground pointer-events-none">
                                <Search size={16} />
                            </span>
                            <Input
                                type="text"
                                placeholder="Search by route path or service..."
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                className="pl-9 text-xs"
                            />
                        </div>

                        {/* Service Filter */}
                        <div className="flex items-center gap-2 w-full">
                            <span className="text-xs text-muted-foreground font-semibold shrink-0">Service:</span>
                            <select
                                value={selectedService}
                                onChange={(e) => setSelectedService(e.target.value)}
                                className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2 text-foreground"
                            >
                                {servicesList.map(service => (
                                    <option key={service} value={service}>{service}</option>
                                ))}
                            </select>
                        </div>

                        {/* Method Filter */}
                        <div className="flex items-center gap-2 w-full">
                            <span className="text-xs text-muted-foreground font-semibold shrink-0">Method:</span>
                            <select
                                value={selectedMethod}
                                onChange={(e) => setSelectedMethod(e.target.value)}
                                className="w-full text-xs bg-input-bg border border-border-color focus:border-cyan-500 focus:ring-1 focus:ring-cyan-500 outline-none rounded-lg p-2 text-foreground"
                            >
                                {methodsList.map(method => (
                                    <option key={method} value={method}>{method}</option>
                                ))}
                            </select>
                        </div>
                    </div>
                </CardContent>
            </Card>

            {/* API metrics Table */}
            <Card>
                <CardHeader className="pb-3 flex flex-row items-center justify-between border-b border-border-color/20">
                    <div>
                        <CardTitle>Telemetry APIs Profile</CardTitle>
                        <CardDescription>Comprehensive aggregations compiled from server metrics databases</CardDescription>
                    </div>
                    <Badge variant="info" className="font-mono text-[10px]">
                        Total API Routes: {pagination.totalCount}
                    </Badge>
                </CardHeader>
                <CardContent className="p-0">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead className="cursor-pointer select-none" onClick={() => handleSort('endpoint')}>
                                    <div className="flex items-center gap-1">
                                        API Route & Service
                                        <ArrowUpDown size={12} className={sortField === 'endpoint' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                    </div>
                                </TableHead>
                                <TableHead className="cursor-pointer select-none" onClick={() => handleSort('hits')}>
                                    <div className="flex items-center gap-1">
                                        Total Hits
                                        <ArrowUpDown size={12} className={sortField === 'hits' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                    </div>
                                </TableHead>
                                <TableHead className="cursor-pointer select-none" onClick={() => handleSort('errorRate')}>
                                    <div className="flex items-center gap-1">
                                        Errors
                                        <ArrowUpDown size={12} className={sortField === 'errorRate' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                    </div>
                                </TableHead>
                                <TableHead className="cursor-pointer select-none" onClick={() => handleSort('avgLatency')}>
                                    <div className="flex items-center gap-1">
                                        Avg Latency
                                        <ArrowUpDown size={12} className={sortField === 'avgLatency' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                    </div>
                                </TableHead>
                                <TableHead className="cursor-pointer select-none w-1/4" onClick={() => handleSort('maxLatency')}>
                                    <div className="flex items-center gap-1">
                                        Latency Profile (Min → Avg → Max)
                                        <ArrowUpDown size={12} className={sortField === 'maxLatency' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                    </div>
                                </TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {processedApis.length === 0 ? (
                                <TableRow>
                                    <TableCell colSpan={5} className="text-center py-12 text-muted-foreground">
                                        <div className="flex flex-col items-center justify-center">
                                            <ActivitySquare className="w-8 h-8 text-muted-foreground/30 mb-2" />
                                            <p className="font-semibold text-sm">No Matching API Outlets</p>
                                            <p className="text-xs text-muted-foreground/70 mt-0.5">Try clearing filters or search query.</p>
                                        </div>
                                    </TableCell>
                                </TableRow>
                            ) : (
                                processedApis.map((api, index) => {
                                    // Math checks for the horizontal range indicator
                                    const minPct = maxGlobalLatency > 0 ? (api.minLatency / maxGlobalLatency) * 100 : 0;
                                    const avgPct = maxGlobalLatency > 0 ? (api.avgLatency / maxGlobalLatency) * 100 : 0;
                                    const maxPct = maxGlobalLatency > 0 ? (api.maxLatency / maxGlobalLatency) * 100 : 0;

                                    return (
                                        <TableRow key={`${api.serviceName}|${api.method}|${api.endpoint}-${index}`}>
                                            <TableCell>
                                                <div className="flex flex-col gap-1 pr-4">
                                                    <div className="flex items-center gap-2">
                                                        <Badge variant={getMethodColor(api.method)} className="h-5 font-bold">
                                                            {api.method}
                                                        </Badge>
                                                        <code className="text-xs font-mono text-foreground font-semibold break-all">
                                                            {api.endpoint}
                                                        </code>
                                                    </div>
                                                    <span className="flex items-center gap-1 text-[10px] text-muted-foreground font-semibold uppercase tracking-wider">
                                                        <Server size={10} className="text-cyan-500" />
                                                        {api.serviceName}
                                                    </span>
                                                </div>
                                            </TableCell>
                                            <TableCell className="font-mono font-semibold text-sm">
                                                {api.totalHits.toLocaleString()}
                                            </TableCell>
                                            <TableCell className="font-mono text-sm font-semibold">
                                                <span className={cn(api.errorRate > 5 ? "text-rose-400 font-bold" : "text-emerald-400")}>
                                                    {api.errorRate.toFixed(1)}%
                                                </span>
                                            </TableCell>
                                            <TableCell className={cn("font-mono text-sm font-semibold", getLatencyColorClass(api.avgLatency))}>
                                                {api.avgLatency.toFixed(1)} ms
                                            </TableCell>
                                            
                                            {/* Latency Range Sparkline Visual */}
                                            <TableCell className="align-middle">
                                                <div className="space-y-1.5 min-w-[140px] pr-2">
                                                    <div className="relative h-1.5 w-full bg-white/5 rounded-full">
                                                        {/* Span bar (Min to Max) */}
                                                        <div 
                                                            className="absolute h-full rounded-full bg-cyan-500/30"
                                                            style={{
                                                                left: `${minPct}%`,
                                                                right: `${Math.max(0, 100 - maxPct)}%`
                                                            }}
                                                        />
                                                        {/* Average Dot Indicator */}
                                                        <div 
                                                            className="absolute w-2.5 h-2.5 -top-0.5 rounded-full bg-cyan-400 border border-zinc-950 shadow shadow-cyan-400/80 -translate-x-1/2"
                                                            style={{ left: `${avgPct}%` }}
                                                        />
                                                    </div>
                                                    <div className="flex items-center justify-between text-[9px] font-mono text-muted-foreground/80 leading-none">
                                                        <span>Min: {api.minLatency.toFixed(0)}ms</span>
                                                        <span className={cn("font-bold", getLatencyColorClass(api.avgLatency))}>Avg: {api.avgLatency.toFixed(0)}ms</span>
                                                        <span>Max: {api.maxLatency.toFixed(0)}ms</span>
                                                    </div>
                                                </div>
                                            </TableCell>
                                        </TableRow>
                                    );
                                })
                            )}
                        </TableBody>
                    </Table>

                    {/* Pagination bar */}
                    {pagination.totalPages > 1 && (
                        <div className="p-4 border-t border-border-color/20 flex flex-col sm:flex-row items-center justify-between gap-4 bg-zinc-950/20">
                            {/* Page Info */}
                            <div className="text-xs text-muted-foreground font-medium">
                                Showing page <span className="text-foreground font-bold">{page}</span> of <span className="text-foreground font-bold">{pagination.totalPages}</span> ({pagination.totalCount} endpoints registered)
                            </div>

                            {/* Page buttons */}
                            <div className="flex items-center gap-2">
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setPage(p => Math.max(1, p - 1))}
                                    disabled={page === 1}
                                    className="h-8 w-8 p-0 cursor-pointer"
                                >
                                    <ChevronLeft size={16} />
                                </Button>
                                
                                <div className="flex items-center gap-1">
                                    {Array.from({ length: pagination.totalPages }, (_, i) => i + 1).map(pageNum => (
                                        <Button
                                            key={pageNum}
                                            variant={page === pageNum ? "default" : "outline"}
                                            size="sm"
                                            onClick={() => setPage(pageNum)}
                                            className="h-8 w-8 p-0 text-xs font-semibold cursor-pointer"
                                        >
                                            {pageNum}
                                        </Button>
                                    ))}
                                </div>

                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setPage(p => Math.min(pagination.totalPages, p + 1))}
                                    disabled={page === pagination.totalPages}
                                    className="h-8 w-8 p-0 cursor-pointer"
                                >
                                    <ChevronRight size={16} />
                                </Button>
                            </div>

                            {/* Limit chooser */}
                            <div className="flex items-center gap-2 text-xs font-semibold">
                                <span className="text-muted-foreground">Rows per page:</span>
                                <select
                                    value={limit}
                                    onChange={(e) => {
                                        setLimit(Number(e.target.value));
                                        setPage(1); // reset to page 1
                                    }}
                                    className="bg-input-bg border border-border-color rounded px-1.5 py-1 text-foreground"
                                >
                                    <option value={5}>5</option>
                                    <option value={10}>10</option>
                                    <option value={20}>20</option>
                                    <option value={50}>50</option>
                                </select>
                            </div>
                        </div>
                    )}
                </CardContent>
            </Card>
        </div>
    );
}
