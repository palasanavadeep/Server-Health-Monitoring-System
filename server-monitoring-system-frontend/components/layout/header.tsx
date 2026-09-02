"use client";
import React from 'react';
import { Menu, X, Search, Bell } from 'lucide-react';
import { useQueryClient, useIsFetching } from '@tanstack/react-query';
import { useAuth } from '@/contexts/auth-context';
import { LiveIndicator } from '@/components/ui/live-indicator';

interface HeaderProps {
    sidebarOpen: boolean;
    setSidebarOpen: (open: boolean) => void;
}

export function Header({ sidebarOpen, setSidebarOpen }: HeaderProps) {
    const { user } = useAuth();
    const queryClient = useQueryClient();
    const isFetching = useIsFetching() > 0;

    const handleRefresh = () => {
        queryClient.invalidateQueries();
    };

    return (
        <header className="h-14 border-b border-[#242932] bg-[#0E1014] sticky top-0 z-30 flex items-center justify-between px-4 sm:px-6">
            {/* Left: Mobile Toggle */}
            <div className="flex items-center gap-3">
                <button
                    className="lg:hidden p-1.5 text-zinc-400 hover:text-zinc-100 cursor-pointer rounded hover:bg-white/5 transition-colors"
                    onClick={() => setSidebarOpen(!sidebarOpen)}
                    aria-label={sidebarOpen ? 'Close menu' : 'Open menu'}
                >
                    {sidebarOpen ? <X size={18} /> : <Menu size={18} />}
                </button>
            </div>

            {/* Center: Quick Search Bar */}
            <div className="hidden md:flex items-center max-w-sm w-full mx-4">
                <div className="relative w-full">
                    <Search size={13} className="absolute left-2.5 top-2.5 text-zinc-400" />
                    <input
                        type="text"
                        placeholder="Search routes, services, operators... ⌘K"
                        className="w-full bg-[#111419] border border-[#242932] rounded px-8 py-1 text-xs text-zinc-200 placeholder:text-zinc-400 focus:outline-none focus:border-[#4CB8D6] transition-colors"
                    />
                </div>
            </div>

            {/* Right: Freshness + Alerts + Avatar */}
            <div className="flex items-center gap-3">
                {/* Live Freshness Indicator */}
                <LiveIndicator onRefresh={handleRefresh} isFetching={isFetching} />

                {/* Notification Bell */}
                <button
                    className="p-1.5 rounded hover:bg-[#15191F] text-zinc-400 hover:text-zinc-200 transition-colors relative cursor-pointer"
                    title="Alerts & Notifications"
                    aria-label="Alerts"
                >
                    <Bell size={14} />
                    <span className="absolute top-1 right-1 w-1.5 h-1.5 bg-[#4CB8D6] rounded-full" />
                </button>

                {/* User Avatar */}
                <div className="w-6 h-6 rounded bg-[#181D24] border border-[#242932] flex items-center justify-center text-[10px] font-bold text-zinc-300 select-none uppercase">
                    {user?.username?.substring(0, 2) || 'OP'}
                </div>
            </div>
        </header>
    );
}
