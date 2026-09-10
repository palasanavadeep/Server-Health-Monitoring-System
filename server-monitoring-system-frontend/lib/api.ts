import axios, { AxiosResponse } from 'axios';

const api = axios.create({
    baseURL: '/api',
    withCredentials: true,
});

// Setup global error handling interceptor
api.interceptors.response.use(
    (response: AxiosResponse) => response,
    (error: unknown) => {
        if (axios.isAxiosError(error) && error.response && error.response.status === 401) {
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
    isActive?: boolean;
}

export interface ClientCompany {
    id: string;
    name: string;
    email?: string;
    description?: string;
    website?: string;
    createdAt: string;
    isActive?: boolean;
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
    canIngest: boolean;
    canRead: boolean;
    allowedIps: string[];
    allowedOrigins: string[];
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
    expiresAt?: number | string; // duration in minutes or ISO timestamp
    expiresInMinutes?: number;
    allowedIps?: string[];
    allowedOrigins?: string[];
    canIngest?: boolean;
    canRead?: boolean;
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

export interface ApiMetricsEntry {
    serviceName: string;
    endpoint: string;
    method: string;
    totalHits: number;
    errorHits: number;
    successHits: number;
    errorRate: number;
    avgLatency: number;
    minLatency: number;
    maxLatency: number;
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

export interface AnalyticsQueryParams {
    clientId?: string;
    startTime?: string;
    endTime?: string;
    limit?: number;
}

export interface CreateClientUserInput {
    username: string;
    email: string;
    password: string;
    role?: string;
}

export interface ApiSuccessResponse<T = unknown> {
    success: boolean;
    message: string;
    data: T;
    statusCode: number;
    timestamp: string;
}

// ── Helpers / Normalizers ───────────────────────────────────────────────────

function normalizeUserProfile(raw: Record<string, unknown> | null | undefined): ClientProfile {
    if (!raw) {
        return {
            id: '',
            username: '',
            role: '',
        };
    }
    return {
        id: String(raw.id ?? raw._id ?? ''),
        username: String(raw.username ?? ''),
        email: raw.email ? String(raw.email) : undefined,
        role: String(raw.role ?? ''),
        clientId: raw.clientId ? String(raw.clientId) : (raw.client_id ? String(raw.client_id) : undefined),
        createdAt: raw.createdAt ? String(raw.createdAt) : (raw.created_at ? String(raw.created_at) : undefined),
        isActive: raw.isActive !== undefined ? Boolean(raw.isActive) : (raw.is_active !== undefined ? Boolean(raw.is_active) : true),
    };
}

function normalizeApiKey(raw: Record<string, unknown> | null | undefined): ApiKey {
    if (!raw) {
        return {
            id: '',
            keyId: '',
            name: '',
            environment: 'production',
            createdAt: new Date().toISOString(),
            isActive: true,
            canIngest: true,
            canRead: false,
            allowedIps: ['0.0.0.0/0'],
            allowedOrigins: ['*'],
        };
    }

    const rawKeyVal = (raw.keyValue ?? raw.key_value ?? raw.key) as string | undefined;
    let prefix = raw.prefix as string | undefined;
    if (!prefix && rawKeyVal && typeof rawKeyVal === 'string') {
        const stripped = rawKeyVal.replace(/^sm_key_/, '');
        prefix = stripped.substring(0, 4);
    }

    const securityObj = raw.security as Record<string, unknown> | undefined;
    const permissionsObj = raw.permissions as Record<string, unknown> | undefined;

    const allowedIps: string[] = (raw.allowedIps ?? raw.allowed_i_ps ?? raw.allowed_ips ?? (securityObj?.allowedIPs ?? ['0.0.0.0/0'])) as string[];
    const allowedOrigins: string[] = (raw.allowedOrigins ?? raw.allowed_origins ?? (securityObj?.allowedOrigins ?? ['*'])) as string[];
    const canIngest: boolean = (raw.canIngest ?? raw.can_ingest ?? (permissionsObj?.canIngest !== false)) as boolean;
    const canRead: boolean = (raw.canRead ?? raw.can_read ?? (permissionsObj?.canReadAnalytics === true)) as boolean;

    return {
        id: String(raw.id ?? raw._id ?? raw.keyId ?? raw.key_id ?? ''),
        keyId: String(raw.keyId ?? raw.key_id ?? raw.id ?? raw._id ?? ''),
        name: String(raw.name ?? ''),
        description: raw.description ? String(raw.description) : undefined,
        environment: String(raw.environment ?? 'production'),
        key: raw.key ? String(raw.key) : rawKeyVal,
        keyValue: rawKeyVal,
        prefix,
        createdAt: String(raw.createdAt ?? raw.created_at ?? new Date().toISOString()),
        isActive: raw.isActive !== undefined ? Boolean(raw.isActive) : (raw.is_active !== undefined ? Boolean(raw.is_active) : true),
        canIngest,
        canRead,
        allowedIps,
        allowedOrigins,
        permissions: {
            canIngest,
            canReadAnalytics: canRead,
            allowedServices: Array.isArray(permissionsObj?.allowedServices) ? (permissionsObj?.allowedServices as string[]) : [],
        },
        security: {
            allowedIPs: allowedIps,
            allowedOrigins: allowedOrigins,
            rotationWarningDays: typeof securityObj?.rotationWarningDays === 'number' ? securityObj.rotationWarningDays : 30,
        },
        createdBy: raw.createdBy ? String(raw.createdBy) : (raw.created_by ? String(raw.created_by) : undefined),
        expiresAt: raw.expiresAt ? String(raw.expiresAt) : (raw.expires_at ? String(raw.expires_at) : undefined),
    };
}

function transformApiKeyInput(input: ApiKeyCreateInput) {
    const allowedIps = input.allowedIps ?? input.security?.allowedIPs ?? ['0.0.0.0/0'];
    const allowedOrigins = input.allowedOrigins ?? input.security?.allowedOrigins ?? ['*'];
    const canIngest = input.canIngest ?? input.permissions?.canIngest ?? true;
    const canRead = input.canRead ?? input.permissions?.canReadAnalytics ?? false;
    const rotationWarningDays = input.security?.rotationWarningDays;
    const expiresInMinutes = typeof input.expiresAt === 'number' ? input.expiresAt : input.expiresInMinutes;
    const expiresAt = typeof input.expiresAt === 'string' ? input.expiresAt : undefined;

    return {
        name: input.name,
        description: input.description,
        environment: input.environment,
        allowedIps: Array.isArray(allowedIps) ? allowedIps : [allowedIps],
        allowedOrigins: Array.isArray(allowedOrigins) ? allowedOrigins : [allowedOrigins],
        canIngest,
        canRead,
        expiresInMinutes,
        expiresAt,
        rotationWarningDays,
    };
}

// ── API Services ────────────────────────────────────────────────────────────

export const authApi = {
    login: async (credentials: { email?: string; username?: string; password: string }): Promise<ApiSuccessResponse<ClientProfile>> => {
        const email = credentials.email || credentials.username || '';
        const response = await api.post('/auth/login', {
            email,
            password: credentials.password,
        });
        return response.data;
    },
    register: async (userData: { username: string; email: string; password: string; role?: string }): Promise<ApiSuccessResponse<ClientProfile>> => {
        const response = await api.post('/auth/register', userData);
        return response.data;
    },
    onboardSuperAdmin: async (userData: { username: string; email: string; password: string }): Promise<ApiSuccessResponse<ClientProfile>> => {
        const response = await api.post('/auth/onboard-super-admin', userData);
        return response.data;
    },
    getProfile: async (options?: { signal?: AbortSignal }): Promise<ClientProfile> => {
        const response = await api.get('/auth/profile', { signal: options?.signal });
        const raw = response.data?.data ?? response.data;
        return normalizeUserProfile(raw);
    },
    logout: async (): Promise<ApiSuccessResponse<Record<string, never>>> => {
        const response = await api.get('/auth/logout');
        return response.data;
    },
    updateProfile: async (profileData: { username?: string; email?: string }): Promise<ClientProfile> => {
        const response = await api.put('/auth/profile', profileData);
        const raw = response.data?.data ?? response.data;
        return normalizeUserProfile(raw);
    },
    deactivateUser: async (userId: string): Promise<ClientProfile> => {
        const response = await api.patch(`/auth/users/${userId}/deactivate`);
        const raw = response.data?.data ?? response.data;
        return normalizeUserProfile(raw);
    },
    activateUser: async (userId: string): Promise<ClientProfile> => {
        const response = await api.patch(`/auth/users/${userId}/activate`);
        const raw = response.data?.data ?? response.data;
        return normalizeUserProfile(raw);
    }
};

export const analyticsApi = {
    getDashboard: async (params?: AnalyticsQueryParams): Promise<DashboardPayload> => {
        const response = await api.get('/analytics/dashboard', { params });
        const payload = response.data || {};
        const data = payload.data || {};

        data.stats = data.stats ?? {
            totalHits: 0,
            avgLatency: 0,
            errorRate: 0,
            errorHits: 0,
            successHits: 0,
            uniqueServices: 0,
            uniqueEndpoints: 0,
        };

        data.topEndpoints = data.topEndpoints ?? [];
        data.recentActivity = data.recentActivity ?? data.recentActitivy ?? [];

        return {
            success: payload.success ?? true,
            data,
        };
    },
    getStats: async (params?: AnalyticsQueryParams): Promise<MetricStats> => {
        const response = await api.get('/analytics/stats', { params });
        return response.data?.data ?? response.data;
    },
    getTopEndpoints: async (params?: AnalyticsQueryParams): Promise<TopEndpoint[]> => {
        const response = await api.get('/analytics/top-endpoints', { params });
        return response.data?.data ?? response.data;
    },
    getTimeSeries: async (params?: AnalyticsQueryParams): Promise<RecentActivity[]> => {
        const response = await api.get('/analytics/time-series', { params });
        return response.data?.data ?? response.data;
    },
    getApisMetrics: async (params?: { page?: number; limit?: number; clientId?: string }): Promise<{
        items: ApiMetricsEntry[];
        pagination: { page: number; limit: number; totalCount: number; totalPages: number };
    }> => {
        const response = await api.get('/analytics/apis', { params });
        const resData = response.data || {};
        const rawItems = Array.isArray(resData.data) ? resData.data : (resData.data?.items ?? resData.items ?? []);
        const pagination = resData.pagination ?? resData.data?.pagination ?? {};
        const limit = params?.limit ?? pagination.limit ?? 10;
        const totalCount = pagination.total ?? pagination.totalCount ?? rawItems.length;
        const totalPages = pagination.totalPages ?? pagination.total_pages ?? (limit > 0 ? Math.ceil(totalCount / limit) : 1);

        return {
            items: rawItems,
            pagination: {
                page: pagination.page ?? params?.page ?? 1,
                limit,
                totalCount,
                totalPages,
            },
        };
    },
};

export const clientApi = {
    getAllClients: async (): Promise<ClientCompany[]> => {
        const response = await api.get('/admin/clients');
        const rawList = response.data?.data ?? response.data;
        if (Array.isArray(rawList)) {
            return rawList.map((client: any) => ({
                id: String(client.id ?? client._id ?? ''),
                name: String(client.name ?? ''),
                email: client.email ? String(client.email) : undefined,
                description: client.description ? String(client.description) : undefined,
                website: client.website ? String(client.website) : undefined,
                createdAt: String(client.createdAt ?? client.created_at ?? new Date().toISOString()),
                isActive: client.isActive !== undefined ? Boolean(client.isActive) : (client.is_active !== undefined ? Boolean(client.is_active) : true),
            }));
        }
        return [];
    },
    createClient: async (clientData: { name: string; email: string; description?: string; website?: string }): Promise<ClientCompany> => {
        const response = await api.post('/admin/clients/onboard', clientData);
        const resData = response.data?.data ?? response.data;
        const client = resData?.client ?? resData;
        return {
            id: String(client.id ?? client._id ?? ''),
            name: String(client.name ?? ''),
            email: client.email ? String(client.email) : undefined,
            description: client.description ? String(client.description) : undefined,
            website: client.website ? String(client.website) : undefined,
            createdAt: String(client.createdAt ?? client.created_at ?? new Date().toISOString()),
        };
    },
    createApiKey: async (clientId: string, keyData: ApiKeyCreateInput): Promise<ApiKey> => {
        const payload = transformApiKeyInput(keyData);
        const response = await api.post(`/admin/clients/${clientId}/api/keys`, payload);
        const raw = response.data?.data ?? response.data;
        return normalizeApiKey(raw);
    },
    getClientApiKeys: async (clientId: string): Promise<ApiKey[]> => {
        const response = await api.get(`/admin/clients/${clientId}/api/keys`);
        const rawList = response.data?.data ?? response.data;
        if (Array.isArray(rawList)) {
            return rawList.map(normalizeApiKey);
        }
        return [];
    },
    createClientUser: async (clientId: string, userData: CreateClientUserInput): Promise<ClientProfile> => {
        const response = await api.post(`/admin/clients/${clientId}/users`, userData);
        const raw = response.data?.data ?? response.data;
        return normalizeUserProfile(raw);
    },
    getClientUsers: async (clientId?: string): Promise<ClientProfile[]> => {
        const url = clientId ? `/admin/clients/${clientId}/users` : '/client/users';
        const response = await api.get(url);
        const rawList = response.data?.data ?? response.data;
        if (Array.isArray(rawList)) {
            return rawList.map(normalizeUserProfile);
        }
        return [];
    },
    updateApiKey: async (clientId: string, keyId: string, keyData: ApiKeyCreateInput): Promise<ApiKey> => {
        const payload = transformApiKeyInput(keyData);
        const response = await api.put(`/admin/clients/${clientId}/api/keys/${keyId}`, payload);
        const raw = response.data?.data ?? response.data;
        return normalizeApiKey(raw);
    },
    deleteApiKey: async (clientId: string, keyId: string): Promise<ApiSuccessResponse<Record<string, never>>> => {
        const response = await api.delete(`/admin/clients/${clientId}/api/keys/${keyId}`);
        return response.data?.data ?? response.data;
    },
    deactivateApiKey: async (clientId: string, keyId: string): Promise<ApiKey> => {
        const response = await api.patch(`/admin/clients/${clientId}/api/keys/${keyId}/deactivate`);
        const raw = response.data?.data ?? response.data;
        return normalizeApiKey(raw);
    },
    activateApiKey: async (clientId: string, keyId: string): Promise<ApiKey> => {
        const response = await api.patch(`/admin/clients/${clientId}/api/keys/${keyId}/activate`);
        const raw = response.data?.data ?? response.data;
        return normalizeApiKey(raw);
    },
    rotateApiKey: async (clientId: string, keyId: string, reason?: string): Promise<ApiKey> => {
        const response = await api.post(`/admin/clients/${clientId}/api/keys/${keyId}/rotate`, { reason });
        const raw = response.data?.data ?? response.data;
        return normalizeApiKey(raw);
    },
    getApiKey: async (clientId: string, keyId: string): Promise<ApiKey> => {
        const response = await api.get(`/admin/clients/${clientId}/api/keys/${keyId}`);
        const raw = response.data?.data ?? response.data;
        return normalizeApiKey(raw);
    }
};

export default api;
