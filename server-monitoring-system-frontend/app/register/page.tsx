"use client";

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { useAuth } from '@/contexts/auth-context';
import { authApi } from '@/lib/api';
import { Activity, Lock, User, Mail, Loader2, Eye, EyeOff } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useToast } from '@/contexts/toast-context';

export default function RegisterPage() {
    const { isAuthenticated, loading } = useAuth();
    const router = useRouter();
    const toast = useToast();

    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');
    const [error, setError] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    
    // Toggle show password states
    const [showPassword, setShowPassword] = useState(false);
    const [showConfirmPassword, setShowConfirmPassword] = useState(false);

    useEffect(() => {
        if (isAuthenticated) {
            router.push('/dashboard');
        }
    }, [isAuthenticated, router]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');

        if (password !== confirmPassword) {
            setError('Passwords do not match');
            return;
        }

        if (password.length < 8) {
            setError('Password must be at least 8 characters long');
            return;
        }

        setIsSubmitting(true);
        try {
            // First attempt onboard-super-admin (for initial setup), fall back to standard register
            let res;
            try {
                res = await authApi.onboardSuperAdmin({ username, email, password });
            } catch (superAdminErr: any) {
                // If super admin exists or endpoint is not suitable, try register endpoint
                if (superAdminErr.response?.status === 400 && superAdminErr.response?.data?.message?.includes('already exists')) {
                    res = await authApi.register({ username, email, password });
                } else {
                    throw superAdminErr;
                }
            }

            if (res.success) {
                toast('Account registered successfully! Please sign in.', 'success');
                router.push('/login');
            } else {
                setError(res.message || 'Registration failed');
            }
        } catch (err: any) {
            setError(err.response?.data?.message || err.message || 'Registration failed');
        } finally {
            setIsSubmitting(false);
        }
    };

    if (loading || isAuthenticated) {
        return (
            <div className="min-h-screen bg-background flex flex-col items-center justify-center gap-3">
                <Loader2 className="animate-spin text-cyan-500 w-8 h-8" />
                <p className="text-xs font-mono text-muted-foreground animate-pulse">Establishing operator session...</p>
            </div>
        );
    }

    return (
        <div className="min-h-screen flex items-center justify-center p-4 relative overflow-hidden bg-background">
            {/* Animated background grid */}
            <div className="absolute inset-0 grid-bg opacity-30 pointer-events-none" />

            <div className="w-full max-w-md glass-panel p-8 rounded-2xl border border-border-color shadow-2xl relative z-10 space-y-6">
                
                {/* Brand Logo & Title */}
                <div className="text-center space-y-2">
                    <div className="inline-flex items-center justify-center w-12 h-12 rounded-2xl bg-cyan-500/10 text-cyan-400 border border-cyan-500/20 shadow-md shadow-cyan-500/5 animate-pulse">
                        <Activity size={24} />
                    </div>
                    <h2 className="text-xl font-extrabold tracking-tight text-foreground">
                        Register Operator User
                    </h2>
                    <p className="text-xs text-muted-foreground">
                        Create administrative credentials to deploy monitoring scopes.
                    </p>
                </div>

                {error && (
                    <div className="bg-rose-500/10 border border-rose-500/20 text-rose-400 text-xs px-4 py-3 rounded-lg flex items-center gap-2">
                        <span className="w-1.5 h-1.5 rounded-full bg-rose-400 shrink-0" />
                        <span className="font-medium">{error}</span>
                    </div>
                )}

                {/* Form */}
                <form onSubmit={handleSubmit} className="space-y-4">
                    <div className="space-y-1.5">
                        <label htmlFor="username" className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                            Username
                        </label>
                        <div className="relative">
                            <span className="absolute inset-y-0 left-0 pl-3 flex items-center text-muted-foreground">
                                <User size={16} />
                            </span>
                            <Input
                                type="text"
                                id="username"
                                value={username}
                                onChange={(e) => setUsername(e.target.value)}
                                required
                                disabled={isSubmitting}
                                className="pl-10"
                                placeholder="Choose a username"
                            />
                        </div>
                    </div>

                    <div className="space-y-1.5">
                        <label htmlFor="email" className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                            Email Address
                        </label>
                        <div className="relative">
                            <span className="absolute inset-y-0 left-0 pl-3 flex items-center text-muted-foreground">
                                <Mail size={16} />
                            </span>
                            <Input
                                type="email"
                                id="email"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                required
                                disabled={isSubmitting}
                                className="pl-10"
                                placeholder="operator@company.com"
                            />
                        </div>
                    </div>

                    <div className="space-y-1.5">
                        <label htmlFor="password" className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                            Password
                        </label>
                        <div className="relative">
                            <span className="absolute inset-y-0 left-0 pl-3 flex items-center text-muted-foreground">
                                <Lock size={16} />
                            </span>
                            <Input
                                type={showPassword ? "text" : "password"}
                                id="password"
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                required
                                disabled={isSubmitting}
                                className="pl-10 pr-10"
                                placeholder="Choose a password (min. 8 chars)"
                            />
                            <button
                                type="button"
                                onClick={() => setShowPassword(!showPassword)}
                                className="absolute inset-y-0 right-0 pr-3 flex items-center text-muted-foreground hover:text-foreground cursor-pointer"
                            >
                                {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                            </button>
                        </div>
                    </div>

                    <div className="space-y-1.5">
                        <label htmlFor="confirmPassword" className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                            Confirm Password
                        </label>
                        <div className="relative">
                            <span className="absolute inset-y-0 left-0 pl-3 flex items-center text-muted-foreground">
                                <Lock size={16} />
                            </span>
                            <Input
                                type={showConfirmPassword ? "text" : "password"}
                                id="confirmPassword"
                                value={confirmPassword}
                                onChange={(e) => setConfirmPassword(e.target.value)}
                                required
                                disabled={isSubmitting}
                                className="pl-10 pr-10"
                                placeholder="Repeat your password"
                            />
                            <button
                                type="button"
                                onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                                className="absolute inset-y-0 right-0 pr-3 flex items-center text-muted-foreground hover:text-foreground cursor-pointer"
                            >
                                {showConfirmPassword ? <EyeOff size={16} /> : <Eye size={16} />}
                            </button>
                        </div>
                    </div>

                    <Button
                        type="submit"
                        className="w-full mt-2 cursor-pointer"
                        isLoading={isSubmitting}
                    >
                        Sign Up
                    </Button>
                </form>

                {/* Footnotes */}
                <div className="text-center text-xs text-muted-foreground pt-2">
                    Already have an account?{' '}
                    <Link href="/login" className="text-cyan-400 hover:text-cyan-300 font-bold underline underline-offset-4">
                        Sign In
                    </Link>
                </div>
            </div>
        </div>
    );
}
