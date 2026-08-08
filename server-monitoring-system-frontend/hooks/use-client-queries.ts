"use client";

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { clientApi, ApiKeyCreateInput } from '@/lib/api';

export const CLIENT_QUERY_KEYS = {
    API_KEYS: (clientId: string) => ['apiKeys', clientId],
};

export function useCreateClientMutation() {
    return useMutation({
        mutationFn: clientApi.createClient,
    });
}

export function useClientApiKeysQuery(clientId: string) {
    return useQuery({
        queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId),
        queryFn: () => clientApi.getClientApiKeys(clientId),
        enabled: !!clientId,
    });
}

export function useCreateApiKeyMutation(clientId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: ApiKeyCreateInput) => 
            clientApi.createApiKey(clientId, data),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId) });
        },
    });
}

export function useCreateClientUserMutation(clientId: string) {
    return useMutation({
        mutationFn: (userData: any) => clientApi.createClientUser(clientId, userData),
    });
}

export function useUpdateApiKeyMutation(clientId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (data: { keyId: string } & ApiKeyCreateInput) => {
            const { keyId, ...rest } = data;
            return clientApi.updateApiKey(clientId, keyId, rest);
        },
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId) });
        },
    });
}

export function useDeleteApiKeyMutation(clientId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (keyId: string) => clientApi.deleteApiKey(clientId, keyId),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId) });
        },
    });
}

export function useDeactivateApiKeyMutation(clientId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (keyId: string) => clientApi.deactivateApiKey(clientId, keyId),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId) });
        },
    });
}

export function useActivateApiKeyMutation(clientId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (keyId: string) => clientApi.activateApiKey(clientId, keyId),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId) });
        },
    });
}

export function useRotateApiKeyMutation(clientId: string) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (keyId: string) => clientApi.rotateApiKey(clientId, keyId),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.API_KEYS(clientId) });
        },
    });
}
