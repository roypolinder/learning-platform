<script lang="ts">
	import CodeBlock from '#lib/components/ui/CodeBlock.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Markdown from '#lib/components/ui/Markdown.svelte';
	import type { CourseStep } from '#lib/types/api.ts';
	import { asArray, asRecord, asString } from '#lib/utils/config.ts';

	let { step }: { step: CourseStep } = $props();

	let panels = $derived(
		asArray(step.config.panels).map((panel) => {
			const record = asRecord(panel);
			return { title: asString(record.title) ?? '', body: asString(record.body) ?? '' };
		})
	);
	let snippets = $derived(
		asArray(step.config.snippets).map((snippet) => {
			const record = asRecord(snippet);
			return {
				code: asString(record.code) ?? '',
				language: asString(record.language),
				filename: asString(record.filename),
				caption: asString(record.caption)
			};
		})
	);
	let videoUrl = $derived(step.type === 'video' ? asString(step.config.url) : undefined);
	let transcript = $derived(asString(step.config.transcript));
	let activePanel = $state(0);
</script>

{#if panels.length}
	<div class="overflow-hidden rounded-lg border border-line bg-surface">
		<div class="flex overflow-x-auto border-b border-line" role="tablist">
			{#each panels as panel, index (index)}
				<button
					type="button"
					role="tab"
					aria-selected={activePanel === index}
					onclick={() => (activePanel = index)}
					class="relative h-10 shrink-0 px-4 text-[13px] text-fg-muted hover:text-fg aria-selected:font-medium aria-selected:text-fg aria-selected:after:absolute aria-selected:after:inset-x-3 aria-selected:after:-bottom-px aria-selected:after:h-0.5 aria-selected:after:bg-accent"
				>
					<span class="mr-1.5 font-mono text-[11px] text-fg-subtle">{index + 1}</span>{panel.title}
				</button>
			{/each}
		</div>
		<div class="px-5 py-4" role="tabpanel">
			<Markdown source={panels[activePanel]?.body ?? ''} />
			<div class="mt-4 flex justify-between">
				<button type="button" class="text-[13px] text-fg-muted hover:text-fg disabled:opacity-0" disabled={activePanel === 0} onclick={() => activePanel--}>
					Previous
				</button>
				{#if activePanel < panels.length - 1}
					<button type="button" class="inline-flex items-center gap-1 text-[13px] font-medium text-accent" onclick={() => activePanel++}>
						{panels[activePanel + 1]?.title} <Icon name="arrow-right" size={13} />
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}

{#each snippets as snippet, index (index)}
	<CodeBlock {...snippet} />
{/each}

{#if videoUrl}
	<a href={videoUrl} target="_blank" rel="noopener noreferrer" class="flex items-center gap-3 rounded-lg border border-line bg-surface px-4 py-3 hover:bg-surface-hover">
		<span class="grid size-8 place-items-center rounded-full bg-accent-soft text-accent"><Icon name="play" size={14} /></span>
		<span class="text-[13px] font-medium">Watch the video</span>
		<Icon name="external" size={14} class="ml-auto text-fg-subtle" />
	</a>
	{#if transcript}
		<details class="rounded-lg border border-line px-4 py-3 text-[13px]">
			<summary class="cursor-pointer font-medium">Transcript</summary>
			<Markdown source={transcript} class="mt-3" />
		</details>
	{/if}
{/if}
