<script lang="ts">
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { authApi } from '#lib/services/api/auth.ts';
	import { ApiError } from '#lib/services/api/client.ts';

	let email = $state('');
	let sentTo = $state<string | null>(null);
	let submitting = $state(false);
	let error = $state<string | null>(null);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		submitting = true;
		error = null;
		try {
			await authApi.forgotPassword(email.trim());
			sentTo = email.trim();
		} catch (caught) {
			error = caught instanceof ApiError ? caught.message : t('common.error');
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head><title>{t('auth.forgot_title')} · {t('app.name')}</title></svelte:head>

<AuthLayout title={t('auth.forgot_title')} subtitle={t('auth.forgot_subtitle')}>
	{#if sentTo}
		<Notice tone="success">{t('auth.forgot_sent', { email: sentTo })}</Notice>
	{:else}
		<form class="flex flex-col gap-4" onsubmit={submit}>
			{#if error}<Notice tone="danger">{error}</Notice>{/if}
			<TextField label={t('auth.email')} type="email" bind:value={email} autocomplete="email" required />
			<Button type="submit" variant="primary" loading={submitting} class="mt-1 h-9">{t('auth.forgot_submit')}</Button>
		</form>
	{/if}
	{#snippet footer()}
		<a href="/login" class="font-medium text-fg hover:underline">{t('auth.back_to_sign_in')}</a>
	{/snippet}
</AuthLayout>
