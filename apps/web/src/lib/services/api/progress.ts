import type { ProgressOverview } from '#lib/types/api.ts';
import { api } from './client.ts';

export const progressApi = {
	overview: () => api.get<ProgressOverview>('/progress/overview')
};
