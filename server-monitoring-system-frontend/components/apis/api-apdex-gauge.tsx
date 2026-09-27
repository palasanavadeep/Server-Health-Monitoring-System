"use client";

import React from 'react';
import { Gauge, CheckCircle2, AlertTriangle, XCircle, Info } from 'lucide-react';
import { cn } from '@/lib/utils';
import { ApdexScore } from '@/lib/api';

interface ApdexGaugeProps {
    apdex?: ApdexScore;
    apdexThresholdMs?: number;
    className?: string;
}

export function ApdexRadialGauge({
    apdex,
    apdexThresholdMs = 500,
    className = "",
}: ApdexGaugeProps) {
    const score = apdex?.score ?? 1.0;
    const satisfied = apdex?.satisfied ?? 0;
    const tolerating = apdex?.tolerating ?? 0;
    const frustrated = apdex?.frustrated ?? 0;
    const total = satisfied + tolerating + frustrated || 1;

    const satisfiedPct = Math.round((satisfied / total) * 100);
    const toleratingPct = Math.round((tolerating / total) * 100);
    const frustratedPct = Math.round((frustrated / total) * 100);

    // Color and label resolution
    let ratingColor = '#48B982'; // Emerald
    let ratingLabel = 'Excellent';
    let ratingBg = 'bg-[#48B982]/10 text-[#48B982] border-[#48B982]/30';

    if (score < 0.70) {
        ratingColor = '#E45865';
        ratingLabel = 'Poor';
        ratingBg = 'bg-[#E45865]/10 text-[#E45865] border-[#E45865]/30';
    } else if (score < 0.85) {
        ratingColor = '#F59E0B';
        ratingLabel = 'Fair';
        ratingBg = 'bg-[#F59E0B]/10 text-[#F59E0B] border-[#F59E0B]/30';
    } else if (score < 0.94) {
        ratingColor = '#4CB8D6';
        ratingLabel = 'Good';
        ratingBg = 'bg-[#4CB8D6]/10 text-[#4CB8D6] border-[#4CB8D6]/30';
    }

    // Semi-circular SVG gauge parameters
    // Radius: 70, Circumference of half circle = PI * 70 = 219.9
    const radius = 70;
    const halfCircumference = Math.PI * radius;
    const strokeDashoffset = halfCircumference * (1 - Math.min(1, Math.max(0, score)));

    return (
        <div className={cn("surface-panel p-5 flex flex-col justify-between space-y-4", className)}>
            {/* Header */}
            <div className="flex items-center justify-between pb-3 border-b border-[#242932]">
                <div>
                    <h3 className="text-xs font-semibold text-zinc-200 uppercase tracking-wider flex items-center gap-2">
                        <Gauge size={14} className="text-[#48B982]" />
                        Apdex SLA Satisfaction Index
                    </h3>
                    <p className="text-[11px] text-zinc-400 mt-0.5">
                        User satisfaction index measured against target threshold T = {apdexThresholdMs}ms
                    </p>
                </div>
                <span className={cn("px-2.5 py-0.5 rounded text-xs font-mono font-bold border", ratingBg)}>
                    {ratingLabel}
                </span>
            </div>

            {/* Circular Arc Meter */}
            <div className="flex flex-col items-center justify-center py-2 relative">
                <svg width="200" height="115" viewBox="0 0 200 115" className="overflow-visible">
                    <defs>
                        <linearGradient id="apdexGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                            <stop offset="0%" stopColor="#E45865" />
                            <stop offset="50%" stopColor="#F59E0B" />
                            <stop offset="85%" stopColor="#4CB8D6" />
                            <stop offset="100%" stopColor="#48B982" />
                        </linearGradient>
                    </defs>

                    {/* Track Background */}
                    <path
                        d="M 25 105 A 75 75 0 0 1 175 105"
                        fill="none"
                        stroke="#1E232B"
                        strokeWidth="14"
                        strokeLinecap="round"
                    />

                    {/* Active Gauge Fill */}
                    <path
                        d="M 25 105 A 75 75 0 0 1 175 105"
                        fill="none"
                        stroke="url(#apdexGradient)"
                        strokeWidth="14"
                        strokeLinecap="round"
                        strokeDasharray={halfCircumference}
                        strokeDashoffset={strokeDashoffset}
                        className="transition-all duration-700 ease-out"
                    />
                </svg>

                {/* Centered Score Display */}
                <div className="absolute top-[52px] flex flex-col items-center justify-center text-center">
                    <span className="text-3xl font-bold font-mono text-zinc-100 tracking-tight">
                        {score.toFixed(2)}
                    </span>
                    <span className="text-[10px] font-mono text-zinc-500 uppercase tracking-wider">
                        SLA Score (0-1)
                    </span>
                </div>
            </div>

            {/* 3-Tier Breakdown Cards */}
            <div className="grid grid-cols-3 gap-2.5 pt-1 text-center">
                <div className="p-2.5 rounded bg-[#0E1014] border border-[#48B982]/25 space-y-1">
                    <div className="flex items-center justify-center gap-1 text-[10px] text-[#48B982] font-semibold uppercase">
                        <CheckCircle2 size={11} />
                        <span>Satisfied</span>
                    </div>
                    <span className="text-base font-bold font-mono text-zinc-100 block">
                        {satisfied.toLocaleString()}
                    </span>
                    <span className="text-[10px] text-zinc-500 font-mono block">
                        ≤ {apdexThresholdMs}ms ({satisfiedPct}%)
                    </span>
                </div>

                <div className="p-2.5 rounded bg-[#0E1014] border border-[#F59E0B]/25 space-y-1">
                    <div className="flex items-center justify-center gap-1 text-[10px] text-[#F59E0B] font-semibold uppercase">
                        <AlertTriangle size={11} />
                        <span>Tolerating</span>
                    </div>
                    <span className="text-base font-bold font-mono text-zinc-100 block">
                        {tolerating.toLocaleString()}
                    </span>
                    <span className="text-[10px] text-zinc-500 font-mono block">
                        ≤ {apdexThresholdMs * 4}ms ({toleratingPct}%)
                    </span>
                </div>

                <div className="p-2.5 rounded bg-[#0E1014] border border-[#E45865]/25 space-y-1">
                    <div className="flex items-center justify-center gap-1 text-[10px] text-[#E45865] font-semibold uppercase">
                        <XCircle size={11} />
                        <span>Frustrated</span>
                    </div>
                    <span className="text-base font-bold font-mono text-zinc-100 block">
                        {frustrated.toLocaleString()}
                    </span>
                    <span className="text-[10px] text-zinc-500 font-mono block">
                        &gt; {apdexThresholdMs * 4}ms ({frustratedPct}%)
                    </span>
                </div>
            </div>
        </div>
    );
}
