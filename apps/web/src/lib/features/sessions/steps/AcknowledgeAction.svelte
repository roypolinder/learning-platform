<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';

	interface Props {
		step: CourseStep;
		progress: StepProgress | null;
		session: LearningSession;
		label?: string;
	}

	let { step, progress, session, label = t('learn.mark_complete') }: Props = $props();
</script>

{#if progress?.status === 'completed'}
	<div class="flex items-center justify-between gap-3">
		<p class="flex items-center gap-2 text-[13px] text-success"><Icon name="check" /> {t('common.completed')}</p>
		<Button variant="secondary" onclick={() => session.advance(step.id)}>
			{t('common.next_step')} <Icon name="arrow-right" size={14} />
		</Button>
	</div>
{:else}
	<div class="flex justify-end">
		<Button variant="primary" loading={session.pending} onclick={() => session.complete(step.id)}>
			{label} <Icon name="arrow-right" size={14} />
		</Button>
	</div>
{/if}
