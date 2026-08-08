"use client";

import { useQuery } from '@tanstack/react-query';
import { analyticsApi } from '@/lib/api';

export const QUERY_KEYS = {
    DASHBOARD: ['dashboard'],
    STATS: ['stats'],
    TOP_ENDPOINTS: ['topEndpoints'],
    TIME_SERIES: ['timeSeries'],
    APIS_METRICS: (page: number, limit: number) => ['apisMetrics', page, limit],
};

export const REFETCH_INTERVAL = 15_000; // Refresh every 15 seconds for real-time monitoring feel

export function useDashboardQuery(options = {}) {
    return useQuery({
        queryKey: QUERY_KEYS.DASHBOARD,
        queryFn: analyticsApi.getDashboard,
        refetchInterval: REFETCH_INTERVAL,
        ...options,
    });
}

export function useApisMetricsQuery(page: number, limit: number, options = {}) {
    return useQuery({
        queryKey: QUERY_KEYS.APIS_METRICS(page, limit),
        queryFn: () => analyticsApi.getApisMetrics({ page, limit }),
        refetchInterval: REFETCH_INTERVAL,
        ...options,
    });
}
