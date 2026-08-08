"use client";

import { useEffect } from 'react';
import { useRouter } from 'next/navigation';
import { useAuth } from '@/contexts/auth-context';
import { RefreshCw } from 'lucide-react';

export default function GatePage() {
    const { isAuthenticated, loading } = useAuth();
    const router = useRouter();

    useEffect(() => {
        if (!loading) {
            if (isAuthenticated) {
                router.replace('/dashboard');
            } else {
                router.replace('/login');
            }
        }
    }, [isAuthenticated, loading, router]);

    return (
        <div className="min-h-screen grid place-items-center bg-background text-foreground">
            <RefreshCw className="animate-spin text-violet-500 w-8 h-8" />
        </div>
    );
}
