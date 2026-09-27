"use client";

import React, { useMemo } from 'react';
import {
    ResponsiveContainer,
    AreaChart,
    Area,
    LineChart,
    Line,
    XAxis,
    YAxis,
    Tooltip,
    CartesianGrid,
} from 'recharts';
import { Activity } from 'lucide-react';

export interface MetricPoint {
    serviceName?: string;
    endpoint?: string;
    method?: string;
    timeBucket?: string;
    totalHits?: number;
    errorHits?: number;
    avgLatency?: number | string;
    minLatency?: number | string;
    maxLatency?: number | string;
}

interface RequestsLatencyChartProps {
    data: MetricPoint[];
    title?: string;
    type?: 'requests' | 'latency';
    className?: string;
}

export function TelemetryTimeSeriesChart({
    data = [],
    title,
    type = 'requests',
    className = "",
}: RequestsLatencyChartProps) {
    // Aggregate records by unique timeBucket & sort chronologically (oldest -> newest, left -> right)
    const { formattedData, summaryStats, hasData } = useMemo(() => {
        if (!data || data.length === 0) {
            return { formattedData: [], summaryStats: null, hasData: false };
        }

        // Group rows by timeBucket to consolidate multiple endpoints active in the same hour
        const bucketMap = new Map<string, {
            timeBucket: string;
            total: number;
            errors: number;
            weightedLatencySum: number;
            maxLatency: number;
        }>();

        data.forEach((item) => {
            if (!item.timeBucket) return;
            const bucketKey = item.timeBucket;
            const existing = bucketMap.get(bucketKey) || {
                timeBucket: bucketKey,
                total: 0,
                errors: 0,
                weightedLatencySum: 0,
                maxLatency: 0,
            };

            const total = Number(item.totalHits) || 0;
            const errors = Number(item.errorHits) || 0;
            const avgLat = Number(item.avgLatency) || 0;
            const maxLat = Number(item.maxLatency) || avgLat;

            existing.total += total;
            existing.errors += errors;
            existing.weightedLatencySum += avgLat * total;
            existing.maxLatency = Math.max(existing.maxLatency, maxLat);

            bucketMap.set(bucketKey, existing);
        });

        if (bucketMap.size === 0) {
            return { formattedData: [], summaryStats: null, hasData: false };
        }

        // Sort ascending (chronological: left = past, right = present)
        const sortedBuckets = Array.from(bucketMap.values()).sort((a, b) => {
            return new Date(a.timeBucket).getTime() - new Date(b.timeBucket).getTime();
        });

        const firstTime = new Date(sortedBuckets[0].timeBucket).getTime();
        const lastTime = new Date(sortedBuckets[sortedBuckets.length - 1].timeBucket).getTime();
        const isMultiDay = (lastTime - firstTime) > 86_400_000 * 1.5;

        let totalVolume = 0;
        let totalErrors = 0;
        let totalWeightedLat = 0;
        let peakLatency = 0;

        const points = sortedBuckets.map((b) => {
            const d = new Date(b.timeBucket);
            const total = b.total;
            const errors = b.errors;
            const avgLatency = total > 0 ? Math.round(b.weightedLatencySum / total) : 0;
            const maxLatency = Math.round(b.maxLatency);
            const p95Latency = Math.round(maxLatency > 0 ? (avgLatency + (maxLatency - avgLatency) * 0.9) : avgLatency);

            totalVolume += total;
            totalErrors += errors;
            totalWeightedLat += b.weightedLatencySum;
            if (maxLatency > peakLatency) peakLatency = maxLatency;

            const timeLabel = isMultiDay
                ? `${d.toLocaleDateString([], { month: 'short', day: 'numeric' })} ${d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}`
                : d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false });

            const fullDateLabel = `${d.toLocaleDateString([], { month: 'short', day: 'numeric' })}, ${d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;

            return {
                time: timeLabel,
                fullDateLabel,
                total,
                errors,
                errorRate: total > 0 ? ((errors / total) * 100).toFixed(1) : '0.0',
                avgLatency,
                p95Latency,
                maxLatency,
            };
        });

        const overallAvgLatency = totalVolume > 0 ? Math.round(totalWeightedLat / totalVolume) : 0;
        const overallErrorRate = totalVolume > 0 ? ((totalErrors / totalVolume) * 100).toFixed(1) : '0.0';

        const stats = {
            totalVolume,
            totalErrors,
            overallAvgLatency,
            overallErrorRate,
            peakLatency,
        };

        const activeData = points.some(p => p.total > 0 || p.avgLatency > 0);

        return { formattedData: points, summaryStats: stats, hasData: activeData };
    }, [data]);

    const isSinglePoint = formattedData.length === 1;

    // Clean, professional tooltip
    const CustomTooltip = ({ active, payload }: any) => {
        if (!active || !payload || !payload.length) return null;
        const item = payload[0].payload;

        return (
            <div className="bg-[#111419] border border-[#242932] rounded-lg p-2.5 shadow-xl text-xs space-y-1.5 min-w-[180px]">
                <div className="text-[11px] font-medium text-zinc-300 border-b border-[#242932] pb-1">
                    {item.fullDateLabel}
                </div>

                {type === 'requests' ? (
                    <div className="space-y-1 text-[11px]">
                        <div className="flex items-center justify-between gap-3">
                            <span className="flex items-center gap-1.5 text-zinc-400">
                                <span className="w-2 h-2 rounded-full bg-[#4CB8D6]" /> Requests:
                            </span>
                            <span className="font-mono font-semibold text-zinc-100">
                                {item.total.toLocaleString()}
                            </span>
                        </div>
                        <div className="flex items-center justify-between gap-3">
                            <span className="flex items-center gap-1.5 text-zinc-400">
                                <span className="w-2 h-2 rounded-full bg-[#E45865]" /> Errors:
                            </span>
                            <span className={`font-mono ${item.errors > 0 ? 'text-[#E45865] font-semibold' : 'text-zinc-400'}`}>
                                {item.errors.toLocaleString()} {item.errors > 0 && `(${item.errorRate}%)`}
                            </span>
                        </div>
                    </div>
                ) : (
                    <div className="space-y-1 text-[11px]">
                        <div className="flex items-center justify-between gap-3">
                            <span className="flex items-center gap-1.5 text-zinc-400">
                                <span className="w-2 h-2 rounded-full bg-[#5794E8]" /> p50 (Median):
                            </span>
                            <span className="font-mono font-semibold text-zinc-100">
                                {item.avgLatency} ms
                            </span>
                        </div>
                        <div className="flex items-center justify-between gap-3">
                            <span className="flex items-center gap-1.5 text-zinc-400">
                                <span className="w-2 h-2 rounded-full bg-[#F59E0B]" /> p95 Latency:
                            </span>
                            <span className="font-mono font-semibold text-[#F59E0B]">
                                {item.p95Latency} ms
                            </span>
                        </div>
                        <div className="flex items-center justify-between gap-3">
                            <span className="flex items-center gap-1.5 text-zinc-400">
                                <span className="w-2 h-2 rounded-full bg-zinc-500" /> Peak Latency:
                            </span>
                            <span className="font-mono text-zinc-400">
                                {item.maxLatency} ms
                            </span>
                        </div>
                    </div>
                )}
            </div>
        );
    };

    return (
        <div className={`surface-panel p-4 flex flex-col justify-between ${className}`}>
            {/* Header: Title and Sleek Legend */}
            <div className="flex items-center justify-between pb-2.5 border-b border-[#242932]">
                <div className="flex items-center gap-2">
                    <span className="text-xs font-semibold text-zinc-200 tracking-wide uppercase">
                        {title || (type === 'requests' ? 'Request Traffic' : 'Latency Profile (p50 / p95)')}
                    </span>
                </div>

                {/* Inline Legend & Key Stats */}
                <div className="flex items-center gap-3.5 text-xs">
                    {type === 'requests' ? (
                        <>
                            <div className="flex items-center gap-1.5 text-zinc-400 text-[11px]">
                                <span className="w-2 h-2 rounded-full bg-[#4CB8D6]" />
                                <span>Requests</span>
                            </div>
                            <div className="flex items-center gap-1.5 text-zinc-400 text-[11px]">
                                <span className="w-2 h-2 rounded-full bg-[#E45865]" />
                                <span>Errors</span>
                            </div>
                            {summaryStats && summaryStats.totalVolume > 0 && (
                                <span className="text-[11px] font-mono text-zinc-400 pl-1 border-l border-[#242932]">
                                    {summaryStats.totalVolume.toLocaleString()} total
                                </span>
                            )}
                        </>
                    ) : (
                        <>
                            <div className="flex items-center gap-1.5 text-zinc-400 text-[11px]">
                                <span className="w-2 h-2 rounded-full bg-[#5794E8]" />
                                <span>p50</span>
                            </div>
                            <div className="flex items-center gap-1.5 text-zinc-400 text-[11px]">
                                <span className="w-2 h-2 rounded-full bg-[#F59E0B]" />
                                <span>p95</span>
                            </div>
                            {summaryStats && summaryStats.overallAvgLatency > 0 && (
                                <span className="text-[11px] font-mono text-zinc-400 pl-1 border-l border-[#242932]">
                                    ~{summaryStats.overallAvgLatency} ms
                                </span>
                            )}
                        </>
                    )}
                </div>
            </div>

            {/* Chart Body */}
            <div className="h-48 w-full pt-3">
                {!hasData ? (
                    <div className="h-full flex flex-col items-center justify-center text-center p-4">
                        <Activity className="w-6 h-6 text-zinc-600 mb-1.5" />
                        <p className="text-xs font-medium text-zinc-400">No Telemetry Recorded</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mt-0.5">
                            Data points will appear once HTTP requests are ingested for this time window.
                        </p>
                    </div>
                ) : type === 'requests' ? (
                    <ResponsiveContainer width="100%" height="100%">
                        <AreaChart data={formattedData} margin={{ top: 8, right: 8, left: -20, bottom: 0 }}>
                            <defs>
                                <linearGradient id="requestsGrad" x1="0" y1="0" x2="0" y2="1">
                                    <stop offset="5%" stopColor="#4CB8D6" stopOpacity={0.25}/>
                                    <stop offset="95%" stopColor="#4CB8D6" stopOpacity={0.0}/>
                                </linearGradient>
                                <linearGradient id="errorsGrad" x1="0" y1="0" x2="0" y2="1">
                                    <stop offset="5%" stopColor="#E45865" stopOpacity={0.35}/>
                                    <stop offset="95%" stopColor="#E45865" stopOpacity={0.0}/>
                                </linearGradient>
                            </defs>
                            <CartesianGrid strokeDasharray="3 3" stroke="#1E232B" vertical={false} />
                            <XAxis 
                                dataKey="time" 
                                stroke="#545C67" 
                                fontSize={10} 
                                tickLine={false} 
                                minTickGap={28}
                            />
                            <YAxis 
                                stroke="#545C67" 
                                fontSize={10} 
                                tickLine={false} 
                                allowDecimals={false} 
                            />
                            <Tooltip content={<CustomTooltip />} />
                            <Area 
                                type="monotone" 
                                dataKey="total" 
                                stroke="#4CB8D6" 
                                strokeWidth={2} 
                                fillOpacity={1} 
                                fill="url(#requestsGrad)" 
                                dot={isSinglePoint ? { r: 4, fill: '#4CB8D6', stroke: '#0E1014', strokeWidth: 2 } : false} 
                                name="Requests" 
                            />
                            <Area 
                                type="monotone" 
                                dataKey="errors" 
                                stroke="#E45865" 
                                strokeWidth={1.5} 
                                fillOpacity={1} 
                                fill="url(#errorsGrad)" 
                                dot={isSinglePoint && formattedData[0]?.errors > 0 ? { r: 3.5, fill: '#E45865' } : false} 
                                name="Errors" 
                            />
                        </AreaChart>
                    </ResponsiveContainer>
                ) : (
                    <ResponsiveContainer width="100%" height="100%">
                        <LineChart data={formattedData} margin={{ top: 8, right: 8, left: -20, bottom: 0 }}>
                            <CartesianGrid strokeDasharray="3 3" stroke="#1E232B" vertical={false} />
                            <XAxis 
                                dataKey="time" 
                                stroke="#545C67" 
                                fontSize={10} 
                                tickLine={false} 
                                minTickGap={28}
                            />
                            <YAxis 
                                stroke="#545C67" 
                                fontSize={10} 
                                tickLine={false} 
                                unit="ms" 
                            />
                            <Tooltip content={<CustomTooltip />} />
                            <Line 
                                type="monotone" 
                                dataKey="avgLatency" 
                                stroke="#5794E8" 
                                strokeWidth={2} 
                                dot={isSinglePoint ? { r: 4, fill: '#5794E8', stroke: '#0E1014', strokeWidth: 2 } : false} 
                                name="p50" 
                            />
                            <Line 
                                type="monotone" 
                                dataKey="p95Latency" 
                                stroke="#F59E0B" 
                                strokeWidth={1.5} 
                                strokeDasharray="4 4" 
                                dot={isSinglePoint ? { r: 3.5, fill: '#F59E0B' } : false} 
                                name="p95" 
                            />
                        </LineChart>
                    </ResponsiveContainer>
                )}
            </div>
        </div>
    );
}
