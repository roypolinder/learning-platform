<script lang="ts">
	import Icon from '#lib/components/ui/Icon.svelte';
	import DiagramExerciseStep from '#lib/features/diagrams/DiagramExerciseStep.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import AcknowledgeAction from './AcknowledgeAction.svelte';
	import AgentEvidenceStep from './AgentEvidenceStep.svelte';
	import ContentBlocks from './ContentBlocks.svelte';
	import FillInStep from './FillInStep.svelte';
	import FreeTextStep from './FreeTextStep.svelte';
	import McpConnectionStep from './McpConnectionStep.svelte';
	import PrerequisiteStep from './PrerequisiteStep.svelte';
	import ProjectSetupStep from './ProjectSetupStep.svelte';
	import QuizStep from './QuizStep.svelte';
	import ServiceCheckStep from './ServiceCheckStep.svelte';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();
</script>

<!--
	Specialised views exist for known step types. Anything else falls back to its
	completion strategy, so new step types from new extensions work without UI changes.
-->
<div class="flex flex-col gap-5">
	<ContentBlocks {step} />

	{#if progress?.status === 'locked'}
		<p class="flex items-center gap-2 rounded-lg border border-line bg-surface-muted px-4 py-3 text-[13px] text-fg-muted">
			<Icon name="lock" /> {t('learn.locked')}
		</p>
	{:else if progress?.status === 'skipped'}
		<p class="rounded-lg border border-line bg-surface-muted px-4 py-3 text-[13px] text-fg-muted">
			{t('learn.skipped')}
		</p>
	{:else if step.type === 'quiz' || step.type === 'multiple_choice'}
		<QuizStep {step} {progress} {session} />
	{:else if step.type === 'diagram_exercise'}
		<DiagramExerciseStep {step} {progress} {session} />
	{:else if step.type === 'project_setup'}
		<ProjectSetupStep {step} {progress} {session} />
	{:else if step.type === 'free_text'}
		<FreeTextStep {step} {progress} {session} />
	{:else if step.type === 'interactive_exercise'}
		<FillInStep {step} {progress} {session} />
	{:else if step.type === 'mcp_connection_check'}
		<McpConnectionStep {step} {progress} {session} />
	{:else if step.client_task}
		<ServiceCheckStep {step} {progress} {session} />
	{:else if step.type === 'prerequisite'}
		<PrerequisiteStep {step} {progress} {session} />
	{:else if step.completion === 'submission'}
		<AgentEvidenceStep {step} {progress} {session} />
	{:else if step.completion === 'event' || step.completion === 'automatic'}
		{#if progress?.status === 'completed'}
			<AcknowledgeAction {step} {progress} {session} />
		{:else}
			<p class="rounded-lg border border-dashed border-line-strong px-4 py-3 text-[13px] text-fg-muted">
				{t('learn.automatic')}
			</p>
		{/if}
	{:else}
		<AcknowledgeAction
			{step}
			{progress}
			{session}
			label={t(step.category === 'content' ? 'learn.got_it' : 'learn.mark_done')}
		/>
	{/if}
</div>
