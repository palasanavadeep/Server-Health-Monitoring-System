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
        <div className="min-h-screen grid place-items-center bg-[#0B0D10] text-[#F1F3F5]">
            <RefreshCw className="animate-spin text-[#4CB8D6] w-6 h-6" />
        </div>
    );
}
