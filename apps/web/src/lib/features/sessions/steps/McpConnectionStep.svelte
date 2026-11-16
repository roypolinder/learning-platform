<script lang="ts">
	import Icon from '#lib/components/ui/Icon.svelte';
	import McpSetup from '#lib/features/integrations/McpSetup.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import OutcomePanel from '../OutcomePanel.svelte';
	import AcknowledgeAction from './AcknowledgeAction.svelte';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();
</script>

<div class="flex flex-col gap-4">
	{#if progress?.status === 'completed'}
		{#if progress.outcome}<OutcomePanel outcome={progress.outcome} title="Connected" />{/if}
		<AcknowledgeAction {step} {progress} {session} />
	{:else}
		<McpSetup compact />
		<div class="flex items-center gap-2.5 rounded-lg border border-dashed border-line-strong px-4 py-3 text-[13px] text-fg-muted">
			<span class="relative flex size-2">
				<span class="absolute inline-flex size-full animate-ping rounded-full bg-accent opacity-50"></span>
				<span class="relative inline-flex size-2 rounded-full bg-accent"></span>
			</span>
			Waiting for your agent to call the platform. This page updates automatically.
			<Icon name="terminal" class="ml-auto text-fg-subtle" />
		</div>
	{/if}
</div>
