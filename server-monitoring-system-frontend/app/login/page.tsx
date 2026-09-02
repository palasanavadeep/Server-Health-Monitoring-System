"use client";

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { useAuth } from '@/contexts/auth-context';
import { Lock, Mail, Loader2, Eye, EyeOff, Radio } from 'lucide-react';
import { Button } from '@/components/ui/button';

const EMAIL_REGEX = /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$/;

export default function LoginPage() {
    const { login, isAuthenticated, user, loading } = useAuth();
    const router = useRouter();

    const [accountIdentifier, setAccountIdentifier] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [showPassword, setShowPassword] = useState(false);

    useEffect(() => {
        if (isAuthenticated && user) {
            if (user.role === 'super_admin') {
                router.push('/dashboard/tenants');
            } else {
                router.push('/dashboard');
            }
        }
    }, [isAuthenticated, user, router]);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');

        const trimmedIdentifier = accountIdentifier.trim();
        if (!trimmedIdentifier) {
            setError('Please enter your email or username');
            return;
        }

        const isEmail = EMAIL_REGEX.test(trimmedIdentifier);
        
        let emailToSend = trimmedIdentifier;
        if (!isEmail) {
            if (!trimmedIdentifier.includes('@')) {
                setError('Please enter a valid email address (e.g. operator@company.com)');
                return;
            } else {
                setError('Please provide a valid email address format');
                return;
            }
        }

        setIsSubmitting(true);

        try {
            const res = await login({ email: emailToSend, password });
            const userRole = res?.data?.role;
            if (userRole === 'super_admin') {
                router.push('/dashboard/tenants');
            } else {
                router.push('/dashboard');
            }
        } catch (err: any) {
            setError(err.response?.data?.message || err.message || 'Invalid email or password');
        } finally {
            setIsSubmitting(false);
        }
    };

    if (loading || isAuthenticated) {
        return (
            <div className="min-h-screen bg-[#0B0D10] flex flex-col items-center justify-center gap-2.5 text-zinc-400">
                <Loader2 className="animate-spin text-[#4CB8D6] w-5 h-5" />
                <p className="text-xs">Establishing operator session...</p>
            </div>
        );
    }

    return (
        <div className="min-h-screen flex items-center justify-center p-4 bg-[#0B0D10] select-none">
            <div className="w-full max-w-sm surface-panel p-6 bg-[#111419] border border-[#242932] space-y-5">
                {/* Brand Header */}
                <div className="text-center space-y-1">
                    <div className="inline-flex items-center justify-center w-9 h-9 rounded bg-[#4CB8D6]/10 text-[#4CB8D6] border border-[#4CB8D6]/20 mb-1">
                        <Radio size={16} />
                    </div>
                    <h2 className="text-base font-semibold tracking-tight text-zinc-100">
                        Telemetry Core
                    </h2>
                    <p className="text-xs text-zinc-400">
                        Sign in to access your observability workspace.
                    </p>
                </div>

                {error && (
                    <div className="p-2.5 rounded bg-[#E45865]/10 border border-[#E45865]/20 text-[#E45865] text-xs">
                        {error}
                    </div>
                )}

                <form onSubmit={handleSubmit} className="space-y-3.5">
                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block font-medium">Email Address</label>
                        <div className="relative">
                            <input
                                type="text"
                                value={accountIdentifier}
                                onChange={(e) => setAccountIdentifier(e.target.value)}
                                placeholder="operator@company.com"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded pl-8 pr-3 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6]"
                            />
                            <Mail size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                        </div>
                    </div>

                    <div className="space-y-1">
                        <div className="flex items-center justify-between">
                            <label className="text-xs text-zinc-300 block font-medium">Password</label>
                        </div>
                        <div className="relative">
                            <input
                                type={showPassword ? "text" : "password"}
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                placeholder="••••••••"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded pl-8 pr-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6]"
                            />
                            <Lock size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                            <button
                                type="button"
                                onClick={() => setShowPassword(!showPassword)}
                                className="absolute right-2.5 top-2 text-zinc-500 hover:text-zinc-300 cursor-pointer"
                            >
                                {showPassword ? <EyeOff size={13} /> : <Eye size={13} />}
                            </button>
                        </div>
                    </div>

                    <Button
                        type="submit"
                        isLoading={isSubmitting}
                        className="w-full text-xs h-8 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer mt-2"
                    >
                        Sign In
                    </Button>
                </form>

                <div className="pt-3 border-t border-[#242932] text-center text-xs text-zinc-400">
                    Need an account?{' '}
                    <Link href="/register" className="text-[#4CB8D6] hover:underline font-medium">
                        Register workspace
                    </Link>
                </div>
            </div>
        </div>
    );
}
