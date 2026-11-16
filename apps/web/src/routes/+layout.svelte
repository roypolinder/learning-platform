<script lang="ts">
	import './layout.css';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import AppShell from '#lib/components/layout/AppShell.svelte';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
	const standalone = ['/login', '/register', '/connect/authorize'];

	onMount(() => {
		void auth.load();
	});
</script>

<svelte:head>
	<title>Learning Platform</title>
	<meta name="description" content="Learn AI development by building real things with your own agent." />
</svelte:head>

{#if standalone.includes(page.url.pathname)}
	{@render children()}
{:else}
	<AppShell>{@render children()}</AppShell>
{/if}
