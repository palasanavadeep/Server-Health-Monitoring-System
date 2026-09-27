import React from 'react';
import { cn } from '@/lib/utils';
import { StatusDistribution } from '@/lib/api';

interface StatusBreakdownProps {
    distribution?: StatusDistribution;
    successHits?: number;
    errorHits?: number;
    className?: string;
}

export function StatusBreakdown({ 
    distribution, 
    successHits = 0, 
    errorHits = 0, 
    className 
}: StatusBreakdownProps) {
    if (distribution) {
        const s1 = distribution.status_1xx || 0;
        const s2 = distribution.status_2xx || 0;
        const s3 = distribution.status_3xx || 0;
        const s4 = distribution.status_4xx || 0;
        const s5 = distribution.status_5xx || 0;
        const total = s1 + s2 + s3 + s4 + s5;

        const p1 = total > 0 ? (s1 / total) * 100 : 0;
        const p2 = total > 0 ? (s2 / total) * 100 : 0;
        const p3 = total > 0 ? (s3 / total) * 100 : 0;
        const p4 = total > 0 ? (s4 / total) * 100 : 0;
        const p5 = total > 0 ? (s5 / total) * 100 : 0;

        return (
            <div className={cn("space-y-3", className)}>
                {/* 5-class segment bar */}
                <div className="h-2.5 w-full rounded-full bg-[#181D24] overflow-hidden flex shadow-inner">
                    {p1 > 0 && (
                        <div 
                            style={{ width: `${p1}%` }} 
                            className="bg-[#38BDF8] h-full transition-all duration-300" 
                            title={`1xx Info: ${s1} (${p1.toFixed(1)}%)`}
                        />
                    )}
                    {p2 > 0 && (
                        <div 
                            style={{ width: `${p2}%` }} 
                            className="bg-[#48B982] h-full transition-all duration-300" 
                            title={`2xx Success: ${s2} (${p2.toFixed(1)}%)`}
                        />
                    )}
                    {p3 > 0 && (
                        <div 
                            style={{ width: `${p3}%` }} 
                            className="bg-[#A78BFA] h-full transition-all duration-300" 
                            title={`3xx Redirect: ${s3} (${p3.toFixed(1)}%)`}
                        />
                    )}
                    {p4 > 0 && (
                        <div 
                            style={{ width: `${p4}%` }} 
                            className="bg-[#F59E0B] h-full transition-all duration-300" 
                            title={`4xx Client Error: ${s4} (${p4.toFixed(1)}%)`}
                        />
                    )}
                    {p5 > 0 && (
                        <div 
                            style={{ width: `${p5}%` }} 
                            className="bg-[#E45865] h-full transition-all duration-300" 
                            title={`5xx Server Error: ${s5} (${p5.toFixed(1)}%)`}
                        />
                    )}
                    {total === 0 && (
                        <div className="w-full bg-[#242932] h-full" title="No traffic recorded" />
                    )}
                </div>

                {/* 5-class Legend grid */}
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-2 text-xs text-zinc-400">
                    <div className="flex items-center gap-1.5">
                        <span className="w-2 h-2 rounded-full bg-[#48B982] shrink-0" />
                        <span>2xx: <strong className="text-zinc-200 font-mono">{s2}</strong> <span className="text-[10px] text-zinc-500">({p2.toFixed(1)}%)</span></span>
                    </div>
                    <div className="flex items-center gap-1.5">
                        <span className="w-2 h-2 rounded-full bg-[#F59E0B] shrink-0" />
                        <span>4xx: <strong className="text-zinc-200 font-mono">{s4}</strong> <span className="text-[10px] text-zinc-500">({p4.toFixed(1)}%)</span></span>
                    </div>
                    <div className="flex items-center gap-1.5">
                        <span className="w-2 h-2 rounded-full bg-[#E45865] shrink-0" />
                        <span>5xx: <strong className="text-zinc-200 font-mono">{s5}</strong> <span className="text-[10px] text-zinc-500">({p5.toFixed(1)}%)</span></span>
                    </div>
                    {(s3 > 0 || s1 > 0) && (
                        <>
                            {s3 > 0 && (
                                <div className="flex items-center gap-1.5">
                                    <span className="w-2 h-2 rounded-full bg-[#A78BFA] shrink-0" />
                                    <span>3xx: <strong className="text-zinc-200 font-mono">{s3}</strong> <span className="text-[10px] text-zinc-500">({p3.toFixed(1)}%)</span></span>
                                </div>
                            )}
                            {s1 > 0 && (
                                <div className="flex items-center gap-1.5">
                                    <span className="w-2 h-2 rounded-full bg-[#38BDF8] shrink-0" />
                                    <span>1xx: <strong className="text-zinc-200 font-mono">{s1}</strong> <span className="text-[10px] text-zinc-500">({p1.toFixed(1)}%)</span></span>
                                </div>
                            )}
                        </>
                    )}
                </div>
            </div>
        );
    }

    // Fallback: 2-class binary breakdown
    const total = successHits + errorHits;
    const successPct = total > 0 ? (successHits / total) * 100 : 100;
    const errorPct = total > 0 ? (errorHits / total) * 100 : 0;

    return (
        <div className={cn("space-y-2", className)}>
            <div className="h-2 w-full rounded bg-[#181D24] overflow-hidden flex">
                <div 
                    style={{ width: `${successPct}%` }} 
                    className="bg-[#48B982] h-full transition-all duration-300" 
                    title={`Success: ${successHits} (${successPct.toFixed(1)}%)`}
                />
                <div 
                    style={{ width: `${errorPct}%` }} 
                    className="bg-[#E45865] h-full transition-all duration-300" 
                    title={`Errors: ${errorHits} (${errorPct.toFixed(1)}%)`}
                />
            </div>

            <div className="flex items-center justify-between text-xs text-zinc-400">
                <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-xs bg-[#48B982]" />
                    <span>2xx Success: <strong className="text-zinc-200 font-mono">{successHits}</strong> ({successPct.toFixed(1)}%)</span>
                </div>
                <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-xs bg-[#E45865]" />
                    <span>4xx/5xx Errors: <strong className="text-zinc-200 font-mono">{errorHits}</strong> ({errorPct.toFixed(1)}%)</span>
                </div>
            </div>
        </div>
    );
}
