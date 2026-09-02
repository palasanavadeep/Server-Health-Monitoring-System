"use client";

import { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { useAuth } from '@/contexts/auth-context';
import { authApi } from '@/lib/api';
import { Lock, Mail, Loader2, Eye, EyeOff, Radio, User } from 'lucide-react';
import { Button } from '@/components/ui/button';
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
            let res;
            try {
                res = await authApi.onboardSuperAdmin({ username, email, password });
            } catch (superAdminErr: any) {
                if (superAdminErr.response?.status === 400 && superAdminErr.response?.data?.message?.includes('already exists')) {
                    res = await authApi.register({ username, email, password });
                } else {
                    throw superAdminErr;
                }
            }

            if (res.success) {
                toast('Account created successfully! You can now log in.', 'success');
                router.push('/login');
            } else {
                setError(res.message || 'Registration failed');
            }
        } catch (err: any) {
            setError(err.response?.data?.message || err.message || 'Registration failed. Try different credentials.');
        } finally {
            setIsSubmitting(false);
        }
    };

    if (loading || isAuthenticated) {
        return (
            <div className="min-h-screen bg-[#0B0D10] flex flex-col items-center justify-center gap-2.5 text-zinc-400">
                <Loader2 className="animate-spin text-[#4CB8D6] w-5 h-5" />
                <p className="text-xs">Initializing...</p>
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
                        Create Workspace Account
                    </h2>
                    <p className="text-xs text-zinc-400">
                        Provision an operator or administrator profile.
                    </p>
                </div>

                {error && (
                    <div className="p-2.5 rounded bg-[#E45865]/10 border border-[#E45865]/20 text-[#E45865] text-xs">
                        {error}
                    </div>
                )}

                <form onSubmit={handleSubmit} className="space-y-3.5">
                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block font-medium">Username</label>
                        <div className="relative">
                            <input
                                type="text"
                                value={username}
                                onChange={(e) => setUsername(e.target.value)}
                                placeholder="e.g. dev_operator"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded pl-8 pr-3 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6]"
                            />
                            <User size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                        </div>
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block font-medium">Email Address</label>
                        <div className="relative">
                            <input
                                type="email"
                                value={email}
                                onChange={(e) => setEmail(e.target.value)}
                                placeholder="operator@company.com"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded pl-8 pr-3 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6]"
                            />
                            <Mail size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                        </div>
                    </div>

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block font-medium">Password</label>
                        <div className="relative">
                            <input
                                type={showPassword ? "text" : "password"}
                                value={password}
                                onChange={(e) => setPassword(e.target.value)}
                                placeholder="Min. 8 characters"
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

                    <div className="space-y-1">
                        <label className="text-xs text-zinc-300 block font-medium">Confirm Password</label>
                        <div className="relative">
                            <input
                                type={showConfirmPassword ? "text" : "password"}
                                value={confirmPassword}
                                onChange={(e) => setConfirmPassword(e.target.value)}
                                placeholder="Re-enter password"
                                required
                                className="w-full bg-[#0E1014] border border-[#242932] rounded pl-8 pr-8 py-1.5 text-xs text-zinc-200 placeholder:text-zinc-500 focus:outline-none focus:border-[#4CB8D6]"
                            />
                            <Lock size={13} className="absolute left-2.5 top-2.5 text-zinc-500" />
                            <button
                                type="button"
                                onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                                className="absolute right-2.5 top-2 text-zinc-500 hover:text-zinc-300 cursor-pointer"
                            >
                                {showConfirmPassword ? <EyeOff size={13} /> : <Eye size={13} />}
                            </button>
                        </div>
                    </div>

                    <Button
                        type="submit"
                        isLoading={isSubmitting}
                        className="w-full text-xs h-8 bg-[#4CB8D6] hover:bg-[#65C6E0] text-zinc-950 font-semibold cursor-pointer mt-2"
                    >
                        Register
                    </Button>
                </form>

                <div className="pt-3 border-t border-[#242932] text-center text-xs text-zinc-400">
                    Already have an account?{' '}
                    <Link href="/login" className="text-[#4CB8D6] hover:underline font-medium">
                        Sign in
                    </Link>
                </div>
            </div>
        </div>
    );
}
