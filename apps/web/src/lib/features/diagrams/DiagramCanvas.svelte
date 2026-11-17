<script lang="ts">
	import type { Snippet } from 'svelte';
	import { layoutDiagram, NODE_HEIGHT, NODE_WIDTH, type DiagramEdge, type DiagramNode, type PlacedNode } from './layout.ts';

	interface Props {
		nodes: DiagramNode[];
		edges: DiagramEdge[];
		/** Renders the inside of a node; defaults to its label. */
		node?: Snippet<[PlacedNode]>;
		label?: string;
	}

	let { nodes, edges, node, label }: Props = $props();

	let layout = $derived(layoutDiagram(nodes, edges));
	const markerId = `arrow-${Math.random().toString(36).slice(2, 8)}`;
</script>

<!-- Nodes are HTML so they can wrap text and act as buttons or drop targets; arrows are SVG. -->
<div class="overflow-x-auto rounded-lg border border-line bg-surface-muted/50 p-3" role="group" aria-label={label}>
	<div class="relative mx-auto" style="width: {layout.width}px; height: {layout.height}px">
		<svg class="absolute inset-0 text-fg-subtle" width={layout.width} height={layout.height} aria-hidden="true">
			<defs>
				<marker id={markerId} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
					<path d="M0 0L10 5L0 10z" fill="currentColor" />
				</marker>
			</defs>
			{#each layout.edges as edge, index (index)}
				<line
					x1={edge.x1}
					y1={edge.y1}
					x2={edge.x2}
					y2={edge.y2}
					stroke="currentColor"
					stroke-width="1.5"
					stroke-dasharray={edge.dashed ? '5 4' : undefined}
					marker-end="url(#{markerId})"
				/>
			{/each}
		</svg>
		{#each layout.edges.filter((edge) => edge.label) as edge, index (index)}
			<span
				class="absolute max-w-28 -translate-x-1/2 -translate-y-1/2 rounded bg-surface px-1 text-center text-[11px] leading-tight text-fg-muted"
				style="left: {(edge.x1 + edge.x2) / 2}px; top: {(edge.y1 + edge.y2) / 2}px"
			>
				{edge.label}
			</span>
		{/each}
		{#each layout.nodes as placed (placed.id)}
			<div class="absolute" style="left: {placed.x}px; top: {placed.y}px; width: {NODE_WIDTH}px; height: {NODE_HEIGHT}px">
				{#if node}
					{@render node(placed)}
				{:else}
					<div class="grid size-full place-items-center rounded-lg border border-line-strong bg-surface px-2 text-center text-[13px] font-medium">
						{placed.label}
					</div>
				{/if}
			</div>
		{/each}
	</div>
</div>
