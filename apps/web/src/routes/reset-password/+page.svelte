<script lang="ts">
	import { onMount } from 'svelte';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { takeLinkToken } from '#lib/utils/link-token.ts';

	let token = $state<string | null>(null);
	let loaded = $state(false);
	let password = $state('');
	let submitting = $state(false);
	let done = $state(false);
	let error = $state<string | null>(null);
	let invalidLink = $state(false);
	let fieldError = $state<string | undefined>();

	onMount(() => {
		token = takeLinkToken();
		loaded = true;
	});

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!token) return;
		submitting = true;
		error = null;
		fieldError = undefined;
		try {
			await authApi.resetPassword(token, password);
			// Every session was revoked, including this browser's.
			auth.user = null;
			done = true;
		} catch (caught) {
			if (caught instanceof ApiError && caught.code === 'invalid_token') invalidLink = true;
			else if (caught instanceof ApiError && caught.fieldErrors().password) fieldError = t('auth.password_hint');
			else error = caught instanceof ApiError ? caught.message : t('common.error');
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head><title>{t('auth.reset_title')} · {t('app.name')}</title></svelte:head>

<AuthLayout title={t('auth.reset_title')} subtitle={t('auth.reset_subtitle')}>
	{#if done}
		<div class="flex flex-col gap-4">
			<Notice tone="success">{t('auth.reset_done')}</Notice>
			<Button variant="primary" href="/login" class="h-9">{t('auth.sign_in')}</Button>
		</div>
	{:else if invalidLink || (loaded && !token)}
		<div class="flex flex-col gap-4">
			<Notice tone="danger">{invalidLink ? t('auth.link_invalid') : t('auth.link_missing')}</Notice>
			<Button href="/forgot-password" class="h-9">{t('auth.request_new_link')}</Button>
		</div>
	{:else if token}
		<form class="flex flex-col gap-4" onsubmit={submit}>
			{#if error}<Notice tone="danger">{error}</Notice>{/if}
			<TextField
				label={t('auth.new_password')}
				type="password"
				bind:value={password}
				autocomplete="new-password"
				required
				minlength={10}
				error={fieldError}
				hint={t('auth.password_hint')}
			/>
			<Button type="submit" variant="primary" loading={submitting} class="mt-1 h-9">{t('auth.reset_submit')}</Button>
		</form>
	{/if}
	{#snippet footer()}
		<a href="/login" class="font-medium text-fg hover:underline">{t('auth.back_to_sign_in')}</a>
	{/snippet}
</AuthLayout>
