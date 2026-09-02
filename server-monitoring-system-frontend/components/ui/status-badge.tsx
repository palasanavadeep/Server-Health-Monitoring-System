import React from 'react';
import { cn } from '@/lib/utils';

export type HealthStatus = 'healthy' | 'degraded' | 'critical' | 'offline' | 'unknown';

interface StatusBadgeProps {
    status: HealthStatus | string;
    label?: string;
    size?: 'sm' | 'md';
    className?: string;
}

export function StatusBadge({ status, label, size = 'sm', className }: StatusBadgeProps) {
    const normalized = (status || 'unknown').toLowerCase();

    let dotColor = 'bg-zinc-500';
    let textColor = 'text-zinc-400';
    let defaultLabel = 'Unknown';
    let symbol = '○';

    if (normalized === 'healthy' || normalized === 'active' || normalized === 'operational' || normalized === 'ok') {
        dotColor = 'bg-[#48B982]';
        textColor = 'text-[#48B982]';
        defaultLabel = 'Healthy';
        symbol = '●';
    } else if (normalized === 'degraded' || normalized === 'warning' || normalized === 'elevated') {
        dotColor = 'bg-[#D99A3D]';
        textColor = 'text-[#D99A3D]';
        defaultLabel = 'Degraded';
        symbol = '▲';
    } else if (normalized === 'critical' || normalized === 'down' || normalized === 'error' || normalized === 'failed') {
        dotColor = 'bg-[#E45865]';
        textColor = 'text-[#E45865]';
        defaultLabel = 'Critical';
        symbol = '!';
    } else if (normalized === 'offline' || normalized === 'disabled' || normalized === 'inactive') {
        dotColor = 'bg-zinc-500';
        textColor = 'text-zinc-400';
        defaultLabel = 'Offline';
        symbol = '○';
    }

    return (
        <span
            className={cn(
                "inline-flex items-center gap-1.5 font-medium select-none",
                size === 'sm' ? "text-xs" : "text-sm",
                textColor,
                className
            )}
        >
            <span className="text-[10px] leading-none" aria-hidden="true">{symbol}</span>
            <span className="text-zinc-200">{label || defaultLabel}</span>
        </span>
    );
}

export function HealthDot({ status, className }: { status: HealthStatus | string; className?: string }) {
    const normalized = (status || 'unknown').toLowerCase();
    let dotColor = 'bg-zinc-500';

    if (normalized === 'healthy' || normalized === 'active' || normalized === 'operational') {
        dotColor = 'bg-[#48B982]';
    } else if (normalized === 'degraded' || normalized === 'warning') {
        dotColor = 'bg-[#D99A3D]';
    } else if (normalized === 'critical' || normalized === 'down' || normalized === 'error') {
        dotColor = 'bg-[#E45865]';
    }

    return <span className={cn("inline-block w-2 h-2 rounded-full", dotColor, className)} />;
}
