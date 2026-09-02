"use client";

import React, { useState, useEffect, useCallback } from 'react';
import { cn } from '@/lib/utils';
import { RefreshCw } from 'lucide-react';

interface LiveIndicatorProps {
    onRefresh?: () => void;
    isFetching?: boolean;
    className?: string;
    autoRefreshIntervalMs?: number; // Defaults to 60,000ms (1 minute)
}

function formatTimeAgo(seconds: number): string {
    if (seconds < 5) return 'Updated just now';
    if (seconds < 60) return `Updated ${seconds}s ago`;
    
    const minutes = Math.floor(seconds / 60);
    const remainingSecs = seconds % 60;
    
    if (minutes < 60) {
        return remainingSecs > 0 && minutes < 3
            ? `Updated ${minutes}m ${remainingSecs}s ago`
            : `Updated ${minutes}m ago`;
    }
    
    const hours = Math.floor(minutes / 60);
    const remainingMins = minutes % 60;
    
    return remainingMins > 0 
        ? `Updated ${hours}h ${remainingMins}m ago`
        : `Updated ${hours}h ago`;
}

export function LiveIndicator({ 
    onRefresh, 
    isFetching, 
    className,
    autoRefreshIntervalMs = 60000 
}: LiveIndicatorProps) {
    const [secondsAgo, setSecondsAgo] = useState(0);

    const triggerRefresh = useCallback(() => {
        setSecondsAgo(0);
        onRefresh?.();
    }, [onRefresh]);

    // 1-second ticker + auto-refresh at interval (default 60s)
    useEffect(() => {
        const autoRefreshSecs = Math.max(10, Math.floor(autoRefreshIntervalMs / 1000));
        
        const timer = setInterval(() => {
            setSecondsAgo((prev) => {
                if (prev + 1 >= autoRefreshSecs) {
                    onRefresh?.();
                    return 0;
                }
                return prev + 1;
            });
        }, 1000);

        return () => clearInterval(timer);
    }, [onRefresh, autoRefreshIntervalMs]);

    // Reset timer whenever a fetch finishes
    useEffect(() => {
        if (!isFetching) {
            setSecondsAgo(0);
        }
    }, [isFetching]);

    return (
        <div 
            className={cn(
                "inline-flex items-center gap-2 px-2.5 py-1 rounded border border-[#242932] bg-[#111419] text-xs select-none shadow-xs",
                className
            )}
            title="Auto-refreshes every 1 minute"
        >
            <div className="flex items-center gap-1.5">
                <span className={cn(
                    "w-2 h-2 rounded-full",
                    isFetching ? "bg-[#4CB8D6] animate-spin" : "bg-[#48B982] animate-pulse"
                )} />
                <span className="font-semibold text-zinc-300">Live</span>
            </div>
            <span className="text-zinc-600">•</span>
            <span className="text-zinc-400 text-[11px] font-mono whitespace-nowrap">
                {formatTimeAgo(secondsAgo)}
            </span>
            {onRefresh && (
                <button
                    onClick={triggerRefresh}
                    disabled={isFetching}
                    className="p-1 rounded hover:bg-white/5 text-zinc-400 hover:text-zinc-200 transition-colors ml-0.5 cursor-pointer disabled:opacity-50"
                    title="Click to refresh telemetry now"
                    aria-label="Refresh telemetry"
                >
                    <RefreshCw 
                        size={12} 
                        className={cn("transition-transform", isFetching && "animate-spin text-[#4CB8D6]")} 
                    />
                </button>
            )}
        </div>
    );
}
