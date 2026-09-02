"use client";

import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { Button } from '@/components/ui/button';
import { ShieldAlert, ArrowLeft, Home, Radio } from 'lucide-react';

export default function NotFound() {
    const router = useRouter();
    const { user, isAuthenticated } = useAuth();

    const getRedirectPath = () => {
        if (!isAuthenticated) return '/login';
        if (user?.role === 'super_admin') return '/dashboard/tenants';
        return '/dashboard';
    };

    return (
        <div className="min-h-screen flex flex-col items-center justify-center bg-[#0B0D10] text-[#F1F3F5] p-6 select-none">
            <div className="max-w-md w-full text-center space-y-5 surface-panel p-8 bg-[#111419] border border-[#242932]">
                {/* Icon */}
                <div className="mx-auto w-12 h-12 rounded bg-[#E45865]/10 border border-[#E45865]/20 text-[#E45865] flex items-center justify-center">
                    <ShieldAlert size={22} />
                </div>

                {/* Status code & title */}
                <div className="space-y-1.5">
                    <span className="text-[10px] font-mono font-bold tracking-widest text-[#E45865] uppercase bg-[#E45865]/10 px-2 py-0.5 rounded border border-[#E45865]/20">
                        Error 404 / Access Restricted
                    </span>
                    <h1 className="text-xl font-semibold tracking-tight text-zinc-100">
                        Page Not Found
                    </h1>
                    <p className="text-xs text-zinc-400 leading-relaxed">
                        The requested resource does not exist or your active security credentials do not have authorization to view this page.
                    </p>
                </div>

                {/* Actions */}
                <div className="flex flex-col sm:flex-row items-center justify-center gap-2 pt-2">
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={() => router.back()}
                        className="w-full sm:w-auto text-xs h-8 gap-1.5 cursor-pointer"
                    >
                        <ArrowLeft size={13} />
                        Go Back
                    </Button>
                    <Link href={getRedirectPath()} className="w-full sm:w-auto">
                        <Button
                            size="sm"
                            className="w-full text-xs h-8 gap-1.5 cursor-pointer bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold"
                        >
                            <Home size={13} />
                            Dashboard
                        </Button>
                    </Link>
                </div>

                {/* Footer brand */}
                <div className="pt-4 border-t border-[#242932] flex items-center justify-center gap-1.5 text-[11px] text-zinc-500 font-mono">
                    <Radio size={11} className="text-[#4CB8D6]" />
                    <span>Telemetry Core</span>
                </div>
            </div>
        </div>
    );
}
