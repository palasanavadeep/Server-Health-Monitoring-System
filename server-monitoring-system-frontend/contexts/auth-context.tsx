"use client";

import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import { authApi, ClientProfile } from '@/lib/api';
import { useQueryClient } from '@tanstack/react-query';

interface AuthContextType {
    isAuthenticated: boolean | null;
    user: ClientProfile | null;
    loading: boolean;
    login: (credentials: any) => Promise<any>;
    logout: () => Promise<void>;
    refreshProfile: () => Promise<void>;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function useAuth() {
    const context = useContext(AuthContext);
    if (!context) {
        throw new Error('useAuth must be used within an AuthProvider');
    }
    return context;
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
    const [user, setUser] = useState<ClientProfile | null>(null);
    const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null);
    const [loading, setLoading] = useState(true);
    const queryClient = useQueryClient();

    const fetchProfile = useCallback(async (signal?: AbortSignal) => {
        try {
            const data = await authApi.getProfile({ signal });
            setUser(data);
            setIsAuthenticated(true);
        } catch (err: any) {
            if (err.name !== 'CanceledError' && err.name !== 'AbortError') {
                setUser(null);
                setIsAuthenticated(false);
            }
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        const controller = new AbortController();
        fetchProfile(controller.signal);

        return () => {
            controller.abort();
        };
    }, [fetchProfile]);

    const handleLogin = useCallback(async (credentials: any) => {
        setLoading(true);
        try {
            const res = await authApi.login(credentials);
            if (res.success) {
                await fetchProfile();
                return res;
            } else {
                throw new Error(res.message || 'Login failed');
            }
        } catch (err) {
            setLoading(false);
            throw err;
        }
    }, [fetchProfile]);

    const handleLogout = useCallback(async () => {
        setLoading(true);
        try {
            await authApi.logout();
        } catch (err) {
            // Ignore logout API failures
        } finally {
            setUser(null);
            setIsAuthenticated(false);
            setLoading(false);
            queryClient.clear();
        }
    }, [queryClient]);

    // Listen for global 401 interceptor broadcasts
    useEffect(() => {
        if (isAuthenticated !== true) return;

        const handleUnauthorized = () => {
            setUser(null);
            setIsAuthenticated(false);
            queryClient.clear();
        };

        window.addEventListener('auth:unauthorized', handleUnauthorized);
        return () => {
            window.removeEventListener('auth:unauthorized', handleUnauthorized);
        };
    }, [isAuthenticated, queryClient]);

    const value = {
        isAuthenticated,
        user,
        loading,
        login: handleLogin,
        logout: handleLogout,
        refreshProfile: fetchProfile,
    };

    return (
        <AuthContext.Provider value={value}>
            {children}
        </AuthContext.Provider>
    );
}
