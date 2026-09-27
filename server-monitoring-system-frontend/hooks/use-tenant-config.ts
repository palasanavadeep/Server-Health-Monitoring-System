"use client";

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { clientApi, TenantConfig, TenantConfigUpdate, HistogramProfile } from '@/lib/api';

export const TENANT_CONFIG_KEYS = {
    CONFIG: (clientId: string) => ['tenantConfig', clientId],
    CURRENT_CONFIG: ['tenantConfig', 'current'],
    PROFILES: ['histogramProfiles'],
};

export function useTenantConfigQuery(clientId?: string, options = {}) {
    return useQuery<TenantConfig>({
        queryKey: TENANT_CONFIG_KEYS.CONFIG(clientId || ''),
        queryFn: () => clientApi.getTenantConfig(clientId!),
        enabled: !!clientId,
        staleTime: 30_000,
        ...options,
    });
}

export function useCurrentTenantConfigQuery(options = {}) {
    return useQuery<TenantConfig>({
        queryKey: TENANT_CONFIG_KEYS.CURRENT_CONFIG,
        queryFn: () => clientApi.getCurrentTenantConfig(),
        staleTime: 30_000,
        ...options,
    });
}

export function useUpdateTenantConfigMutation() {
    const queryClient = useQueryClient();

    return useMutation({
        mutationFn: ({ clientId, data }: { clientId: string; data: TenantConfigUpdate }) =>
            clientApi.updateTenantConfig(clientId, data),
        onSuccess: (_updated, variables) => {
            queryClient.invalidateQueries({ queryKey: TENANT_CONFIG_KEYS.CONFIG(variables.clientId) });
            queryClient.invalidateQueries({ queryKey: TENANT_CONFIG_KEYS.CURRENT_CONFIG });
            queryClient.invalidateQueries({ queryKey: ['apisMetrics'] });
        },
    });
}

export function useUpdateCurrentTenantConfigMutation() {
    const queryClient = useQueryClient();

    return useMutation({
        mutationFn: (data: TenantConfigUpdate) => clientApi.updateCurrentTenantConfig(data),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: TENANT_CONFIG_KEYS.CURRENT_CONFIG });
            queryClient.invalidateQueries({ queryKey: ['apisMetrics'] });
        },
    });
}

export function useHistogramProfilesQuery(options = {}) {
    return useQuery<HistogramProfile[]>({
        queryKey: TENANT_CONFIG_KEYS.PROFILES,
        queryFn: () => clientApi.getHistogramProfiles(),
        staleTime: 60_000 * 5, // profiles rarely change
        ...options,
    });
}
