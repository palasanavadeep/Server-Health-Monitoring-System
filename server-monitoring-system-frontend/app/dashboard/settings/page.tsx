"use client";

import { useState, useEffect } from 'react';
import { useTheme } from '@/contexts/theme-context';
import { useAuth } from '@/contexts/auth-context';
import { authApi } from '@/lib/api';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useToast } from '@/contexts/toast-context';
import { 
    User, 
    Palette, 
    Cpu, 
    Save, 
    Eye, 
    EyeOff, 
    Building, 
    Activity, 
    Shield, 
    Mail, 
    Lock,
    Settings,
    UserMinus,
    CheckCircle2,
    Sun,
    Moon
} from 'lucide-react';
import { cn } from '@/lib/utils';

export default function SettingsPage() {
    const toast = useToast();
    const { currentTheme, themes, switchTheme } = useTheme();
    const { user, refreshProfile, logout } = useAuth();

    // Profile updates form fields
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [updatingProfile, setUpdatingProfile] = useState(false);
    const [showPassword, setShowPassword] = useState(false);

    // Sync profile username and email with inputs on load
    useEffect(() => {
        if (user) {
            setUsername(user.username || '');
            setEmail(user.email || '');
        }
    }, [user]);

    // Update Profile Information
    const handleUpdateProfile = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!username.trim()) return;

        setUpdatingProfile(true);
        try {
            await authApi.updateProfile({
                username,
                email: email.trim() ? email : undefined,
                password: password.trim() ? password : undefined
            });
            toast('Profile configurations saved successfully.', 'success');
            await refreshProfile(); // Refresh session state to show updated name in Sidebar
            setPassword('');
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to update profile configurations', 'error');
        } finally {
            setUpdatingProfile(false);
        }
    };

    // Deactivate Account
    const handleDeactivateAccount = async () => {
        if (!user?.id) return;
        
        if (!confirm("Are you sure you want to suspend this credentials session? Suspending your operator access will terminate this session immediately.")) {
            return;
        }

        try {
            await authApi.deactivateUser(user.id);
            toast('Session suspended. Logging out...', 'info');
            await logout();
        } catch (err: any) {
            toast(err.response?.data?.message || err.message || 'Failed to suspend operator session', 'error');
        }
    };

    return (
        <div className="max-w-4xl mx-auto pb-16 space-y-10 animate-in fade-in duration-300">
            {/* Simple, clean header */}
            <div className="border-b border-border-color/30 pb-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div className="space-y-1">
                    <h1 className="text-2xl font-bold tracking-tight text-foreground flex items-center gap-2">
                        Account & System Configurations
                    </h1>
                    <p className="text-xs text-muted-foreground">
                        Configure workspace settings, customize appearance themes, and manage credentials.
                    </p>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                    <Badge variant="outline" className="font-mono text-[9px] uppercase border-border-color bg-zinc-950/20 text-muted-foreground">
                        Operator: {user?.username}
                    </Badge>
                </div>
            </div>

            {/* Layout: Section blocks with side titles */}
            <div className="space-y-12">

                {/* Section 1: User Identity & Profile */}
                <div className="grid grid-cols-1 md:grid-cols-12 gap-6 pt-4">
                    <div className="md:col-span-4 space-y-1">
                        <h3 className="text-sm font-semibold text-foreground flex items-center gap-1.5">
                            <User size={14} className="text-cyan-400" />
                            Operator Profile
                        </h3>
                        <p className="text-[11px] text-muted-foreground leading-normal">
                            Manage your login credentials, email settings, and administrative information.
                        </p>
                    </div>
                    <div className="md:col-span-8">
                        <Card className="border border-border-color shadow-sm">
                            <CardContent className="pt-6 space-y-4">
                                <form onSubmit={handleUpdateProfile} className="space-y-4">
                                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                                        <div className="space-y-1">
                                            <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Username</label>
                                            <Input
                                                value={username}
                                                onChange={(e) => setUsername(e.target.value)}
                                                placeholder="Username"
                                                required
                                                className="text-xs"
                                            />
                                        </div>
                                        <div className="space-y-1">
                                            <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Email Address</label>
                                            <Input
                                                type="email"
                                                value={email}
                                                onChange={(e) => setEmail(e.target.value)}
                                                placeholder="operator@company.com"
                                                className="text-xs"
                                            />
                                        </div>
                                    </div>
                                    <div className="space-y-1">
                                        <label className="text-[10px] font-bold text-muted-foreground uppercase tracking-wider block">Update Password</label>
                                        <div className="relative">
                                            <Input
                                                type={showPassword ? "text" : "password"}
                                                value={password}
                                                onChange={(e) => setPassword(e.target.value)}
                                                placeholder="Leave blank to keep current password"
                                                className="pr-10 text-xs"
                                            />
                                            <button
                                                type="button"
                                                onClick={() => setShowPassword(!showPassword)}
                                                className="absolute inset-y-0 right-0 pr-3 flex items-center text-muted-foreground hover:text-foreground cursor-pointer"
                                            >
                                                {showPassword ? <EyeOff size={14} /> : <Eye size={14} />}
                                            </button>
                                        </div>
                                    </div>
                                    <div className="flex justify-end pt-1">
                                        <Button 
                                            type="submit" 
                                            size="sm"
                                            className="text-xs h-8 gap-1.5 cursor-pointer" 
                                            isLoading={updatingProfile}
                                        >
                                            <Save size={13} />
                                            Save Changes
                                        </Button>
                                    </div>
                                </form>
                            </CardContent>
                        </Card>
                    </div>
                </div>

                <hr className="border-border-color/20" />

                {/* Section 2: Appearance & Styling */}
                <div className="grid grid-cols-1 md:grid-cols-12 gap-6">
                    <div className="md:col-span-4 space-y-1">
                        <h3 className="text-sm font-semibold text-foreground flex items-center gap-1.5">
                            <Palette size={14} className="text-cyan-400" />
                            Visual Interface
                        </h3>
                        <p className="text-[11px] text-muted-foreground leading-normal">
                            Choose between visual themes optimized for operators review sessions.
                        </p>
                    </div>
                    <div className="md:col-span-8">
                        <Card className="border border-border-color shadow-sm">
                            <CardContent className="pt-6 grid grid-cols-1 sm:grid-cols-2 gap-4">
                                {Object.entries(themes).map(([themeKey, themeDetails]) => {
                                    const isActive = currentTheme === themeKey;
                                    const Icon = themeKey === 'light' ? Sun : Moon;

                                    return (
                                        <button
                                            key={themeKey}
                                            onClick={() => switchTheme(themeKey as 'cyber' | 'light')}
                                            className={cn(
                                                "flex items-center justify-between p-4 rounded-xl border text-left transition-colors cursor-pointer select-none",
                                                isActive 
                                                    ? "border-cyan-500/30 bg-cyan-500/5 text-cyan-400" 
                                                    : "border-border-color bg-glass-card hover:bg-glass-card-hover"
                                            )}
                                        >
                                            <div className="flex items-center gap-3">
                                                <Icon size={16} className={isActive ? "text-cyan-400" : "text-muted-foreground"} />
                                                <div>
                                                    <span className="text-xs font-semibold text-foreground block">{themeDetails.name}</span>
                                                    <span className="text-[10px] text-muted-foreground font-medium block mt-0.5">{themeDetails.description}</span>
                                                </div>
                                            </div>
                                            {isActive && <CheckCircle2 size={14} className="text-cyan-400 shrink-0 ml-2" />}
                                        </button>
                                    );
                                })}
                            </CardContent>
                        </Card>
                    </div>
                </div>

                <hr className="border-border-color/20" />

                {/* Section 3: Telemetry Synchronization & Metrics */}
                <div className="grid grid-cols-1 md:grid-cols-12 gap-6">
                    <div className="md:col-span-4 space-y-1">
                        <h3 className="text-sm font-semibold text-foreground flex items-center gap-1.5">
                            <Cpu size={14} className="text-cyan-400" />
                            System Telemetry
                        </h3>
                        <p className="text-[11px] text-muted-foreground leading-normal">
                            Configure synchronization intervals and caching rules for metrics polling.
                        </p>
                    </div>
                    <div className="md:col-span-8">
                        <Card className="border border-border-color shadow-sm">
                            <CardContent className="pt-6 grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs font-medium text-muted-foreground">
                                <div className="p-3.5 rounded-xl border border-border-color bg-glass-card/20 space-y-1">
                                    <span className="text-[9px] uppercase font-bold tracking-wider block">Auto refresh cycle</span>
                                    <div className="text-foreground font-semibold mt-0.5">Every 15 Seconds</div>
                                </div>
                                <div className="p-3.5 rounded-xl border border-border-color bg-glass-card/20 space-y-1">
                                    <span className="text-[9px] uppercase font-bold tracking-wider block">Local Cache Lifetime</span>
                                    <div className="text-foreground font-semibold mt-0.5">5 Minutes Buffering</div>
                                </div>
                            </CardContent>
                        </Card>
                    </div>
                </div>

                <hr className="border-border-color/20" />

                {/* Section 4: Security & Suspend actions (No Danger Zone terminology) */}
                <div className="grid grid-cols-1 md:grid-cols-12 gap-6">
                    <div className="md:col-span-4 space-y-1">
                        <h3 className="text-sm font-semibold text-foreground flex items-center gap-1.5">
                            <Shield size={14} className="text-rose-400" />
                            Account Suspend Actions
                        </h3>
                        <p className="text-[11px] text-muted-foreground leading-normal">
                            Administrative actions to terminate credentials access mapping.
                        </p>
                    </div>
                    <div className="md:col-span-8">
                        <Card className="border border-rose-500/10 bg-rose-500/5 shadow-sm">
                            <CardContent className="pt-6 flex flex-col sm:flex-row sm:items-center justify-between gap-4">
                                <div className="space-y-1.5">
                                    <p className="text-xs font-bold text-foreground">Deactivate Operator Session</p>
                                    <p className="text-[11px] text-muted-foreground leading-normal">
                                        Terminates your current operator session and deactivates access. Reactivation requires administrator approval.
                                    </p>
                                </div>
                                <Button 
                                    type="button" 
                                    onClick={handleDeactivateAccount}
                                    variant="outline"
                                    className="border-rose-500/20 text-rose-400 hover:bg-rose-500/10 shrink-0 text-xs h-8 cursor-pointer"
                                >
                                    <UserMinus size={13} className="mr-1.5" />
                                    Deactivate Session
                                </Button>
                            </CardContent>
                        </Card>
                    </div>
                </div>

            </div>
        </div>
    );
}
