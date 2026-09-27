"use client";

import { useQuery } from '@tanstack/react-query';
import { analyticsApi, DashboardPayload } from '@/lib/api';

export const QUERY_KEYS = {
    DASHBOARD: ['dashboard'],
    STATS: ['stats'],
    TOP_ENDPOINTS: ['topEndpoints'],
    TIME_SERIES: ['timeSeries'],
    APIS_METRICS: (page: number, limit: number, clientId?: string) => ['apisMetrics', page, limit, clientId],
    ENDPOINT_METRICS: (serviceName: string, endpoint: string, method: string, clientId?: string) =>
        ['endpointMetrics', serviceName, endpoint, method, clientId],
};

export const REFETCH_INTERVAL = 15_000; // Refresh every 15 seconds for real-time monitoring feel

export function useDashboardQuery(params?: { clientId?: string; startTime?: string; endTime?: string }, options = {}) {
    return useQuery<DashboardPayload>({
        queryKey: [...QUERY_KEYS.DASHBOARD, params?.clientId, params?.startTime, params?.endTime],
        queryFn: () => analyticsApi.getDashboard(params),
        refetchInterval: REFETCH_INTERVAL,
        ...options,
    });
}

export function useApisMetricsQuery(page: number, limit: number, clientId?: string, options = {}) {
    return useQuery({
        queryKey: QUERY_KEYS.APIS_METRICS(page, limit, clientId),
        queryFn: () => analyticsApi.getApisMetrics({ page, limit, clientId }),
        refetchInterval: REFETCH_INTERVAL,
        ...options,
    });
}

export function useEndpointMetricsQuery(
    params: {
        serviceName: string;
        endpoint: string;
        method: string;
        startTime?: string;
        endTime?: string;
        clientId?: string;
    },
    options = {}
) {
    return useQuery({
        queryKey: [
            ...QUERY_KEYS.ENDPOINT_METRICS(params.serviceName, params.endpoint, params.method, params.clientId),
            params.startTime,
            params.endTime,
        ],
        queryFn: () => analyticsApi.getEndpointMetrics(params),
        enabled: Boolean(params.serviceName && params.endpoint && params.method),
        refetchInterval: REFETCH_INTERVAL,
        ...options,
    });
}

