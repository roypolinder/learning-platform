<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import { asString } from '#lib/utils/config.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import OutcomePanel from '../OutcomePanel.svelte';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	let text = $state('');
	let words = $derived(text.trim() ? text.trim().split(/\s+/).length : 0);
	let completed = $derived(progress?.status === 'completed');
</script>

<div class="flex flex-col gap-4">
	<label class="flex flex-col gap-2">
		<span class="text-[14px] font-medium">{asString(step.config.prompt)}</span>
		<textarea
			bind:value={text}
			rows="5"
			disabled={completed}
			placeholder={asString(step.config.placeholder) ?? t('free_text.placeholder')}
			class="resize-y rounded-md border border-line-strong bg-surface px-3 py-2.5 text-[14px] leading-relaxed placeholder:text-fg-subtle focus:border-accent focus:outline-none"
		></textarea>
	</label>
	{#if progress?.outcome}<OutcomePanel outcome={progress.outcome} />{/if}
	<div class="flex items-center justify-between">
		<span class="text-[12px] text-fg-subtle">{t('free_text.words', { count: words })}</span>
		{#if completed}
			<Button onclick={() => session.advance(step.id)}>{t('common.next_step')} <Icon name="arrow-right" size={14} /></Button>
		{:else}
			<Button variant="primary" disabled={!words} loading={session.pending} onclick={() => session.submit(step.id, { text })}>
				{t('free_text.submit')}
			</Button>
		{/if}
	</div>
</div>
