"use client";

import React from 'react';
import {
    ResponsiveContainer,
    BarChart,
    Bar,
    XAxis,
    YAxis,
    Tooltip,
    CartesianGrid,
    ReferenceLine,
    Cell,
} from 'recharts';
import { Clock, TrendingUp, Info } from 'lucide-react';
import { cn } from '@/lib/utils';

interface QuantileSpectrumChartProps {
    p50: number;
    p75: number;
    p90: number;
    p95: number;
    p99: number;
    maxLatency: number;
    apdexThresholdMs?: number;
    className?: string;
}

export function QuantileSpectrumChart({
    p50,
    p75,
    p90,
    p95,
    p99,
    maxLatency,
    apdexThresholdMs = 500,
    className = "",
}: QuantileSpectrumChartProps) {
    const data = [
        {
            quantile: 'p50',
            label: 'Median',
            value: Math.round(p50),
            color: '#4CB8D6',
            description: '50% of requests are faster than this',
        },
        {
            quantile: 'p75',
            label: 'Upper Quartile',
            value: Math.round(p75),
            color: '#5794E8',
            description: '75% of requests completed within this duration',
        },
        {
            quantile: 'p90',
            label: '90th %ile',
            value: Math.round(p90),
            color: '#F59E0B',
            description: 'Top 10% response time boundary',
        },
        {
            quantile: 'p95',
            label: 'Critical 95th',
            value: Math.round(p95),
            color: '#D99A3D',
            description: '95% of traffic served under this limit',
        },
        {
            quantile: 'p99',
            label: 'Tail (99th)',
            value: Math.round(p99),
            color: '#E45865',
            description: 'Worst 1% tail latency experience',
        },
        {
            quantile: 'Max',
            label: 'Peak Spike',
            value: Math.round(maxLatency),
            color: '#9E2A2B',
            description: 'Absolute highest recorded latency spike',
        },
    ];

    const CustomTooltip = ({ active, payload }: any) => {
        if (!active || !payload || !payload.length) return null;
        const item = payload[0].payload;
        const delta = Math.round(item.value - p50);
        const ratio = p50 > 0 ? (item.value / p50).toFixed(1) : '1.0';

        return (
            <div className="bg-[#111419] border border-[#242932] rounded-lg p-3 shadow-xl text-xs space-y-1.5 min-w-[200px]">
                <div className="flex items-center justify-between border-b border-[#242932] pb-1">
                    <span className="font-mono font-bold text-zinc-100">{item.quantile}</span>
                    <span className="text-[11px] text-zinc-400">{item.label}</span>
                </div>
                <div className="flex items-baseline justify-between pt-0.5">
                    <span className="text-zinc-400">Response Time:</span>
                    <span className="font-mono font-bold text-base text-zinc-100">{item.value} ms</span>
                </div>
                {item.quantile !== 'p50' && (
                    <div className="flex items-center justify-between text-[11px] text-zinc-400 border-t border-[#242932]/60 pt-1">
                        <span>Delta vs Median:</span>
                        <span className="font-mono text-[#F59E0B]">+{delta}ms ({ratio}x)</span>
                    </div>
                )}
                <p className="text-[10px] text-zinc-500 pt-0.5 leading-relaxed">
                    {item.description}
                </p>
            </div>
        );
    };

    return (
        <div className={cn("surface-panel p-5 flex flex-col justify-between space-y-4", className)}>
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-[#242932]">
                <div>
                    <h3 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider flex items-center gap-2">
                        <Clock size={14} className="text-[#4CB8D6]" />
                        Latency Quantiles Spectrum
                    </h3>
                    <p className="text-[11px] text-zinc-400 mt-0.5">
                        Response time distribution across percentiles vs Apdex T target ({apdexThresholdMs}ms)
                    </p>
                </div>
                <div className="flex items-center gap-2 text-xs font-mono">
                    <span className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-[#0E1014] border border-[#F59E0B]/30 text-[#F59E0B] text-[10px]">
                        <span className="w-2 h-0.5 bg-[#F59E0B] border-b border-dashed" />
                        Target: {apdexThresholdMs}ms
                    </span>
                </div>
            </div>

            {/* Recharts Bar Chart */}
            <div className="h-52 w-full pt-1">
                <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={data} margin={{ top: 12, right: 12, left: -20, bottom: 0 }}>
                        <CartesianGrid strokeDasharray="3 3" stroke="#1E232B" vertical={false} />
                        <XAxis 
                            dataKey="quantile" 
                            stroke="#545C67" 
                            fontSize={11} 
                            tickLine={false} 
                        />
                        <YAxis 
                            stroke="#545C67" 
                            fontSize={10} 
                            tickLine={false} 
                            unit="ms" 
                        />
                        <Tooltip content={<CustomTooltip />} />
                        <ReferenceLine 
                            y={apdexThresholdMs} 
                            stroke="#F59E0B" 
                            strokeDasharray="4 4" 
                            label={{ 
                                value: `Apdex T (${apdexThresholdMs}ms)`, 
                                fill: '#F59E0B', 
                                fontSize: 10, 
                                position: 'top' 
                            }} 
                        />
                        <Bar dataKey="value" radius={[4, 4, 0, 0]}>
                            {data.map((entry, index) => (
                                <Cell key={`cell-${index}`} fill={entry.color} />
                            ))}
                        </Bar>
                    </BarChart>
                </ResponsiveContainer>
            </div>

            {/* Quantile Multiplier Badges */}
            <div className="grid grid-cols-3 sm:grid-cols-6 gap-2 pt-2 border-t border-[#1E232B] text-center">
                {data.map((item) => (
                    <div key={item.quantile} className="p-2 rounded bg-[#0E1014] border border-[#242932] space-y-0.5">
                        <span className="text-[10px] text-zinc-500 font-mono font-semibold block uppercase">
                            {item.quantile}
                        </span>
                        <span className="text-xs font-mono font-bold text-zinc-100 block">
                            {item.value} ms
                        </span>
                    </div>
                ))}
            </div>
        </div>
    );
}
