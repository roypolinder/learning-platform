<script lang="ts">
	import { onMount } from 'svelte';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { EmailConfirmation } from '#lib/types/api.ts';
	import { takeLinkToken } from '#lib/utils/link-token.ts';

	let result = $state<EmailConfirmation | null>(null);
	let error = $state<string | null>(null);

	onMount(() => {
		const token = takeLinkToken();
		if (!token) {
			error = t('auth.link_missing');
			return;
		}
		authApi
			.confirmEmail(token)
			.then(async (confirmed) => {
				result = confirmed;
				// The banner and settings show the new status right away.
				await auth.load();
			})
			.catch((caught: unknown) => {
				if (caught instanceof ApiError && caught.code === 'invalid_token') error = t('auth.link_invalid');
				else if (caught instanceof ApiError && caught.code === 'email_taken') error = t('auth.email_taken');
				else error = caught instanceof ApiError ? caught.message : t('common.error');
			});
	});
</script>

<svelte:head><title>{t('auth.verify_title')} · {t('app.name')}</title></svelte:head>

<AuthLayout title={t('auth.verify_title')} subtitle={result || error ? '' : t('auth.verify_checking')}>
	<div class="flex flex-col gap-4">
		{#if result}
			<Notice tone="success">
				{result.purpose === 'change_email' ? t('auth.email_changed', { email: result.email }) : t('auth.verify_done', { email: result.email })}
			</Notice>
		{:else if error}
			<Notice tone="danger">{error}</Notice>
			{#if auth.user && !auth.user.email_verified}
				<Button href="/settings" class="h-9">{t('auth.request_new_link')}</Button>
			{/if}
		{/if}
		{#if result || error}
			<Button variant="primary" href={auth.user ? '/dashboard' : '/login'} class="h-9">{auth.user ? t('auth.continue') : t('auth.sign_in')}</Button>
		{/if}
	</div>
	{#snippet footer()}
		{#if auth.user}
			<a href="/settings" class="font-medium text-fg hover:underline">{t('nav.settings')}</a>
		{:else}
			<a href="/login" class="font-medium text-fg hover:underline">{t('auth.back_to_sign_in')}</a>
		{/if}
	{/snippet}
</AuthLayout>
