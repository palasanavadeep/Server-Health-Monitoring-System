"use client";

import { useState, useMemo } from 'react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { Input } from '@/components/ui/input';
import { BarChart3, TrendingUp, Clock, AlertCircle, Activity, Search, ArrowUpDown } from 'lucide-react';
import { TopEndpoint } from '@/lib/api';
import { cn, getLatencyColorClass } from '@/lib/utils';

interface TopEndpointsProps {
    endpoints: TopEndpoint[];
}

type SortField = 'hits' | 'latency' | 'errorRate';
type SortOrder = 'asc' | 'desc';

export function TopEndpoints({ endpoints }: TopEndpointsProps) {
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedMethod, setSelectedMethod] = useState<string>('ALL');
    const [sortField, setSortField] = useState<SortField>('hits');
    const [sortOrder, setSortOrder] = useState<SortOrder>('desc');

    const getMethodVariant = (method: string) => {
        const variants: Record<string, 'success' | 'info' | 'warning' | 'destructive' | 'default'> = {
            GET: 'success',
            POST: 'info',
            PUT: 'warning',
            DELETE: 'destructive',
            PATCH: 'default',
        };
        return variants[method] || 'secondary';
    };

    const handleSort = (field: SortField) => {
        if (sortField === field) {
            setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
        } else {
            setSortField(field);
            setSortOrder('desc');
        }
    };

    // Filter and Sort endpoints
    const processedEndpoints = useMemo(() => {
        if (!endpoints) return [];

        let result = endpoints.map(e => {
            const hitsNum = typeof e.totalHits === 'string' ? parseInt(e.totalHits, 10) : Number(e.totalHits ?? 0);
            const avgLatencyNum = typeof e.avgLatency === 'string' ? parseFloat(e.avgLatency) : Number(e.avgLatency ?? 0);
            const errorRateNum = typeof e.errorRate === 'string' ? parseFloat(e.errorRate) : Number(e.errorRate ?? 0);
            
            return {
                ...e,
                hitsNum,
                avgLatency: isNaN(avgLatencyNum) ? 0 : avgLatencyNum,
                errorRate: isNaN(errorRateNum) ? 0 : errorRateNum,
            };
        });


        // Search filter
        if (searchQuery) {
            const query = searchQuery.toLowerCase();
            result = result.filter(e => 
                e.endpoint.toLowerCase().includes(query) ||
                e.serviceName.toLowerCase().includes(query)
            );
        }

        // Method filter
        if (selectedMethod !== 'ALL') {
            result = result.filter(e => e.method === selectedMethod);
        }

        // Sort
        result.sort((a, b) => {
            let valA = 0;
            let valB = 0;

            if (sortField === 'hits') {
                valA = a.hitsNum;
                valB = b.hitsNum;
            } else if (sortField === 'latency') {
                valA = a.avgLatency;
                valB = b.avgLatency;
            } else if (sortField === 'errorRate') {
                valA = a.errorRate;
                valB = b.errorRate;
            }

            return sortOrder === 'asc' ? valA - valB : valB - valA;
        });

        return result;
    }, [endpoints, searchQuery, selectedMethod, sortField, sortOrder]);

    const methodsList = useMemo(() => {
        if (!endpoints) return [];
        const methods = new Set(endpoints.map(e => e.method));
        return ['ALL', ...Array.from(methods)];
    }, [endpoints]);

    if (!endpoints || endpoints.length === 0) {
        return (
            <Card>
                <CardHeader>
                    <div className="flex items-center gap-3">
                        <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/20">
                            <BarChart3 size={18} />
                        </div>
                        <div>
                            <CardTitle>Top Endpoints</CardTitle>
                            <CardDescription>Most active API endpoints</CardDescription>
                        </div>
                    </div>
                </CardHeader>
                <CardContent>
                    <div className="flex flex-col items-center justify-center py-12 text-center">
                        <div className="flex items-center justify-center w-12 h-12 rounded-full bg-white/5 text-muted-foreground/50 mb-4">
                            <Activity size={24} />
                        </div>
                        <p className="text-sm font-semibold text-foreground">No traffic data available yet</p>
                        <p className="text-xs text-muted-foreground mt-1 max-w-[280px]">
                            Endpoint statistics will appear here once metrics are logged.
                        </p>
                    </div>
                </CardContent>
            </Card>
        );
    }

    return (
        <Card className="animate-in fade-in slide-in-from-bottom-3 duration-300">
            <CardHeader className="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-4">
                <div className="flex items-center gap-3">
                    <div className="flex items-center justify-center w-10 h-10 rounded-xl bg-violet-600/10 text-violet-500 border border-violet-500/20 shadow-sm">
                        <BarChart3 size={18} />
                    </div>
                    <div>
                        <CardTitle>Top Endpoints</CardTitle>
                        <CardDescription>Most active API endpoints by hit count</CardDescription>
                    </div>
                </div>

                {/* Filter and Search controls */}
                <div className="flex flex-col sm:flex-row gap-3 items-center">
                    <div className="relative w-full sm:w-48">
                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground w-4 h-4 pointer-events-none" />
                        <Input
                            placeholder="Filter paths..."
                            value={searchQuery}
                            onChange={(e) => setSearchQuery(e.target.value)}
                            className="pl-9 h-9"
                        />
                    </div>

                    <div className="flex items-center gap-1.5 w-full sm:w-auto">
                        <span className="text-xs text-muted-foreground font-semibold hidden md:inline">Method:</span>
                        <div className="flex flex-wrap gap-1 bg-glass-card border border-border-color p-0.5 rounded-lg w-full sm:w-auto">
                            {methodsList.map(method => (
                                <button
                                    key={method}
                                    onClick={() => setSelectedMethod(method)}
                                    className={cn(
                                        "px-2.5 py-1 rounded-md text-xs font-semibold cursor-pointer transition-colors",
                                        selectedMethod === method
                                            ? "bg-cyan-500/20 text-cyan-400 border border-cyan-500/10"
                                            : "text-muted-foreground hover:text-foreground"
                                    )}
                                >
                                    {method}
                                </button>
                            ))}
                        </div>
                    </div>
                </div>
            </CardHeader>
            <CardContent className="pt-2">
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHead className="w-12 text-center">Rank</TableHead>
                            <TableHead>Path & Service</TableHead>
                            <TableHead>Method</TableHead>
                            <TableHead className="cursor-pointer select-none" onClick={() => handleSort('hits')}>
                                <div className="flex items-center gap-1">
                                    Hits
                                    <ArrowUpDown size={12} className={sortField === 'hits' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                </div>
                            </TableHead>
                            <TableHead className="cursor-pointer select-none" onClick={() => handleSort('latency')}>
                                <div className="flex items-center gap-1">
                                    Avg Latency
                                    <ArrowUpDown size={12} className={sortField === 'latency' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                </div>
                            </TableHead>
                            <TableHead className="cursor-pointer select-none" onClick={() => handleSort('errorRate')}>
                                <div className="flex items-center gap-1">
                                    Error Rate
                                    <ArrowUpDown size={12} className={sortField === 'errorRate' ? 'text-cyan-400' : 'text-muted-foreground'} />
                                </div>
                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {processedEndpoints.length === 0 ? (
                            <TableRow>
                                <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">
                                    No matching endpoints found
                                </TableCell>
                            </TableRow>
                        ) : (
                            processedEndpoints.map((endpoint, index) => {
                                const rank = index + 1;
                                return (
                                    <TableRow key={`${endpoint.endpoint}-${endpoint.method}`}>
                                        <TableCell className="text-center font-mono font-bold text-xs">
                                            <span className={cn(
                                                "inline-flex items-center justify-center w-6 h-6 rounded-full",
                                                rank === 1 && "bg-amber-500/10 text-amber-500 border border-amber-500/20",
                                                rank === 2 && "bg-slate-300/10 text-slate-300 border border-slate-300/20",
                                                rank === 3 && "bg-amber-700/10 text-amber-600 border border-amber-700/20",
                                                rank > 3 && "bg-white/5 text-muted-foreground"
                                            )}>
                                                {rank}
                                            </span>
                                        </TableCell>
                                        <TableCell>
                                            <div className="flex flex-col gap-0.5">
                                                <code className="text-sm font-mono text-foreground font-semibold break-all">
                                                    {endpoint.endpoint}
                                                </code>
                                                <span className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">
                                                    {endpoint.serviceName}
                                                </span>
                                            </div>
                                        </TableCell>
                                        <TableCell>
                                            <Badge variant={getMethodVariant(endpoint.method)}>
                                                {endpoint.method}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="font-mono font-semibold text-sm">
                                            <div className="flex items-center gap-2">
                                                <TrendingUp size={13} className="text-cyan-400" />
                                                {endpoint.hitsNum.toLocaleString()}
                                            </div>
                                        </TableCell>
                                        <TableCell className={cn("font-mono font-semibold text-sm", getLatencyColorClass(endpoint.avgLatency))}>
                                            <div className="flex items-center gap-2">
                                                <Clock size={13} className="text-current" />
                                                {endpoint.avgLatency.toFixed(1)} ms
                                            </div>
                                        </TableCell>
                                        <TableCell className="font-mono font-semibold text-sm">
                                            <div className="flex items-center gap-2">
                                                <AlertCircle size={13} className={cn(endpoint.errorRate > 5 ? "text-rose-400" : "text-emerald-400")} />
                                                <span className={cn(endpoint.errorRate > 5 && "text-rose-400 font-bold")}>
                                                    {endpoint.errorRate.toFixed(1)}%
                                                </span>
                                            </div>
                                        </TableCell>
                                    </TableRow>
                                );
                            })
                        )}
                    </TableBody>
                </Table>
            </CardContent>
        </Card>
    );
}
export default TopEndpoints;
