"use client";

import { Card, CardContent } from '@/components/ui/card';
import { 
    TrendingUp, 
    Clock, 
    AlertTriangle, 
    CheckCircle2, 
    Layers, 
    Zap, 
    Activity, 
    ZapOff 
} from 'lucide-react';
import { MetricStats } from '@/lib/api';
import { cn, getLatencyColorClass } from '@/lib/utils';

interface StatsGridProps {
    stats: MetricStats | null;
    peakLatency?: number;
    minLatency?: number;
}

export function StatsGrid({ stats, peakLatency = 0, minLatency = 0 }: StatsGridProps) {
    if (!stats) return null;

    const successRate = 100 - stats.errorRate;

    const statCards = [
        {
            title: 'Total Hits',
            value: stats.totalHits.toLocaleString(),
            subtitle: 'Query telemetry logs',
            icon: TrendingUp,
            colorClass: 'text-cyan-400',
            bgClass: 'bg-cyan-500/15 border-cyan-500/20',
            borderGlow: 'hover:border-cyan-500/40 hover:shadow-[0_0_20px_rgba(6,182,212,0.15)]',
            progressColor: 'bg-gradient-to-r from-blue-500 to-cyan-500',
        },
        {
            title: 'Average Latency',
            value: `${stats.avgLatency.toFixed(1)} ms`,
            subtitle: 'Weighted response time',
            icon: Clock,
            colorClass: 'text-teal-400',
            bgClass: 'bg-teal-500/15 border-teal-500/20',
            borderGlow: 'hover:border-teal-500/40 hover:shadow-[0_0_20px_rgba(20,184,166,0.15)]',
            progressColor: 'bg-gradient-to-r from-teal-500 to-emerald-500',
            valueColor: getLatencyColorClass(stats.avgLatency)
        },
        {
            title: 'Peak Latency',
            value: peakLatency > 0 ? `${peakLatency.toFixed(1)} ms` : '--',
            subtitle: 'Outlier response spike',
            icon: ZapOff,
            colorClass: 'text-orange-400',
            bgClass: 'bg-orange-500/15 border-orange-500/20',
            borderGlow: 'hover:border-orange-500/40 hover:shadow-[0_0_20px_rgba(249,115,22,0.15)]',
            progressColor: 'bg-gradient-to-r from-orange-500 to-amber-500',
            valueColor: peakLatency > 0 ? getLatencyColorClass(peakLatency) : undefined
        },
        {
            title: 'Fastest Latency',
            value: minLatency > 0 ? `${minLatency.toFixed(1)} ms` : '--',
            subtitle: 'Optimal response execution',
            icon: Zap,
            colorClass: 'text-emerald-400',
            bgClass: 'bg-emerald-500/15 border-emerald-500/20',
            borderGlow: 'hover:border-emerald-500/40 hover:shadow-[0_0_20px_rgba(16,185,129,0.15)]',
            progressColor: 'bg-gradient-to-r from-emerald-400 to-teal-400',
            valueColor: minLatency > 0 ? getLatencyColorClass(minLatency) : undefined
        },
        {
            title: 'Success Rate',
            value: `${successRate.toFixed(1)}%`,
            subtitle: `${stats.successHits.toLocaleString()} success hits`,
            icon: CheckCircle2,
            colorClass: 'text-emerald-400',
            bgClass: 'bg-emerald-500/15 border-emerald-500/20',
            borderGlow: 'hover:border-emerald-500/40 hover:shadow-[0_0_20px_rgba(16,185,129,0.15)]',
            progressColor: 'bg-gradient-to-r from-emerald-500 to-green-500',
        },
        {
            title: 'Error Rate',
            value: `${stats.errorRate.toFixed(1)}%`,
            subtitle: `${stats.errorHits.toLocaleString()} client/server errors`,
            icon: AlertTriangle,
            colorClass: 'text-rose-400',
            bgClass: 'bg-rose-500/15 border-rose-500/20',
            borderGlow: 'hover:border-rose-500/40 hover:shadow-[0_0_20px_rgba(244,63,94,0.15)]',
            progressColor: 'bg-gradient-to-r from-rose-500 to-red-500',
        },
        {
            title: 'Active Services',
            value: stats.uniqueServices.toString(),
            subtitle: 'Provisioned microservices',
            icon: Layers,
            colorClass: 'text-blue-400',
            bgClass: 'bg-blue-500/15 border-blue-500/20',
            borderGlow: 'hover:border-blue-500/40 hover:shadow-[0_0_20px_rgba(59,130,246,0.15)]',
            progressColor: 'bg-gradient-to-r from-blue-500 to-indigo-500',
        },
        {
            title: 'Active Endpoints',
            value: stats.uniqueEndpoints.toString(),
            subtitle: 'Logged API routes',
            icon: Activity,
            colorClass: 'text-cyan-400',
            bgClass: 'bg-cyan-500/15 border-cyan-500/20',
            borderGlow: 'hover:border-cyan-500/40 hover:shadow-[0_0_20px_rgba(6,182,212,0.15)]',
            progressColor: 'bg-gradient-to-r from-cyan-500 to-blue-500',
        },
    ];

    return (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {statCards.map((stat, index) => {
                const Icon = stat.icon;
                return (
                    <Card
                        key={stat.title}
                        className={cn(
                            "relative overflow-hidden group select-none transition-all duration-300 animate-in fade-in slide-in-from-bottom-3",
                            stat.borderGlow
                        )}
                        style={{ animationDelay: `${index * 50}ms` }}
                    >
                        <CardContent className="p-5">
                            <div className="flex items-start justify-between">
                                <div className="space-y-1.5">
                                    <p className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider">
                                        {stat.title}
                                    </p>
                                    <h3 className={cn(
                                        "text-xl font-bold font-mono tracking-tight",
                                        stat.valueColor || "text-foreground"
                                    )}>
                                        {stat.value}
                                    </h3>
                                    <p className="text-[10px] text-muted-foreground font-medium">
                                        {stat.subtitle}
                                    </p>
                                </div>
                                <div className={cn(
                                    "flex items-center justify-center w-10 h-10 rounded-xl border transition-all duration-300 group-hover:scale-110",
                                    stat.bgClass
                                )}>
                                    <Icon className={cn("w-4.5 h-4.5", stat.colorClass)} aria-hidden="true" />
                                </div>
                            </div>

                            {/* Glow accent bottom slide line */}
                            <div className="absolute bottom-0 left-0 right-0 h-1 overflow-hidden opacity-80">
                                <div className={cn("w-full h-full rounded-b-xl transition-all duration-300 group-hover:h-1.5", stat.progressColor)} />
                            </div>
                        </CardContent>
                    </Card>
                );
            })}
        </div>
    );
}

export default StatsGrid;
