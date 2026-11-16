import { authApi } from '#lib/services/api/auth.ts';
import type { User } from '#lib/types/api.ts';

class AuthState {
	user = $state<User | null>(null);
	ready = $state(false);

	get isSuperAdmin(): boolean {
		return this.user?.role === 'super_admin';
	}

	async load(): Promise<void> {
		try {
			this.user = await authApi.session();
		} catch (error) {
			console.error(error);
			this.user = null;
		} finally {
			this.ready = true;
		}
	}

	async logout(): Promise<void> {
		await authApi.logout();
		this.user = null;
	}
}

export const auth = new AuthState();
