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
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
            {/* Backdrop */}
            <div 
                className="fixed inset-0 bg-black/70 backdrop-blur-xs transition-opacity cursor-pointer"
                onClick={onClose}
                aria-hidden="true"
            />

            {/* Modal Dialog */}
            <div className={cn("relative w-full surface-elevated bg-[#111419] border border-[#242932] shadow-2xl p-6 z-10 animate-in fade-in zoom-in-95 duration-150", maxWidth)}>
                <div className="flex items-start justify-between pb-4 border-b border-[#242932]">
                    <div>
                        <h3 className="text-base font-semibold text-zinc-100">{title}</h3>
                        {description && <p className="text-xs text-zinc-400 mt-1">{description}</p>}
                    </div>
                    <button
                        onClick={onClose}
                        className="p-1 rounded hover:bg-white/5 text-zinc-400 hover:text-zinc-200 transition-colors cursor-pointer"
                        aria-label="Close modal"
                    >
                        <X size={16} />
                    </button>
                </div>

                <div className="pt-4">
                    {children}
                </div>
            </div>
        </div>
    );
}
