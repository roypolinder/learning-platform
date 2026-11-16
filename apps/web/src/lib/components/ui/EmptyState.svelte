<script lang="ts">
	import type { Snippet } from 'svelte';

	let { title, description, actions }: { title: string; description?: string; actions?: Snippet } = $props();
</script>

<div class="rounded-lg border border-dashed border-line-strong px-6 py-10 text-center">
	<p class="text-[14px] font-medium text-fg">{title}</p>
	{#if description}<p class="mx-auto mt-1 max-w-sm text-[13px] text-fg-muted">{description}</p>{/if}
	{#if actions}<div class="mt-4 flex justify-center gap-2">{@render actions()}</div>{/if}
</div>
