<script lang="ts">
	import Markdown from '#lib/components/ui/Markdown.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { asString } from '#lib/utils/config.ts';
	import DiagramCanvas from './DiagramCanvas.svelte';
	import { parseDiagram } from './layout.ts';
	import { nodeClass } from './node-style.ts';

	/** An explanation diagram: select a box to read what it does. */
	let { config, title }: { config: Record<string, unknown>; title: string } = $props();

	let diagram = $derived(parseDiagram(config));
	let caption = $derived(asString(config.caption));
	let hasDetails = $derived(diagram.nodes.some((node) => node.detail));
	let selectedId = $state<string | null>(null);
	let selected = $derived(diagram.nodes.find((node) => node.id === selectedId) ?? null);
</script>

<figure class="flex flex-col gap-3">
	<DiagramCanvas nodes={diagram.nodes} edges={diagram.edges} label={title}>
		{#snippet node(placed)}
			{#if placed.detail}
				<button
					type="button"
					aria-pressed={selectedId === placed.id}
					onclick={() => (selectedId = selectedId === placed.id ? null : placed.id)}
					class="grid size-full place-items-center border px-2 text-center text-[13px] leading-tight font-medium transition-shadow hover:shadow-sm focus-visible:outline-2 focus-visible:outline-accent aria-pressed:ring-2 aria-pressed:ring-accent {nodeClass(placed.kind)}"
				>
					{placed.label}
				</button>
			{:else}
				<div class="grid size-full place-items-center border px-2 text-center text-[13px] leading-tight font-medium {nodeClass(placed.kind)}">
					{placed.label}
				</div>
			{/if}
		{/snippet}
	</DiagramCanvas>

	{#if selected?.detail}
		<div class="rounded-lg border border-line bg-surface px-4 py-3" aria-live="polite">
			<p class="text-[13px] font-semibold">{selected.label}</p>
			<Markdown source={selected.detail} class="mt-1 text-[14px]" />
		</div>
	{:else if hasDetails}
		<p class="text-[12px] text-fg-subtle">{t('diagram.select_hint')}</p>
	{/if}

	{#if caption}
		<figcaption><Markdown source={caption} class="text-[14px] text-fg-muted" /></figcaption>
	{/if}
</figure>
