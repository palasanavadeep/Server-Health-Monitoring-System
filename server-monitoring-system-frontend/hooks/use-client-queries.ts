"use client";

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { clientApi, ApiKeyCreateInput, CreateClientUserInput } from '@/lib/api';

export const CLIENT_QUERY_KEYS = {
    CLIENTS: ['clients'],
    API_KEYS: (clientId: string) => ['apiKeys', clientId],
    USERS: (clientId?: string) => ['clientUsers', clientId || 'current'],
};

export function useClientsQuery(enabled = true) {
    return useQuery({
        queryKey: CLIENT_QUERY_KEYS.CLIENTS,
        queryFn: () => clientApi.getAllClients(),
        enabled,
    });
}

export function useCreateClientMutation() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: clientApi.createClient,
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.CLIENTS });
        },
    });
}

export function useClientUsersQuery(clientId?: string) {
    return useQuery({
        queryKey: CLIENT_QUERY_KEYS.USERS(clientId),
        queryFn: () => clientApi.getClientUsers(clientId),
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
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (userData: CreateClientUserInput) => clientApi.createClientUser(clientId, userData),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.USERS(clientId) });
            queryClient.invalidateQueries({ queryKey: CLIENT_QUERY_KEYS.USERS() });
        },
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
