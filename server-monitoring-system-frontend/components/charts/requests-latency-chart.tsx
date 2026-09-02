"use client";

import React from 'react';
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
import { Activity, AlertTriangle } from 'lucide-react';

interface MetricPoint {
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
    const hasData = data && data.length > 0 && data.some(d => (Number(d.totalHits) || 0) > 0 || (Number(d.avgLatency) || 0) > 0);

    const formattedData = data.map((item, idx) => {
        const timeStr = item.timeBucket 
            ? new Date(item.timeBucket).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            : `T-${idx}`;
        
        const total = Number(item.totalHits) || 0;
        const errors = Number(item.errorHits) || 0;
        const avgLat = Number(item.avgLatency) || 0;
        const maxLat = Number(item.maxLatency) || avgLat;

        return {
            time: timeStr,
            total,
            success: Math.max(0, total - errors),
            errors,
            avgLatency: Math.round(avgLat),
            p95Latency: Math.round(maxLat * 0.95),
        };
    });

    return (
        <div className={`surface-panel p-4 flex flex-col justify-between ${className}`}>
            <div className="flex items-center justify-between pb-3 border-b border-[#242932]">
                <span className="text-xs font-semibold text-zinc-300 uppercase tracking-wider">
                    {title || (type === 'requests' ? 'Request Traffic & Failures' : 'Latency Profile (p50 / p95)')}
                </span>
                <div className="flex items-center gap-3 text-[11px] text-zinc-400">
                    {type === 'requests' ? (
                        <>
                            <span className="inline-flex items-center gap-1.5">
                                <span className="w-2 h-2 rounded-xs bg-[#4CB8D6]" /> Total
                            </span>
                            <span className="inline-flex items-center gap-1.5">
                                <span className="w-2 h-2 rounded-xs bg-[#E45865]" /> Errors
                            </span>
                        </>
                    ) : (
                        <>
                            <span className="inline-flex items-center gap-1.5">
                                <span className="w-2 h-2 rounded-xs bg-[#5794E8]" /> Avg (p50)
                            </span>
                            <span className="inline-flex items-center gap-1.5">
                                <span className="w-2 h-2 rounded-xs bg-[#D99A3D]" /> p95
                            </span>
                        </>
                    )}
                </div>
            </div>

            <div className="h-48 w-full pt-3">
                {!hasData ? (
                    <div className="h-full flex flex-col items-center justify-center text-center p-4">
                        <Activity className="w-6 h-6 text-zinc-600 mb-1.5" />
                        <p className="text-xs font-medium text-zinc-400">Insufficient Telemetry Data</p>
                        <p className="text-[11px] text-zinc-500 max-w-xs mt-0.5">
                            Real-time time-series streams will render once continuous traffic arrives.
                        </p>
                    </div>
                ) : type === 'requests' ? (
                    <ResponsiveContainer width="100%" height="100%">
                        <AreaChart data={formattedData} margin={{ top: 5, right: 10, left: -20, bottom: 0 }}>
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
                            <XAxis dataKey="time" stroke="#545C67" fontSize={10} tickLine={false} />
                            <YAxis stroke="#545C67" fontSize={10} tickLine={false} allowDecimals={false} />
                            <Tooltip
                                contentStyle={{
                                    backgroundColor: '#111419',
                                    borderColor: '#242932',
                                    borderRadius: '6px',
                                    fontSize: '11px',
                                    color: '#F1F3F5',
                                }}
                            />
                            <Area type="monotone" dataKey="total" stroke="#4CB8D6" strokeWidth={1.5} fillOpacity={1} fill="url(#requestsGrad)" name="Total Requests" />
                            <Area type="monotone" dataKey="errors" stroke="#E45865" strokeWidth={1.5} fillOpacity={1} fill="url(#errorsGrad)" name="Errors" />
                        </AreaChart>
                    </ResponsiveContainer>
                ) : (
                    <ResponsiveContainer width="100%" height="100%">
                        <LineChart data={formattedData} margin={{ top: 5, right: 10, left: -20, bottom: 0 }}>
                            <CartesianGrid strokeDasharray="3 3" stroke="#1E232B" vertical={false} />
                            <XAxis dataKey="time" stroke="#545C67" fontSize={10} tickLine={false} />
                            <YAxis stroke="#545C67" fontSize={10} tickLine={false} unit="ms" />
                            <Tooltip
                                contentStyle={{
                                    backgroundColor: '#111419',
                                    borderColor: '#242932',
                                    borderRadius: '6px',
                                    fontSize: '11px',
                                    color: '#F1F3F5',
                                }}
                            />
                            <Line type="monotone" dataKey="avgLatency" stroke="#5794E8" strokeWidth={1.5} dot={false} name="Avg (ms)" />
                            <Line type="monotone" dataKey="p95Latency" stroke="#D99A3D" strokeWidth={1.5} strokeDasharray="4 4" dot={false} name="p95 (ms)" />
                        </LineChart>
                    </ResponsiveContainer>
                )}
            </div>
        </div>
    );
}
