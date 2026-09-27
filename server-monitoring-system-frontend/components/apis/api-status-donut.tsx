"use client";

import React from 'react';
import {
    ResponsiveContainer,
    PieChart,
    Pie,
    Cell,
    Tooltip,
} from 'recharts';
import { Activity, CheckCircle, AlertCircle, AlertOctagon, HelpCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import { StatusDistribution } from '@/lib/api';

interface HttpStatusDonutProps {
    distribution?: StatusDistribution;
    successHits?: number;
    errorHits?: number;
    totalHits?: number;
    className?: string;
}

export function HttpStatusDonutChart({
    distribution,
    successHits = 0,
    errorHits = 0,
    totalHits = 0,
    className = "",
}: HttpStatusDonutProps) {
    const s1 = distribution?.status_1xx ?? 0;
    const s2 = distribution?.status_2xx ?? successHits;
    const s3 = distribution?.status_3xx ?? 0;
    const s4 = distribution?.status_4xx ?? 0;
    const s5 = distribution?.status_5xx ?? (distribution ? 0 : errorHits);

    const total = totalHits > 0 ? totalHits : (s1 + s2 + s3 + s4 + s5 || 1);
    const successRate = total > 0 ? (((s2 + s3) / total) * 100).toFixed(1) : '100.0';

    const chartData = [
        { name: '2xx Success', value: s2, color: '#48B982', category: 'Success responses (200, 201, 204)' },
        { name: '3xx Redirect', value: s3, color: '#4CB8D6', category: 'Redirection (301, 302, 304)' },
        { name: '4xx Client Error', value: s4, color: '#F59E0B', category: 'Client faults (400, 401, 403, 404)' },
        { name: '5xx Server Error', value: s5, color: '#E45865', category: 'Server failures (500, 502, 503)' },
    ].filter(d => d.value > 0);

    const fallbackData = chartData.length > 0 ? chartData : [
        { name: 'No Requests', value: 1, color: '#242932', category: 'No telemetry recorded' },
    ];

    const CustomTooltip = ({ active, payload }: any) => {
        if (!active || !payload || !payload.length) return null;
        const item = payload[0].payload;
        if (item.name === 'No Requests') return null;

        const pct = total > 0 ? ((item.value / total) * 100).toFixed(1) : '0';

        return (
            <div className="bg-[#111419] border border-[#242932] rounded-lg p-2.5 shadow-xl text-xs space-y-1 min-w-[190px]">
                <div className="flex items-center gap-1.5 font-semibold text-zinc-100 border-b border-[#242932] pb-1">
                    <span className="w-2 h-2 rounded-full" style={{ backgroundColor: item.color }} />
                    <span>{item.name}</span>
                </div>
                <div className="flex items-baseline justify-between pt-0.5">
                    <span className="text-zinc-400">Hits:</span>
                    <span className="font-mono font-bold text-zinc-100">{item.value.toLocaleString()} ({pct}%)</span>
                </div>
                <p className="text-[10px] text-zinc-500 pt-0.5">{item.category}</p>
            </div>
        );
    };

    return (
        <div className={cn("surface-panel p-5 flex flex-col justify-between space-y-4", className)}>
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-[#242932]">
                <div>
                    <h3 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider flex items-center gap-2">
                        <Activity size={14} className="text-[#4CB8D6]" />
                        HTTP Status Composition
                    </h3>
                    <p className="text-[11px] text-zinc-400 mt-0.5">
                        Response code class breakdown & overall availability ratio
                    </p>
                </div>
                <span className="text-xs font-mono font-bold text-[#48B982] bg-[#48B982]/10 border border-[#48B982]/30 px-2.5 py-0.5 rounded">
                    {successRate}% Success
                </span>
            </div>

            {/* Donut and Legend Grid */}
            <div className="grid grid-cols-1 sm:grid-cols-12 gap-4 items-center">
                {/* Donut Chart with Centered Metric */}
                <div className="sm:col-span-5 h-44 w-full relative flex items-center justify-center">
                    <ResponsiveContainer width="100%" height="100%">
                        <PieChart>
                            <Pie
                                data={fallbackData}
                                cx="50%"
                                cy="50%"
                                innerRadius={52}
                                outerRadius={72}
                                paddingAngle={chartData.length > 1 ? 3 : 0}
                                dataKey="value"
                                animationDuration={600}
                                stroke="none"
                            >
                                {fallbackData.map((entry, index) => (
                                    <Cell key={`cell-${index}`} fill={entry.color} />
                                ))}
                            </Pie>
                            <Tooltip content={<CustomTooltip />} />
                        </PieChart>
                    </ResponsiveContainer>

                    {/* Centered Availability Percentage */}
                    <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
                        <span className="text-xl font-bold font-mono text-zinc-100">
                            {successRate}%
                        </span>
                        <span className="text-[9px] font-mono text-zinc-500 uppercase tracking-wider">
                            Success
                        </span>
                    </div>
                </div>

                {/* Status Class Cards */}
                <div className="sm:col-span-7 grid grid-cols-2 gap-2 text-xs">
                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#48B982]/20 space-y-0.5">
                        <div className="flex items-center gap-1.5 text-[10px] text-[#48B982] font-semibold uppercase">
                            <span className="w-1.5 h-1.5 rounded-full bg-[#48B982]" />
                            2xx Success
                        </div>
                        <span className="text-sm font-bold font-mono text-zinc-100 block">
                            {s2.toLocaleString()}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            {total > 0 ? ((s2 / total) * 100).toFixed(1) : 0}% of traffic
                        </span>
                    </div>

                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#4CB8D6]/20 space-y-0.5">
                        <div className="flex items-center gap-1.5 text-[10px] text-[#4CB8D6] font-semibold uppercase">
                            <span className="w-1.5 h-1.5 rounded-full bg-[#4CB8D6]" />
                            3xx Redirect
                        </div>
                        <span className="text-sm font-bold font-mono text-zinc-100 block">
                            {s3.toLocaleString()}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            {total > 0 ? ((s3 / total) * 100).toFixed(1) : 0}% of traffic
                        </span>
                    </div>

                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#F59E0B]/20 space-y-0.5">
                        <div className="flex items-center gap-1.5 text-[10px] text-[#F59E0B] font-semibold uppercase">
                            <span className="w-1.5 h-1.5 rounded-full bg-[#F59E0B]" />
                            4xx Client Error
                        </div>
                        <span className="text-sm font-bold font-mono text-zinc-100 block">
                            {s4.toLocaleString()}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            {total > 0 ? ((s4 / total) * 100).toFixed(1) : 0}% of traffic
                        </span>
                    </div>

                    <div className="p-2.5 rounded bg-[#0E1014] border border-[#E45865]/20 space-y-0.5">
                        <div className="flex items-center gap-1.5 text-[10px] text-[#E45865] font-semibold uppercase">
                            <span className="w-1.5 h-1.5 rounded-full bg-[#E45865]" />
                            5xx Server Error
                        </div>
                        <span className="text-sm font-bold font-mono text-zinc-100 block">
                            {s5.toLocaleString()}
                        </span>
                        <span className="text-[10px] text-zinc-500 font-mono block">
                            {total > 0 ? ((s5 / total) * 100).toFixed(1) : 0}% of traffic
                        </span>
                    </div>
                </div>
            </div>
        </div>
    );
}
