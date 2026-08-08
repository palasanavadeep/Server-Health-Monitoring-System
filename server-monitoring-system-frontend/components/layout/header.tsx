"use client";

import React, { useState, useEffect } from 'react';
import { Menu, X, LogOut, Clock, RefreshCw, User } from 'lucide-react';
import { useQueryClient, useIsFetching } from '@tanstack/react-query';
import { useAuth } from '@/contexts/auth-context';
import { useDashboardQuery, QUERY_KEYS } from '@/hooks/use-dashboard-queries';
import { Button } from '@/components/ui/button';

interface HeaderProps {
    sidebarOpen: boolean;
    setSidebarOpen: (open: boolean) => void;
}

export function Header({ sidebarOpen, setSidebarOpen }: HeaderProps) {
    const { logout, user } = useAuth();
    const queryClient = useQueryClient();
    
    // Check if any queries are currently fetching in the background
    const isFetching = useIsFetching({ queryKey: QUERY_KEYS.DASHBOARD }) > 0;
    const { dataUpdatedAt } = useDashboardQuery();

    const [lastUpdated, setLastUpdated] = useState<string>('--');

    useEffect(() => {
        if (dataUpdatedAt) {
            setLastUpdated(new Date(dataUpdatedAt).toLocaleTimeString());
        }
    }, [dataUpdatedAt]);

    const handleRefresh = () => {
        queryClient.invalidateQueries({ queryKey: QUERY_KEYS.DASHBOARD });
    };

    return (
        <header className="h-16 border-b border-border-color bg-glass-card/50 backdrop-blur-md sticky top-0 z-30">
            <div className="h-full px-6 flex items-center justify-between">
                {/* Mobile Hamburger Menu Toggle */}
                <button
                    className="lg:hidden p-2 text-muted-foreground hover:text-foreground cursor-pointer rounded-lg hover:bg-white/5 transition-colors"
                    onClick={() => setSidebarOpen(!sidebarOpen)}
                    aria-label={sidebarOpen ? 'Close menu' : 'Open menu'}
                >
                    {sidebarOpen ? <X size={20} aria-hidden="true" /> : <Menu size={20} aria-hidden="true" />}
                </button>

                {/* Left Section: Time tracker */}
                <div className="flex items-center gap-2 text-xs text-muted-foreground bg-glass-card border border-border-color/50 px-3 py-1.5 rounded-full select-none shadow-sm">
                    <Clock size={13} className="text-cyan-400" aria-hidden="true" />
                    <span>Last synced: {lastUpdated}</span>
                </div>

                {/* Right Section: Actions & User Menu */}
                <div className="flex items-center gap-3">
                    {/* Manual Invalidate & Sync */}
                    <Button
                        variant="secondary"
                        size="sm"
                        onClick={handleRefresh}
                        disabled={isFetching}
                        aria-label="Refresh telemetry data"
                        className="h-8 gap-1.5 cursor-pointer text-xs"
                    >
                        <RefreshCw
                            size={12}
                            className={isFetching ? "animate-spin text-cyan-400" : ""}
                            aria-hidden="true"
                        />
                        <span className="hidden sm:inline">Sync</span>
                    </Button>

                    {/* Active profile badge */}
                    {user && (
                        <div className="hidden sm:flex items-center gap-2 px-3 py-1 border border-border-color/50 rounded-lg bg-glass-card/40 text-xs font-semibold text-muted-foreground">
                            <User size={12} className="text-cyan-400" />
                            <span>{user.username}</span>
                            <span className="text-[10px] uppercase font-bold bg-cyan-500/20 text-cyan-400 px-1 rounded">
                                {user.role}
                            </span>
                        </div>
                    )}


                    {/* Exit System Button */}
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={logout}
                        aria-label="Log out"
                        className="h-8 w-8 text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 cursor-pointer"
                    >
                        <LogOut size={16} aria-hidden="true" />
                    </Button>
                </div>
            </div>
        </header>
    );
}
