<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, description, actions }: { title: string; description?: string; actions?: Snippet } = $props();
</script>

<header class="flex flex-wrap items-end justify-between gap-4 border-b border-line pb-5">
	<div class="min-w-0">
		<h1 class="text-[20px] font-semibold tracking-tight">{title}</h1>
		{#if description}<p class="mt-1 text-[14px] text-fg-muted">{description}</p>{/if}
	</div>
	{#if actions}<div class="flex items-center gap-2">{@render actions()}</div>{/if}
</header>
