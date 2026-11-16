import { goto } from '$app/navigation';
import { page } from '$app/state';
import { auth } from './auth.svelte.ts';

/** Redirects to the sign-in page once auth has resolved without a user. */
export function requireAuth(): void {
	$effect(() => {
		if (auth.ready && !auth.user) {
			void goto(`/login?next=${encodeURIComponent(page.url.pathname + page.url.search)}`);
		}
	});
}
