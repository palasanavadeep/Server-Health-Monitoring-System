import React from 'react';
import { cn } from '@/lib/utils';

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: 'default' | 'secondary' | 'outline' | 'ghost' | 'danger' | 'success';
    size?: 'sm' | 'md' | 'lg' | 'icon';
    isLoading?: boolean;
}

export function Button({
    className,
    variant = 'default',
    size = 'md',
    isLoading,
    children,
    disabled,
    ...props
}: ButtonProps) {
    return (
        <button
            disabled={disabled || isLoading}
            className={cn(
                "inline-flex items-center justify-center font-medium rounded-lg transition-all duration-200 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
                
                // Variants
                variant === 'default' && "bg-cyan-600 hover:bg-cyan-500 text-white shadow-lg shadow-cyan-500/10 hover:shadow-cyan-500/20 active:scale-98",
                variant === 'secondary' && "bg-glass-card hover:bg-glass-card-hover border border-border-color text-foreground shadow-sm",
                variant === 'outline' && "border border-border-color hover:bg-cyan-500/10 hover:border-cyan-500/50 text-foreground",
                variant === 'ghost' && "hover:bg-cyan-500/10 text-muted-foreground hover:text-foreground",
                variant === 'danger' && "bg-rose-600 hover:bg-rose-500 text-white shadow-lg shadow-rose-500/10 active:scale-98",
                variant === 'success' && "bg-emerald-600 hover:bg-emerald-500 text-white shadow-lg shadow-emerald-500/10 active:scale-98",
                
                // Sizes
                size === 'sm' && "h-8 px-3 text-xs",
                size === 'md' && "h-10 px-4 py-2 text-sm",
                size === 'lg' && "h-12 px-6 text-base",
                size === 'icon' && "h-10 w-10 p-0",
                
                className
            )}
            {...props}
        >
            {isLoading ? (
                <svg className="animate-spin -ml-1 mr-2 h-4 w-4 text-current" fill="none" viewBox="0 0 24 24">
                    <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                    <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                </svg>
            ) : null}
            {children}
        </button>
    );
}
