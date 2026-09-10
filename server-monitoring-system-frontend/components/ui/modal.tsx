"use client";

import React, { useEffect } from 'react';
import { cn } from '@/lib/utils';
import { X } from 'lucide-react';

interface ModalProps {
    isOpen: boolean;
    onClose: () => void;
    title: string;
    description?: string;
    children: React.ReactNode;
    maxWidth?: string;
}

export function Modal({ isOpen, onClose, title, description, children, maxWidth = "max-w-lg" }: ModalProps) {
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
        <div className="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 md:p-6 overflow-hidden">
            {/* Backdrop */}
            <div 
                className="fixed inset-0 bg-black/75 backdrop-blur-xs transition-opacity cursor-pointer"
                onClick={onClose}
                aria-hidden="true"
            />

            {/* Modal Dialog */}
            <div className={cn(
                "relative w-full max-h-[calc(100vh-2rem)] sm:max-h-[calc(100vh-3.5rem)] flex flex-col surface-elevated bg-[#111419] border border-[#242932] rounded-xl shadow-2xl z-10 animate-in fade-in zoom-in-95 duration-150 overflow-hidden",
                maxWidth
            )}>
                {/* Header (fixed at top of modal) */}
                <div className="flex items-start justify-between p-4 sm:p-5 pb-3 sm:pb-4 border-b border-[#242932] shrink-0 bg-[#111419]">
                    <div className="pr-4">
                        <h3 className="text-base font-semibold text-zinc-100">{title}</h3>
                        {description && <p className="text-xs text-zinc-400 mt-0.5 leading-relaxed">{description}</p>}
                    </div>
                    <button
                        onClick={onClose}
                        className="p-1.5 rounded-md hover:bg-white/5 text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer shrink-0"
                        aria-label="Close modal"
                    >
                        <X size={16} />
                    </button>
                </div>

                {/* Content (vertically responsive and scrollable) */}
                <div className="p-4 sm:p-5 overflow-y-auto flex-1 overscroll-contain">
                    {children}
                </div>
            </div>
        </div>
    );
}
