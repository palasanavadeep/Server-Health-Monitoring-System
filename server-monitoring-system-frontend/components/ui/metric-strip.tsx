import React from 'react';
import { cn } from '@/lib/utils';

export interface MetricItem {
    label: string;
    value: string | number;
    subtext?: string;
    status?: 'neutral' | 'healthy' | 'warning' | 'critical';
    trend?: string;
}

interface MetricStripProps {
    items: MetricItem[];
    className?: string;
}

export function MetricStrip({ items, className }: MetricStripProps) {
    return (
        <div className={cn("surface-panel grid grid-cols-2 md:grid-cols-4 divide-y md:divide-y-0 md:divide-x divide-[#242932]", className)}>
            {items.map((item, idx) => {
                let valueColor = "text-zinc-100";
                let badgeColor = "text-zinc-400";

                if (item.status === 'healthy') {
                    valueColor = "text-[#48B982]";
                    badgeColor = "text-[#48B982]";
                } else if (item.status === 'warning') {
                    valueColor = "text-[#D99A3D]";
                    badgeColor = "text-[#D99A3D]";
                } else if (item.status === 'critical') {
                    valueColor = "text-[#E45865]";
                    badgeColor = "text-[#E45865]";
                }

                return (
                    <div key={idx} className="p-4 sm:p-5 flex flex-col justify-between">
                        <span className="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider">
                            {item.label}
                        </span>
                        
                        <div className="mt-2 flex items-baseline gap-2">
                            <span className={cn("text-2xl sm:text-3xl font-bold tracking-tight font-mono", valueColor)}>
                                {item.value}
                            </span>
                            {item.trend && (
                                <span className={cn("text-xs font-mono font-medium", badgeColor)}>
                                    {item.trend}
                                </span>
                            )}
                        </div>

                        {item.subtext && (
                            <span className="text-xs text-zinc-500 mt-1 block">
                                {item.subtext}
                            </span>
                        )}
                    </div>
                );
            })}
        </div>
    );
}
