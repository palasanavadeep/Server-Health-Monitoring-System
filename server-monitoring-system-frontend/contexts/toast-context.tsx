"use client";

import React, { createContext, useCallback, useContext, useState } from 'react';
import { CheckCircle2, XCircle, Info, X } from 'lucide-react';
import { cn } from '@/lib/utils';

export type ToastType = 'success' | 'error' | 'info';

interface Toast {
    id: number;
    message: string;
    type: ToastType;
}

type AddToastType = (message: string, type?: ToastType, duration?: number) => void;

const ToastContext = createContext<AddToastType | undefined>(undefined);

let _idCounter = 0;

const ICONS = {
    success: CheckCircle2,
    error: XCircle,
    info: Info,
};

function ToastItem({ toast, onRemove }: { toast: Toast; onRemove: (id: number) => void }) {
    const Icon = ICONS[toast.type] ?? Info;

    return (
        <div
            role="status"
            aria-live="polite"
            className={cn(
                "flex items-start gap-3 p-3.5 rounded-lg border shadow-xl min-w-[280px] max-w-[380px] animate-in fade-in slide-in-from-bottom-5 duration-200",
                "glass-panel backdrop-blur-md text-foreground",
                toast.type === 'success' && "border-emerald-500/30 shadow-emerald-500/5",
                toast.type === 'error' && "border-rose-500/30 shadow-rose-500/5",
                toast.type === 'info' && "border-border-color shadow-black/10"
            )}
        >
            <Icon
                size={18}
                className={cn(
                    "flex-shrink-0 mt-0.5",
                    toast.type === 'success' && "text-emerald-500",
                    toast.type === 'error' && "text-rose-500",
                    toast.type === 'info' && "text-cyan-500"
                )}
                aria-hidden="true"
            />
            <span className="flex-1 text-sm leading-relaxed">{toast.message}</span>
            <button
                onClick={() => onRemove(toast.id)}
                aria-label="Dismiss notification"
                className="flex-shrink-0 p-0.5 text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
            >
                <X size={16} aria-hidden="true" />
            </button>
        </div>
    );
}

function ToastContainer({ toasts, onRemove }: { toasts: Toast[]; onRemove: (id: number) => void }) {
    if (toasts.length === 0) return null;

    return (
        <div
            aria-label="Notifications"
            className="fixed bottom-6 right-6 flex flex-col gap-2 z-50 pointer-events-none"
        >
            <div className="flex flex-col gap-2 pointer-events-auto">
                {toasts.map((t) => (
                    <ToastItem key={t.id} toast={t} onRemove={onRemove} />
                ))}
            </div>
        </div>
    );
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
    const [toasts, setToasts] = useState<Toast[]>([]);

    const addToast = useCallback<AddToastType>((message, type = 'info', duration = 3500) => {
        const id = ++_idCounter;
        setToasts((prev) => [...prev, { id, message, type }]);
        setTimeout(() => {
            setToasts((prev) => prev.filter((t) => t.id !== id));
        }, duration);
    }, []);

    const removeToast = useCallback((id: number) => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
    }, []);

    return (
        <ToastContext.Provider value={addToast}>
            {children}
            <ToastContainer toasts={toasts} onRemove={removeToast} />
        </ToastContext.Provider>
    );
}

export function useToast() {
    const ctx = useContext(ToastContext);
    if (!ctx) throw new Error('useToast must be used inside <ToastProvider>');
    return ctx;
}
