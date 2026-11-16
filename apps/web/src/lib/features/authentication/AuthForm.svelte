<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { auth } from '#lib/state/auth.svelte.ts';

	let { mode }: { mode: 'login' | 'register' } = $props();

	let email = $state('');
	let password = $state('');
	let displayName = $state('');
	let submitting = $state(false);
	let error = $state<string | null>(null);
	let fieldErrors = $state<Record<string, string>>({});

	function destination(): string {
		const next = page.url.searchParams.get('next');
		return next && next.startsWith('/') && !next.startsWith('//') ? next : '/';
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitting = true;
		error = null;
		fieldErrors = {};
		try {
			auth.user =
				mode === 'login'
					? await authApi.login({ email, password })
					: await authApi.register({ email, password, display_name: displayName });
			await goto(destination());
		} catch (caught) {
			if (caught instanceof ApiError) {
				fieldErrors = caught.fieldErrors();
				error = Object.keys(fieldErrors).length ? null : caught.message;
			} else {
				error = 'Something went wrong. Try again.';
			}
		} finally {
			submitting = false;
		}
	}
</script>

<form class="flex flex-col gap-4" onsubmit={submit} novalidate>
	{#if error}<Notice tone="danger">{error}</Notice>{/if}
	{#if mode === 'register'}
		<TextField label="Name" bind:value={displayName} autocomplete="name" required error={fieldErrors.display_name} />
	{/if}
	<TextField label="Email" type="email" bind:value={email} autocomplete="email" required error={fieldErrors.email} />
	<TextField
		label="Password"
		type="password"
		bind:value={password}
		autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
		required
		error={fieldErrors.password}
		hint={mode === 'register' ? 'At least 10 characters.' : undefined}
	/>
	<Button type="submit" variant="primary" loading={submitting} class="mt-1 h-9">
		{mode === 'login' ? 'Sign in' : 'Create account'}
	</Button>
</form>
