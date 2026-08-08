import axios from 'axios';

const api = axios.create({
    baseURL: '/api',
    withCredentials: true,
});

// Setup global error handling interceptor
api.interceptors.response.use(
    (response) => response,
    (error) => {
        if (error.response && error.response.status === 401) {
            // Dispatch dynamic window event to signal logout to the auth provider
            if (typeof window !== 'undefined') {
                window.dispatchEvent(new CustomEvent('auth:unauthorized'));
            }
        }
        return Promise.reject(error);
    }
);

export interface ClientProfile {
    id: string;
    username: string;
    email?: string;
    role: string;
    clientId?: string;
    createdAt?: string;
}

export interface ClientCompany {
    id: string;
    name: string;
    description?: string;
    createdAt: string;
}

export interface ApiKey {
    id: string;
    keyId: string;
    name: string;
    description?: string;
    environment?: string;
    key?: string; // only returned once upon creation or rotation
    keyValue?: string;
    prefix?: string;
    createdAt: string;
    isActive: boolean;
    permissions?: {
        canIngest: boolean;
        canReadAnalytics: boolean;
        allowedServices: string[];
    };
    security?: {
        allowedIPs: string[];
        allowedOrigins: string[];
        rotationWarningDays: number;
    };
    createdBy?: {
        _id: string;
        username: string;
        email?: string;
    } | string;
    expiresAt?: string;
}

export interface ApiKeyCreateInput {
    name: string;
    description?: string;
    environment?: 'production' | 'staging' | 'development' | 'testing';
    expiresAt?: number; // duration in minutes
    permissions?: {
        canIngest?: boolean;
        canReadAnalytics?: boolean;
        allowedServices?: string[];
    };
    security?: {
        allowedIPs?: string[];
        allowedOrigins?: string[];
        rotationWarningDays?: number;
    };
}

export interface MetricStats {
    totalHits: number;
    avgLatency: number;
    errorRate: number;
    errorHits: number;
    successHits: number;
    uniqueServices: number;
    uniqueEndpoints: number;
}

export interface TopEndpoint {
    endpoint: string;
    method: string;
    serviceName: string;
    totalHits: number;
    avgLatency: number;
    errorRate: number;
}

export interface RecentActivity {
    serviceName: string;
    endpoint: string;
    method: string;
    totalHits: number;
    errorHits: number;
    avgLatency: string | number;
    minLatency: string | number;
    maxLatency: string | number;
    timeBucket: string;
}

export interface DashboardPayload {
    success: boolean;
    data: {
        stats: MetricStats;
        topEndpoints: TopEndpoint[];
        recentActivity: RecentActivity[];
    };
}

export const authApi = {
    login: async (credentials: any) => {
        const response = await api.post('/auth/login', credentials);
        return response.data;
    },
    register: async (userData: any) => {
        const response = await api.post('/auth/register', userData);
        return response.data;
    },
    getProfile: async (options?: { signal?: AbortSignal }): Promise<ClientProfile> => {
        const response = await api.get('/auth/profile', { signal: options?.signal });
        return response.data?.data ?? response.data;
    },
    logout: async () => {
        const response = await api.post('/auth/logout');
        return response.data;
    },
    updateProfile: async (profileData: any) => {
        const response = await api.put('/auth/profile', profileData);
        return response.data?.data ?? response.data;
    },
    deactivateUser: async (userId: string) => {
        const response = await api.patch(`/auth/users/${userId}/deactivate`);
        return response.data?.data ?? response.data;
    }
};

export const analyticsApi = {
    getDashboard: async (): Promise<DashboardPayload> => {
        const response = await api.get('/analytics/dashboard');
        const payload = response.data || {};

        payload.data = payload.data || {};

        payload.data.stats = payload.data.stats ?? {
            totalHits: 0,
            avgLatency: 0,
            errorRate: 0,
            errorHits: 0,
            successHits: 0,
            uniqueServices: 0,
            uniqueEndpoints: 0,
        };

        payload.data.topEndpoints = payload.data.topEndpoints ?? [];
        payload.data.recentActivity = payload.data.recentActivity ?? [];

        return payload;
    },
    getStats: async (params?: any) => {
        const response = await api.get('/analytics/stats', { params });
        return response.data;
    },
    getTopEndpoints: async (params?: any) => {
        const response = await api.get('/analytics/top-endpoints', { params });
        return response.data;
    },
    getTimeSeries: async (params?: any) => {
        const response = await api.get('/analytics/time-series', { params });
        return response.data;
    },
    getApisMetrics: async (params?: { page?: number; limit?: number }): Promise<{
        items: any[];
        pagination: { page: number; limit: number; totalCount: number; totalPages: number };
    }> => {
        const response = await api.get('/analytics/apis', { params });
        return response.data?.data ?? response.data;
    },
};

export const clientApi = {
    createClient: async (clientData: { name: string; email?: string; description?: string; website?: string }): Promise<ClientCompany> => {
        const response = await api.post('/admin/clients/onboard', clientData);
        return response.data?.data ?? response.data;
    },
    createApiKey: async (clientId: string, keyData: ApiKeyCreateInput): Promise<ApiKey> => {
        const response = await api.post(`/admin/clients/${clientId}/api/keys`, keyData);
        return response.data?.data ?? response.data;
    },
    getClientApiKeys: async (clientId: string): Promise<ApiKey[]> => {
        const response = await api.get(`/admin/clients/${clientId}/api/keys`);
        return response.data?.data ?? response.data;
    },
    createClientUser: async (clientId: string, userData: any): Promise<any> => {
        const response = await api.post(`/admin/clients/${clientId}/users`, userData);
        return response.data?.data ?? response.data;
    },
    updateApiKey: async (clientId: string, keyId: string, keyData: ApiKeyCreateInput): Promise<ApiKey> => {
        const response = await api.put(`/admin/clients/${clientId}/api/keys/${keyId}`, keyData);
        return response.data?.data ?? response.data;
    },
    deleteApiKey: async (clientId: string, keyId: string): Promise<any> => {
        const response = await api.delete(`/admin/clients/${clientId}/api/keys/${keyId}`);
        return response.data?.data ?? response.data;
    },
    deactivateApiKey: async (clientId: string, keyId: string): Promise<ApiKey> => {
        const response = await api.patch(`/admin/clients/${clientId}/api/keys/${keyId}/deactivate`);
        return response.data?.data ?? response.data;
    },
    activateApiKey: async (clientId: string, keyId: string): Promise<ApiKey> => {
        const response = await api.patch(`/admin/clients/${clientId}/api/keys/${keyId}/activate`);
        return response.data?.data ?? response.data;
    },
    rotateApiKey: async (clientId: string, keyId: string): Promise<ApiKey> => {
        const response = await api.post(`/admin/clients/${clientId}/api/keys/${keyId}/rotate`);
        return response.data?.data ?? response.data;
    }
};

export default api;
