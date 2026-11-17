<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import { page } from '$app/state';
	import AuthLayout from '#lib/components/layout/AuthLayout.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { oauthApi } from '#lib/services/api/oauth.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';
	import type { AuthorizationPreview } from '#lib/types/api.ts';

	requireAuth();

	const params = Object.fromEntries(page.url.searchParams);
	let preview = $state<AuthorizationPreview | null>(null);
	let error = $state<string | null>(null);
	let deciding = $state<'allow' | 'deny' | null>(null);
	let redirected = $state(false);
	let requested = false;

	$effect(() => {
		if (!auth.user || requested) return;
		requested = true;
		oauthApi
			.preview(params)
			.then((result) => (preview = result))
			.catch((caught) => (error = message(caught)));
	});

	function message(caught: unknown): string {
		return caught instanceof ApiError ? caught.message : 'Something went wrong. Try again.';
	}

	async function decide(approve: boolean) {
		deciding = approve ? 'allow' : 'deny';
		error = null;
		try {
			const { redirect_to } = await oauthApi.decide(params, approve);
			redirected = approve;
			window.location.assign(redirect_to);
		} catch (caught) {
			error = message(caught);
			deciding = null;
		}
	}
</script>

<svelte:head><title>{t('connect.title')} · {t('app.name')}</title></svelte:head>

<AuthLayout title={t('connect.title')} subtitle={t('connect.subtitle')}>
	<div class="flex flex-col gap-4">
		{#if error}<Notice tone="danger" title={t('connect.cannot')}>{error}</Notice>{/if}

		{#if redirected}
			<Notice tone="success" title={t('connect.granted')}>
				{t('connect.returning', { client: preview?.client_name ?? '' })}
			</Notice>
		{:else if preview}
			<p class="text-[14px]">{t('connect.asking', { client: preview.client_name })}</p>
			<ul class="flex list-disc flex-col gap-1 pl-5 text-[13px] text-fg-muted">
				<li>{t('connect.scope_1')}</li>
				<li>{t('connect.scope_2')}</li>
			</ul>
			<p class="text-[12px] text-fg-subtle">{t('connect.fine_print', { target: preview.redirect_target })}</p>
			<div class="flex gap-2">
				<Button variant="primary" class="h-9 flex-1" loading={deciding === 'allow'} disabled={deciding !== null} onclick={() => decide(true)}>
					{t('connect.allow')}
				</Button>
				<Button class="h-9 flex-1" loading={deciding === 'deny'} disabled={deciding !== null} onclick={() => decide(false)}>
					{t('connect.deny')}
				</Button>
			</div>
		{:else if !error}
			<Skeleton class="h-24" />
		{/if}
	</div>
	{#snippet footer()}
		{#if auth.user}
			{t('connect.signed_in_as', { email: auth.user.email })}
			<button type="button" class="font-medium text-fg hover:underline" onclick={() => auth.logout()}>{t('connect.not_you')}</button>
		{/if}
	{/snippet}
</AuthLayout>
