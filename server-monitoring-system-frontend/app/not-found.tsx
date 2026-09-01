"use client";

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { Button } from '@/components/ui/button';
import { ShieldAlert, ArrowLeft, Home, Zap } from 'lucide-react';

export default function NotFound() {
    const router = useRouter();
    const { user, isAuthenticated } = useAuth();

    const getRedirectPath = () => {
        if (!isAuthenticated) return '/login';
        if (user?.role === 'super_admin') return '/dashboard/tenants';
        return '/dashboard';
    };

    return (
        <div className="min-h-screen flex flex-col items-center justify-center bg-background text-foreground p-6 relative overflow-hidden grid-bg">
            {/* Ambient background glow */}
            <div className="absolute w-96 h-96 bg-cyan-500/10 rounded-full blur-3xl pointer-events-none -top-20 -left-20" />
            <div className="absolute w-96 h-96 bg-rose-500/10 rounded-full blur-3xl pointer-events-none -bottom-20 -right-20" />

            <div className="max-w-md w-full text-center space-y-6 relative z-10 glass-panel p-8 rounded-2xl border border-border-color bg-glass-card/90 shadow-2xl backdrop-blur-xl">
                {/* Icon */}
                <div className="mx-auto w-16 h-16 rounded-2xl bg-rose-500/10 border border-rose-500/20 text-rose-400 flex items-center justify-center shadow-lg shadow-rose-500/5">
                    <ShieldAlert size={32} />
                </div>

                {/* Status code & title */}
                <div className="space-y-2">
                    <span className="text-xs font-mono font-bold tracking-widest text-rose-400 uppercase bg-rose-500/10 px-2.5 py-1 rounded border border-rose-500/20">
                        Error 404 / Access Denied
                    </span>
                    <h1 className="text-2xl font-extrabold tracking-tight text-foreground">
                        Page Not Found
                    </h1>
                    <p className="text-xs text-muted-foreground leading-relaxed">
                        The requested resource does not exist or your active security credentials do not have authorization to access this page.
                    </p>
                </div>

                {/* Actions */}
                <div className="flex flex-col sm:flex-row items-center justify-center gap-3 pt-2">
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={() => router.back()}
                        className="w-full sm:w-auto text-xs h-9 gap-1.5 cursor-pointer"
                    >
                        <ArrowLeft size={13} />
                        Go Back
                    </Button>
                    <Link href={getRedirectPath()} className="w-full sm:w-auto">
                        <Button
                            size="sm"
                            className="w-full text-xs h-9 gap-1.5 cursor-pointer"
                        >
                            <Home size={13} />
                            Return to Dashboard
                        </Button>
                    </Link>
                </div>

                {/* Brand watermark */}
                <div className="pt-4 border-t border-border-color/30 flex items-center justify-center gap-2 text-[10px] text-muted-foreground">
                    <Zap size={11} className="text-cyan-400" />
                    <span>Telemetry Core • Access Control Engine</span>
                </div>
            </div>
        </div>
    );
}
