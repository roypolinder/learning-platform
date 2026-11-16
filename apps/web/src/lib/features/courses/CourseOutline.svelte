<script lang="ts">
	import type { CourseSection } from '#lib/types/api.ts';
	import { stepTypeLabel } from '#lib/features/sessions/step-labels.ts';

	let { sections }: { sections: CourseSection[] } = $props();
</script>

<ol class="flex flex-col gap-4">
	{#each sections as section, index (section.id)}
		<li class="overflow-hidden rounded-lg border border-line bg-surface">
			<div class="border-b border-line px-4 py-3">
				<p class="text-[13px] font-medium"><span class="mr-2 font-mono text-fg-subtle">{index + 1}</span>{section.title}</p>
				{#if section.description}<p class="mt-0.5 text-[12px] text-fg-muted">{section.description}</p>{/if}
			</div>
			<ul class="divide-y divide-line">
				{#each section.steps as step (step.id)}
					<li class="flex items-center gap-3 px-4 py-2 text-[13px]">
						<span class="min-w-0 flex-1 truncate">{step.title}</span>
						{#if step.optional}<span class="text-[11px] text-fg-subtle">optional</span>{/if}
						<span class="w-28 shrink-0 text-right text-[12px] text-fg-subtle">{stepTypeLabel(step.type)}</span>
					</li>
				{/each}
			</ul>
		</li>
	{/each}
</ol>
