import type { AuthorizationParams, AuthorizationPreview } from '#lib/types/api.ts';
import { api } from './client.ts';

export const oauthApi = {
	preview: (params: AuthorizationParams) =>
		api.get<AuthorizationPreview>(`/oauth/authorization?${new URLSearchParams(params)}`),
	decide: (params: AuthorizationParams, approve: boolean) =>
		api.post<{ redirect_to: string }>('/oauth/authorization', { params, approve })
};
