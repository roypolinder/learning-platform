import type { Locale } from '#lib/i18n/index.svelte.ts';
import type { EmailConfirmation, User } from '#lib/types/api.ts';
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
	updateProfile: (changes: { display_name?: string; locale?: Locale }) => api.patch<User>('/users/me', changes),

	forgotPassword: (email: string) => api.post<void>('/auth/password/forgot', { email }),
	resetPassword: (token: string, password: string) => api.post<void>('/auth/password/reset', { token, password }),
	confirmEmail: (token: string) => api.post<EmailConfirmation>('/auth/email/confirm', { token }),
	resendVerification: () => api.post<void>('/users/me/email/verification'),
	changeEmail: (email: string, password: string) => api.post<User>('/users/me/email', { email, password }),
	cancelEmailChange: () => api.delete<User>('/users/me/email/pending'),
	changePassword: (current_password: string, new_password: string) =>
		api.post<void>('/users/me/password', { current_password, new_password }),
	/** Deletes the signed-in user's own account. */
	deleteAccount: (password: string) => api.post<void>('/users/me/delete', { password })
};
