<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import { page } from '$app/state';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import AuthForm from '#lib/features/authentication/AuthForm.svelte';

	const next = $derived(page.url.searchParams.get('next'));
</script>

<svelte:head><title>{t('auth.sign_in')} · {t('app.name')}</title></svelte:head>

<AuthLayout title={t('auth.login_title')} subtitle={t('auth.login_subtitle')}>
	<AuthForm mode="login" />
	<p class="mt-3 text-right text-[13px]">
		<a href="/forgot-password" class="text-fg-muted hover:text-fg hover:underline">{t('auth.forgot_password')}</a>
	</p>
	{#snippet footer()}
		{t('auth.new_here')} <a href={next ? `/register?next=${encodeURIComponent(next)}` : '/register'} class="font-medium text-fg hover:underline">{t('auth.create_account')}</a>
	{/snippet}
</AuthLayout>
