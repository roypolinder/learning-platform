<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import { page } from '$app/state';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import AuthForm from '#lib/features/authentication/AuthForm.svelte';

	const next = $derived(page.url.searchParams.get('next'));
</script>

<svelte:head><title>{t('auth.submit_register')} · {t('app.name')}</title></svelte:head>

<AuthLayout title={t('auth.register_title')} subtitle={t('auth.register_subtitle')}>
	<AuthForm mode="register" />
	{#snippet footer()}
		{t('auth.have_account')} <a href={next ? `/login?next=${encodeURIComponent(next)}` : '/login'} class="font-medium text-fg hover:underline">{t('auth.sign_in')}</a>
	{/snippet}
</AuthLayout>
