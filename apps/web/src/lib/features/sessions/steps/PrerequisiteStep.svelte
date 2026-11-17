<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import { asArray, asString } from '#lib/utils/config.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();
	let courses = $derived(asArray(step.config.courses).map((c) => asString(c) ?? ''));
</script>

<div class="flex flex-col gap-3">
	<ul class="divide-y divide-line rounded-lg border border-line bg-surface">
		{#each courses as course (course)}
			<li class="flex items-center justify-between px-4 py-2.5 text-[13px]">
				<a href="/courses/{course}" class="font-medium hover:underline">{course}</a>
				{#if progress?.status === 'completed'}
					<span class="flex items-center gap-1 text-success"><Icon name="check" size={14} /> {t('prerequisite.completed')}</span>
				{:else}
					<span class="text-fg-subtle">{t('prerequisite.not_completed')}</span>
				{/if}
			</li>
		{/each}
	</ul>
	{#if progress?.status !== 'completed'}
		<div class="flex items-center justify-between gap-3">
			<p class="text-[13px] text-fg-muted">{t(step.optional ? 'prerequisite.optional' : 'prerequisite.finish_first')}</p>
			<Button loading={session.pending} onclick={() => session.recheck()}><Icon name="refresh" size={14} /> {t('common.check_again')}</Button>
		</div>
	{/if}
</div>
