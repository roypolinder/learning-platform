<script lang="ts">
	import CodeBlock from '#lib/components/ui/CodeBlock.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Markdown from '#lib/components/ui/Markdown.svelte';
	import DiagramView from '#lib/features/diagrams/DiagramView.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import type { CourseStep } from '#lib/types/api.ts';
	import { asArray, asRecord, asString } from '#lib/utils/config.ts';

	let { step }: { step: CourseStep } = $props();

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
</script>

{#if step.type === 'diagram'}
	<DiagramView config={step.config} title={step.title} />
{/if}

{#each snippets as snippet, index (index)}
	<CodeBlock {...snippet} />
{/each}

{#if videoUrl}
	<a href={videoUrl} target="_blank" rel="noopener noreferrer" class="flex items-center gap-3 rounded-lg border border-line bg-surface px-4 py-3 hover:bg-surface-hover">
		<span class="grid size-8 place-items-center rounded-full bg-accent-soft text-accent"><Icon name="play" size={14} /></span>
		<span class="text-[13px] font-medium">{t('video.watch')}</span>
		<Icon name="external" size={14} class="ml-auto text-fg-subtle" />
	</a>
	{#if transcript}
		<details class="rounded-lg border border-line px-4 py-3 text-[13px]">
			<summary class="cursor-pointer font-medium">{t('video.transcript')}</summary>
			<Markdown source={transcript} class="mt-3" />
		</details>
	{/if}
{/if}
