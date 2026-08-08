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
            <div className="min-h-screen grid place-items-center bg-background text-foreground">
                <div className="flex flex-col items-center gap-3">
                    <RefreshCw className="animate-spin text-violet-500 w-8 h-8" />
                    <p className="text-sm font-semibold tracking-wider text-muted-foreground animate-pulse">
                        Verifying Secure Session...
                    </p>
                </div>
            </div>
        );
    }

    if (isAuthenticated === false) {
        return null; // Prevents flashing dashboard before router pushes to /login
    }

    return (
        <div className="flex h-screen overflow-hidden bg-background">
            {/* Sidebar Navigation */}
            <Sidebar isOpen={sidebarOpen} onClose={() => setSidebarOpen(false)} />
            
            {/* Application Main Frame */}
            <div className="flex flex-col flex-1 overflow-hidden">
                <Header sidebarOpen={sidebarOpen} setSidebarOpen={setSidebarOpen} />
                
                {/* Scrollable Content Container */}
                <main className="flex-1 overflow-y-auto p-6 grid-bg">
                    {children}
                </main>
            </div>
        </div>
    );
}
