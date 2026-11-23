<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import AppShell from '#lib/components/layout/AppShell.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
	// Pages with their own chrome: the landing pages and the sign-in flow.
	const standalone = ['/', '/nl/', '/login', '/register', '/forgot-password', '/reset-password', '/verify-email', '/connect/authorize'];

	onMount(() => {
		void auth.load();
	});
</script>

<svelte:head>
	<title>{t('app.name')}</title>
	<meta name="description" content={t('app.description')} />
</svelte:head>

{#if standalone.includes(page.url.pathname) || page.url.pathname === '/nl'}
	{@render children()}
{:else}
	<AppShell>{@render children()}</AppShell>
{/if}
