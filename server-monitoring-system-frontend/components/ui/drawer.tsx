"use client";

import React, { useEffect } from 'react';
import { cn } from '@/lib/utils';
import { X } from 'lucide-react';

interface DrawerProps {
    isOpen: boolean;
    onClose: () => void;
    title: string;
    description?: string;
    children: React.ReactNode;
    width?: string;
}

export function Drawer({ isOpen, onClose, title, description, children, width = "max-w-2xl" }: DrawerProps) {
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') onClose();
        };
        if (isOpen) {
            document.body.style.overflow = 'hidden';
            window.addEventListener('keydown', handleKeyDown);
        }
        return () => {
            document.body.style.overflow = 'unset';
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [isOpen, onClose]);

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 overflow-hidden">
            {/* Backdrop */}
            <div 
                className="fixed inset-0 bg-black/60 backdrop-blur-xs transition-opacity duration-200 cursor-pointer"
                onClick={onClose}
                aria-hidden="true"
            />

            <div className="fixed inset-y-0 right-0 flex max-w-full pl-10">
                <div className={cn("w-screen bg-[#0E1014] border-l border-[#242932] shadow-2xl flex flex-col animate-in slide-in-from-right duration-200", width)}>
                    {/* Header */}
                    <div className="p-5 border-b border-[#242932] flex items-center justify-between bg-[#111419]/50">
                        <div>
                            <h2 className="text-base font-semibold text-zinc-100">{title}</h2>
                            {description && <p className="text-xs text-zinc-400 mt-0.5">{description}</p>}
                        </div>
                        <button
                            onClick={onClose}
                            className="p-1.5 rounded hover:bg-white/5 text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer"
                            aria-label="Close drawer"
                        >
                            <X size={16} />
                        </button>
                    </div>

                    {/* Content */}
                    <div className="flex-1 overflow-y-auto p-5 space-y-6">
                        {children}
                    </div>
                </div>
            </div>
        </div>
    );
}
