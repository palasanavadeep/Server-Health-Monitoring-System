"use client";

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { cn } from '@/lib/utils';
import { useAuth } from '@/contexts/auth-context';
import {
    Activity,
    Braces,
    KeyRound,
    Users,
    Settings,
    Server,
    FileText,
    Bell,
    AlertOctagon,
    Boxes,
    Building,
    LogOut,
    Radio,
    Sliders,
} from 'lucide-react';

interface NavItem {
    title: string;
    href: string;
    icon: any;
    disabled?: boolean;
    badge?: string;
    hideForRoles?: string[];
}

interface NavSection {
    section: string;
    items: NavItem[];
}

const clientNavSections: NavSection[] = [
    {
        section: "MONITOR",
        items: [
            { title: "Overview", href: "/dashboard", icon: Activity },
            { title: "API Routes", href: "/dashboard/apis", icon: Braces },
            { title: "Services", href: "/dashboard/services", icon: Server, badge: "Upcoming", disabled: true },
            { title: "Logs", href: "/dashboard/logs", icon: FileText, badge: "Upcoming", disabled: true },
        ]
    },
    {
        section: "OPERATIONS",
        items: [
            { title: "Alerts", href: "/dashboard/alerts", icon: Bell, badge: "Upcoming", disabled: true },
            { title: "Incidents", href: "/dashboard/incidents", icon: AlertOctagon, badge: "Upcoming", disabled: true },
        ]
    },
    {
        section: "ACCESS",
        items: [
            { title: "API Keys", href: "/dashboard/clients", icon: KeyRound },
            { title: "Operators", href: "/dashboard/operators", icon: Users, hideForRoles: ['client_viewer'] },
        ]
    },
    {
        section: "CONFIGURE",
        items: [
            { title: "Client Config", href: "/dashboard/client-config", icon: Sliders },
            { title: "Integrations", href: "/dashboard/integrations", icon: Boxes, badge: "Upcoming", disabled: true },
            { title: "Settings", href: "/dashboard/settings", icon: Settings },
        ]
    }
];

const superAdminNavSections: NavSection[] = [
    {
        section: "TENANTS",
        items: [
            { title: "Tenants", href: "/dashboard/tenants", icon: Building },
            { title: "Tenant Users", href: "/dashboard/tenants/users", icon: Users },
        ]
    },
    {
        section: "CONFIGURE",
        items: [
            { title: "Client Config", href: "/dashboard/client-config", icon: Sliders },
            { title: "Settings", href: "/dashboard/settings", icon: Settings },
        ]
    }
];

interface SidebarProps {
    isOpen: boolean;
    onClose: () => void;
}

