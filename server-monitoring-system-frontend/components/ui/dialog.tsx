import React, { useEffect } from 'react';
import { X } from 'lucide-react';
import { cn } from '@/lib/utils';

interface DialogProps {
    isOpen: boolean;
    onClose: () => void;
    title: string;
    children: React.ReactNode;
}

export function Dialog({ isOpen, onClose, title, children }: DialogProps) {
    // Disable scrolling when dialog is open
    useEffect(() => {
        if (isOpen) {
            document.body.style.overflow = 'hidden';
        } else {
            document.body.style.overflow = 'unset';
        }
        return () => {
            document.body.style.overflow = 'unset';
        };
    }, [isOpen]);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* Backdrop */}
            <div 
                className="fixed inset-0 bg-black/60 backdrop-blur-sm cursor-pointer transition-opacity duration-300"
                onClick={onClose}
            />
            
            {/* Dialog Content */}
            <div className="relative w-full max-w-md transform rounded-xl border border-border-color bg-glass-card p-6 shadow-2xl backdrop-blur-lg animate-in fade-in zoom-in-95 duration-200">
                {/* Header */}
                <div className="flex items-center justify-between pb-4 border-b border-border-color/50">
                    <h3 className="text-lg font-bold text-foreground">{title}</h3>
                    <button
                        onClick={onClose}
                        className="rounded-md p-1 text-muted-foreground hover:bg-white/5 hover:text-foreground cursor-pointer transition-colors"
                        aria-label="Close dialog"
                    >
                        <X size={18} />
                    </button>
                </div>
                
                {/* Body */}
                <div className="mt-4">
                    {children}
                </div>
            </div>
        </div>
    );
}
