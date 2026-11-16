<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import CodeBlock from '#lib/components/ui/CodeBlock.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import OutcomePanel from '../OutcomePanel.svelte';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	let completed = $derived(progress?.status === 'completed');
	let prompt = $derived(`Submit the evidence for step "${step.id}" of my current course to the learning platform.`);
	let manualOpen = $state(false);
	let manualPayload = $state('{\n  \n}');
	let parseError = $state<string | null>(null);

	async function submitManually() {
		parseError = null;
		let payload: unknown;
		try {
			payload = JSON.parse(manualPayload);
		} catch {
			parseError = 'Evidence must be valid JSON.';
			return;
		}
		await session.submit(step.id, payload);
	}
</script>

<div class="flex flex-col gap-4">
	{#if !completed}
		<div class="rounded-lg border border-line bg-surface">
			<div class="flex items-center gap-2 border-b border-line px-4 py-2.5 text-[13px] font-medium">
				<Icon name="terminal" class="text-fg-muted" /> Ask your agent
			</div>
			<div class="flex flex-col gap-3 px-4 py-3">
				<p class="text-[13px] text-fg-muted">
					When the task is done, ask your agent to report it. It collects the evidence from your workspace and calls
					<code class="font-mono text-[12px]">report_task_result</code>; the platform validates it server-side.
				</p>
				<CodeBlock code={prompt} filename="Prompt" wrap />
				<p class="flex items-center gap-2 text-[12px] text-fg-subtle">
					<span class="size-1.5 animate-pulse rounded-full bg-accent"></span> Results appear here automatically.
				</p>
			</div>
		</div>
	{/if}

	{#if progress?.outcome}<OutcomePanel outcome={progress.outcome} />{/if}

	{#if completed}
		<div class="flex justify-end">
			<Button onclick={() => session.advance(step.id)}>Next step <Icon name="arrow-right" size={14} /></Button>
		</div>
	{:else if step.submission_schema}
		<details bind:open={manualOpen} class="rounded-lg border border-line px-4 py-3">
			<summary class="cursor-pointer text-[13px] text-fg-muted">Submit evidence manually</summary>
			<div class="mt-3 flex flex-col gap-3">
				<CodeBlock code={JSON.stringify(step.submission_schema, null, 2)} filename="Expected evidence (JSON Schema)" />
				<textarea
					bind:value={manualPayload}
					rows="8"
					spellcheck="false"
					class="rounded-md border border-line-strong bg-code px-3 py-2 font-mono text-[12.5px] focus:border-accent focus:outline-none"
				></textarea>
				{#if parseError}<Notice tone="danger">{parseError}</Notice>{/if}
				<div class="flex justify-end">
					<Button variant="primary" loading={session.pending} onclick={submitManually}>Submit evidence</Button>
				</div>
			</div>
		</details>
	{/if}
</div>
