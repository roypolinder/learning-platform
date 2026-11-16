<script lang="ts">
	import type { Snippet } from 'svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import EmptyState from '#lib/components/ui/EmptyState.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';

	let { children }: { children: Snippet } = $props();
	requireAuth();
</script>

{#if !auth.ready || !auth.user}
	<div class="mx-auto max-w-5xl px-6 py-8"><Skeleton class="h-64" /></div>
{:else if !auth.isSuperAdmin}
	<div class="mx-auto max-w-xl px-6 py-16">
		<EmptyState title="Super admins only" description="The course studio is available to platform administrators.">
			{#snippet actions()}<Button href="/courses">Browse courses</Button>{/snippet}
		</EmptyState>
	</div>
{:else}
	{@render children()}
{/if}
