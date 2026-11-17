import type { Locale } from '#lib/i18n/index.svelte.ts';
import type { User } from '#lib/types/api.ts';
import { api } from './client.ts';

interface AuthResponse {
	user: User;
}

export const authApi = {
	register: (input: { email: string; password: string; display_name: string; locale: Locale }) =>
		api.post<AuthResponse>('/auth/register', input).then((r) => r.user),
	login: (input: { email: string; password: string }) =>
		api.post<AuthResponse>('/auth/login', input).then((r) => r.user),
	logout: () => api.post<void>('/auth/logout'),
	session: () => api.get<{ user: User | null }>('/auth/session').then((r) => r.user),
	updateProfile: (changes: { display_name?: string; locale?: Locale }) => api.patch<User>('/users/me', changes)
};
