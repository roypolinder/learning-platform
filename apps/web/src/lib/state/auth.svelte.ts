import { i18n, isLocale, type Locale } from '#lib/i18n/index.svelte.ts';
import { authApi } from '#lib/services/api/auth.ts';
import type { User } from '#lib/types/api.ts';

class AuthState {
	user = $state<User | null>(null);
	ready = $state(false);

	get isSuperAdmin(): boolean {
		return this.user?.role === 'super_admin';
	}

	/** Signed-in users carry their language with their account. */
	set(user: User | null): void {
		this.user = user;
		if (user && isLocale(user.locale) && user.locale !== i18n.locale) i18n.set(user.locale);
	}

	/** Changes the interface language, and the account's language when signed in. */
	async setLocale(locale: Locale): Promise<void> {
		i18n.set(locale);
		if (this.user && this.user.locale !== locale) {
			this.user = await authApi.updateProfile({ locale });
		}
	}

	async load(): Promise<void> {
		try {
			this.set(await authApi.session());
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
