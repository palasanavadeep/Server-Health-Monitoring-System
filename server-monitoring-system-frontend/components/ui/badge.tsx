import React from 'react';
import { cn } from '@/lib/utils';

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
    variant?: 'default' | 'secondary' | 'outline' | 'success' | 'warning' | 'destructive' | 'info';
}

export function Badge({ className, variant = 'default', children, ...props }: BadgeProps) {
    return (
        <span
            className={cn(
                "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold tracking-wide border transition-colors select-none",
                
                variant === 'default' && "bg-cyan-500/10 text-cyan-400 border-cyan-500/20",
                variant === 'secondary' && "bg-glass-card text-foreground border-border-color",
                variant === 'outline' && "bg-transparent text-muted-foreground border-border-color",
                variant === 'success' && "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
                variant === 'warning' && "bg-amber-500/10 text-amber-400 border-amber-500/20",
                variant === 'destructive' && "bg-rose-500/10 text-rose-400 border-rose-500/20",
                variant === 'info' && "bg-cyan-500/10 text-cyan-400 border-cyan-500/20",
                
                className
            )}
            {...props}
        >
            {children}
        </span>
    );
}
