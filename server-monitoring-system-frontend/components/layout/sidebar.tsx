"use client";

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { cn } from '@/lib/utils';
import { useAuth } from '@/contexts/auth-context';
import {
    LayoutDashboard,
    Settings,
    Zap,
    KeyRound,
    Braces,
    LogOut,
} from 'lucide-react';

const navItems = [
    {
        title: 'Overview',
        href: '/dashboard',
        icon: LayoutDashboard,
        description: 'Main metrics dashboard'
    },
    {
        title: 'APIs & Latencies',
        href: '/dashboard/apis',
        icon: Braces,
        description: 'API endpoint performance'
    },
    {
        title: 'Clients & Keys',
        href: '/dashboard/clients',
        icon: KeyRound,
        description: 'Manage clients and credentials'
    },
];

const bottomNavItems = [
    {
        title: 'Settings',
        href: '/dashboard/settings',
        icon: Settings,
        description: 'App settings'
    },
];

interface SidebarProps {
    isOpen: boolean;
    onClose: () => void;
}

export function Sidebar({ isOpen, onClose }: SidebarProps) {
    const pathname = usePathname();
    const { user, logout } = useAuth();

    return (
        <>
            {isOpen && (
                <div
                    className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm lg:hidden cursor-pointer"
                    onClick={onClose}
                    aria-hidden="true"
                />
            )}
            <aside
                className={cn(
                    "fixed top-0 bottom-0 left-0 z-40 w-64 border-r border-border-color bg-glass-card/90 backdrop-blur-xl transition-transform duration-300 lg:static lg:translate-x-0",
                    isOpen ? "translate-x-0" : "-translate-x-full"
                )}
                aria-label="Sidebar"
                aria-expanded={isOpen}
            >
                <div className="flex flex-col h-full p-5 justify-between">
                    {/* Top: Logo & Nav */}
                    <div className="space-y-6">
                        {/* Logo Section */}
                        <div className="flex items-center gap-3 px-1">
                            <div className="flex items-center justify-center w-9 h-9 rounded-xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 shadow-md shadow-cyan-500/5 animate-pulse-slow">
                                <Zap size={18} aria-hidden="true" />
                            </div>
                            <div>
                                <h2 className="text-base font-bold bg-gradient-to-r from-cyan-400 to-emerald-400 bg-clip-text text-transparent">Telemetry Core</h2>
                                <p className="text-[9px] text-muted-foreground uppercase font-bold tracking-wider">DevOps Streaming</p>
                            </div>
                        </div>

                        {/* Navigation */}
                        <nav className="space-y-1" aria-label="Main navigation">
                            {navItems.map((item) => {
                                const Icon = item.icon;
                                const isActive = pathname === item.href;
                                return (
                                    <Link
                                        key={item.href}
                                        href={item.href}
                                        onClick={onClose}
                                        className={cn(
                                            "flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-sm font-medium transition-all duration-200 group cursor-pointer border",
                                            isActive 
                                                ? "bg-cyan-500/10 text-cyan-400 border-cyan-500/20 shadow-md shadow-cyan-500/5" 
                                                : "text-muted-foreground hover:text-foreground hover:bg-white/5 border-transparent"
                                        )}
                                    >
                                        <Icon 
                                            size={17} 
                                            className={cn(
                                                "transition-colors", 
                                                isActive ? "text-cyan-400" : "text-muted-foreground group-hover:text-foreground"
                                            )} 
                                            aria-hidden="true" 
                                        />
                                        <span>{item.title}</span>
                                    </Link>
                                );
                            })}
                        </nav>
                    </div>

                    {/* Bottom: Settings & User Profile Section */}
                    <div className="space-y-4">
                        {/* Settings Link */}
                        <div className="space-y-1">
                            {bottomNavItems.map((item) => {
                                const Icon = item.icon;
                                const isActive = pathname === item.href;
                                return (
                                    <Link
                                        key={item.href}
                                        href={item.href}
                                        onClick={onClose}
                                        className={cn(
                                            "flex items-center gap-3 px-3.5 py-2.5 rounded-lg text-sm font-medium transition-all duration-200 group cursor-pointer border",
                                            isActive 
                                                ? "bg-cyan-500/10 text-cyan-400 border-cyan-500/20 shadow-md shadow-cyan-500/5" 
                                                : "text-muted-foreground hover:text-foreground hover:bg-white/5 border-transparent"
                                        )}
                                    >
                                        <Icon 
                                            size={17} 
                                            className={cn(
                                                "transition-colors", 
                                                isActive ? "text-cyan-400" : "text-muted-foreground group-hover:text-foreground"
                                            )} 
                                            aria-hidden="true" 
                                        />
                                        <span>{item.title}</span>
                                    </Link>
                                );
                            })}
                        </div>

                        {/* Premium User Profile Section */}
                        {user && (
                            <div className="flex items-center justify-between p-3 rounded-xl border border-border-color bg-glass-card/45 shadow-sm">
                                <div className="flex items-center gap-2.5 min-w-0">
                                    {/* Avatar with live status pulse dot */}
                                    <div className="relative flex-shrink-0 w-8 h-8 rounded-lg bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 flex items-center justify-center font-bold text-xs uppercase">
                                        {user?.username?.substring(0, 2) ?? 'OP'}
                                        <span className="absolute bottom-0 right-0 w-2 h-2 rounded-full bg-emerald-500 border border-zinc-950 animate-pulse" />
                                    </div>
                                    <div className="min-w-0">
                                        <p className="text-xs font-bold text-foreground truncate">{user?.username ?? 'Operator'}</p>
                                        <p className="text-[9px] text-muted-foreground uppercase font-bold tracking-wider">{user?.role ?? 'User'}</p>
                                    </div>
                                </div>
                                <button
                                    onClick={logout}
                                    className="p-1.5 rounded-lg text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 transition-colors cursor-pointer border border-transparent hover:border-rose-500/20"
                                    aria-label="Log out"
                                >
                                    <LogOut size={14} />
                                </button>
                            </div>
                        )}
                    </div>
                </div>
            </aside>
        </>
    );
}
