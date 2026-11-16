<script lang="ts">
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import AcknowledgeAction from './AcknowledgeAction.svelte';
	import AgentEvidenceStep from './AgentEvidenceStep.svelte';
	import ContentBlocks from './ContentBlocks.svelte';
	import FillInStep from './FillInStep.svelte';
	import FreeTextStep from './FreeTextStep.svelte';
	import McpConnectionStep from './McpConnectionStep.svelte';
	import PrerequisiteStep from './PrerequisiteStep.svelte';
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
			<Icon name="lock" /> Finish the previous steps to unlock this one.
		</p>
	{:else if progress?.status === 'skipped'}
		<p class="rounded-lg border border-line bg-surface-muted px-4 py-3 text-[13px] text-fg-muted">
			Skipped — it didn't apply to your path through the course.
		</p>
	{:else if step.type === 'quiz' || step.type === 'multiple_choice'}
		<QuizStep {step} {progress} {session} />
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
				This step completes automatically once the platform observes the required activity.
			</p>
		{/if}
	{:else}
		<AcknowledgeAction
			{step}
			{progress}
			{session}
			label={step.category === 'content' ? 'Got it, continue' : 'Mark as done'}
		/>
	{/if}
</div>
