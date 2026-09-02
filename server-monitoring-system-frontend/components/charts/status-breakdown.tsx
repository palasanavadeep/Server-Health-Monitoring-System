import React from 'react';
import { cn } from '@/lib/utils';

interface StatusBreakdownProps {
    successHits: number;
    errorHits: number;
    className?: string;
}

export function StatusBreakdown({ successHits = 0, errorHits = 0, className }: StatusBreakdownProps) {
    const total = successHits + errorHits;
    const successPct = total > 0 ? (successHits / total) * 100 : 100;
    const errorPct = total > 0 ? (errorHits / total) * 100 : 0;

    return (
        <div className={cn("space-y-2", className)}>
            {/* Split bar */}
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

            {/* Legend row */}
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
