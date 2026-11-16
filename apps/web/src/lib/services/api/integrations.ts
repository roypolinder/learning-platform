import type { IntegrationsResponse } from '#lib/types/api.ts';
import { api } from './client.ts';

export const integrationsApi = {
	list: () => api.get<IntegrationsResponse>('/integrations'),
	revoke: (id: string) => api.delete<void>(`/integrations/${id}`)
};