export function Sidebar({ isOpen, onClose }: SidebarProps) {
    const pathname = usePathname();
    const { user, logout } = useAuth();
    const isSuperAdmin = user?.role === 'super_admin';
    const sections = isSuperAdmin ? superAdminNavSections : clientNavSections;

    return (
        <>
            {isOpen && (
                <div
                    className="fixed inset-0 z-40 bg-black/60 backdrop-blur-xs lg:hidden cursor-pointer"
                    onClick={onClose}
                    aria-hidden="true"
                />
            )}
            <aside
                className={cn(
                    "fixed top-0 bottom-0 left-0 z-40 w-60 border-r border-[#242932] bg-[#0E1014] flex flex-col justify-between transition-transform duration-200 lg:static lg:translate-x-0 select-none",
                    isOpen ? "translate-x-0" : "-translate-x-full"
                )}
                aria-label="Sidebar"
            >
                {/* Top Section */}
                <div className="flex flex-col flex-1 overflow-y-auto">
                    {/* Brand Header (56px) */}
                    <div className="h-14 flex items-center px-4 border-b border-[#242932] shrink-0">
                        <div className="flex items-center gap-2.5">
                            <div className="w-6 h-6 rounded bg-[#4CB8D6]/10 border border-[#4CB8D6]/20 flex items-center justify-center text-[#4CB8D6]">
                                <Radio size={13} />
                            </div>
                            <span className="font-semibold text-sm tracking-tight text-zinc-100">
                                Telemetry Core
                            </span>
                        </div>
                    </div>

                    {/* Navigation Sections */}
                    <div className="p-3 space-y-5 flex-1">
                        {sections.map((sectionGroup) => {
                            const visibleItems = sectionGroup.items.filter(
                                item => !item.hideForRoles || !item.hideForRoles.includes(user?.role || '')
                            );

                            if (visibleItems.length === 0) return null;

                            return (
                                <div key={sectionGroup.section} className="space-y-1">
                                    <span className="px-2.5 text-[10px] font-semibold tracking-wider text-zinc-400 uppercase block mb-1">
                                        {sectionGroup.section}
                                    </span>

                                    {visibleItems.map((item) => {
                                        const Icon = item.icon;
                                        const isActive = pathname === item.href || (item.href !== '/dashboard' && pathname.startsWith(item.href));

                                        if (item.disabled) {
                                            return (
                                                <div
                                                    key={item.title}
                                                    className="flex items-center justify-between px-2.5 py-1.5 rounded text-xs text-zinc-400 opacity-60 cursor-not-allowed"
                                                >
                                                    <div className="flex items-center gap-2.5">
                                                        <Icon size={14} className="text-zinc-400" />
                                                        <span>{item.title}</span>
                                                    </div>
                                                    {item.badge && (
                                                        <span className="text-[9px] font-mono uppercase bg-zinc-900 text-zinc-400 px-1 py-0.2 rounded border border-[#242932]">
                                                            {item.badge}
                                                        </span>
                                                    )}
                                                </div>
                                            );
                                        }

                                        return (
                                            <Link
                                                key={item.href}
                                                href={item.href}
                                                onClick={onClose}
                                                className={cn(
                                                    "flex items-center justify-between px-2.5 py-1.5 rounded text-xs font-medium transition-colors cursor-pointer",
                                                    isActive
                                                        ? "bg-[#181D24] text-zinc-100 font-semibold border-l-2 border-[#4CB8D6]"
                                                        : "text-zinc-400 hover:text-zinc-200 hover:bg-[#15191F]"
                                                )}
                                            >
                                                <div className="flex items-center gap-2.5">
                                                    <Icon 
                                                        size={14} 
                                                        className={cn(
                                                            "transition-colors",
                                                            isActive ? "text-[#4CB8D6]" : "text-zinc-400 group-hover:text-zinc-300"
                                                        )} 
                                                    />
                                                    <span>{item.title}</span>
                                                </div>
                                            </Link>
                                        );
                                    })}
                                </div>
                            );
                        })}
                    </div>
                </div>

                {/* Bottom User / Operator Profile Card */}
                {user && (
                    <div className="p-3 border-t border-[#242932] bg-[#0B0D10]/50 shrink-0">
                        <div className="flex items-center justify-between gap-2 p-2 rounded hover:bg-[#15191F] transition-colors">
                            <div className="flex items-center gap-2.5 min-w-0">
                                <div className="w-7 h-7 rounded bg-[#181D24] border border-[#242932] flex items-center justify-center font-bold text-xs text-[#4CB8D6] shrink-0 uppercase">
                                    {user?.username?.substring(0, 2) || 'OP'}
                                </div>
                                <div className="min-w-0">
                                    <p className="text-xs font-semibold text-zinc-200 truncate leading-tight">
                                        {user?.username || 'Operator'}
                                    </p>
                                    <p className="text-[10px] text-zinc-400 font-mono truncate leading-none mt-0.5">
                                        {user?.role?.replace('_', ' ') || 'User'}
                                    </p>
                                </div>
                            </div>
                            <button
                                onClick={logout}
                                className="p-1 rounded text-zinc-400 hover:text-[#E45865] hover:bg-[#E45865]/10 transition-colors cursor-pointer shrink-0"
                                title="Log out"
                                aria-label="Log out"
                            >
                                <LogOut size={13} />
                            </button>
                        </div>
                    </div>
                )}
            </aside>
        </>
    );
}
