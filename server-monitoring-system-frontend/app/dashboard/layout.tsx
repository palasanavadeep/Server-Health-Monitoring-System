"use client";

import { useEffect, useState } from 'react';
import { useAuth } from '@/contexts/auth-context';
import { useRouter } from 'next/navigation';
import { Sidebar } from '@/components/layout/sidebar';
import { Header } from '@/components/layout/header';
import { RefreshCw } from 'lucide-react';

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
    const { isAuthenticated, loading } = useAuth();
    const router = useRouter();
    const [sidebarOpen, setSidebarOpen] = useState(false);

    useEffect(() => {
        if (!loading && isAuthenticated === false) {
            router.push('/login');
        }
    }, [isAuthenticated, loading, router]);

    if (loading || isAuthenticated === null) {
        return (
            <div className="min-h-screen grid place-items-center bg-[#0B0D10] text-zinc-300">
                <div className="flex flex-col items-center gap-2.5">
                    <RefreshCw className="animate-spin text-[#4CB8D6] w-5 h-5" />
                    <p className="text-xs font-medium text-zinc-400">
                        Loading workspace telemetry...
                    </p>
                </div>
            </div>
        );
    }

    if (isAuthenticated === false) {
        return null;
    }

    return (
        <div className="flex h-screen overflow-hidden bg-[#0B0D10] text-[#F1F3F5]">
            {/* Sidebar Navigation */}
            <Sidebar isOpen={sidebarOpen} onClose={() => setSidebarOpen(false)} />
            
            {/* Application Main Frame */}
            <div className="flex flex-col flex-1 overflow-hidden">
                <Header sidebarOpen={sidebarOpen} setSidebarOpen={setSidebarOpen} />
                
                {/* Scrollable Content Container */}
                <main className="flex-1 overflow-y-auto p-4 sm:p-6 bg-[#0B0D10]">
                    {children}
                </main>
            </div>
        </div>
    );
}
