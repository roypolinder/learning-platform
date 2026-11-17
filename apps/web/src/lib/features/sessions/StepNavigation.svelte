<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import StepStatusIcon from '#lib/components/ui/StepStatusIcon.svelte';
	import type { LearningSession } from './learning-session.svelte.ts';

	let { session }: { session: LearningSession } = $props();
</script>

<nav aria-label={t('learn.steps_nav')} class="flex flex-col gap-5">
	{#each session.detail?.course.sections ?? [] as section, sectionIndex (section.id)}
		<div>
			<p class="mb-1.5 px-2 text-[11px] font-medium tracking-wide text-fg-subtle uppercase">
				{sectionIndex + 1}. {section.title}
			</p>
			<ul class="flex flex-col gap-px">
				{#each section.steps as step (step.id)}
					{@const status = session.progressFor(step.id)?.status ?? 'locked'}
					<li>
						<button
							type="button"
							onclick={() => session.select(step.id)}
							aria-current={session.selectedStepId === step.id ? 'step' : undefined}
							class="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-left text-[13px] hover:bg-surface-hover aria-[current=step]:bg-surface-muted {status === 'locked' || status === 'skipped'
								? 'text-fg-subtle'
								: 'text-fg'}"
						>
							<StepStatusIcon {status} current={session.selectedStepId === step.id} />
							<span class="min-w-0 flex-1 truncate" class:line-through={status === 'skipped'}>{step.title}</span>
							{#if step.optional}<span class="text-[11px] text-fg-subtle">{t('common.optional')}</span>{/if}
						</button>
					</li>
				{/each}
			</ul>
		</div>
	{/each}
</nav>
