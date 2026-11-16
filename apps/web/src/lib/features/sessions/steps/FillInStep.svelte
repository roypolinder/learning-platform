<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import { asArray, asRecord, asString } from '#lib/utils/config.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import OutcomePanel from '../OutcomePanel.svelte';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	type Part = { kind: 'text'; value: string } | { kind: 'blank'; id: string };

	let parts: Part[] = $derived.by(() => {
		const template = asString(step.config.template) ?? '';
		return template.split(/(\{\{[a-z0-9_-]+\}\})/i).map((chunk): Part => {
			const match = /^\{\{([a-z0-9_-]+)\}\}$/i.exec(chunk);
			return match ? { kind: 'blank', id: match[1] } : { kind: 'text', value: chunk };
		});
	});
	let hints = $derived(
		new Map(asArray(step.config.blanks).map((b) => [asString(asRecord(b).id) ?? '', asString(asRecord(b).hint) ?? '']))
	);
	let blanks = $state<Record<string, string>>({});
	let completed = $derived(progress?.status === 'completed');
	let filled = $derived([...hints.keys()].every((id) => blanks[id]?.trim()));
	let failedChecks = $derived(new Set(progress?.outcome?.checks.filter((c) => !c.passed).map((c) => c.id)));
</script>

<div class="flex flex-col gap-4">
	<p class="text-[14px] font-medium">{asString(step.config.prompt)}</p>
	<pre class="overflow-x-auto rounded-lg border border-line bg-code px-4 py-3 font-mono text-[12.5px] leading-8">{#each parts as part, index (index)}{#if part.kind === 'text'}{part.value}{:else}<input
					bind:value={blanks[part.id]}
					disabled={completed}
					aria-label={hints.get(part.id) || part.id}
					title={hints.get(part.id)}
					size={Math.max(6, (blanks[part.id]?.length ?? 0) + 1)}
					class="rounded border bg-surface px-1.5 py-0.5 font-mono text-[12.5px] text-accent focus:outline-none {failedChecks.has(part.id) ? 'border-warning' : 'border-line-strong focus:border-accent'}"
				/>{/if}{/each}</pre>
	{#if progress?.outcome}<OutcomePanel outcome={progress.outcome} />{/if}
	<div class="flex justify-end">
		{#if completed}
			<Button onclick={() => session.advance(step.id)}>Next step <Icon name="arrow-right" size={14} /></Button>
		{:else}
			<Button variant="primary" disabled={!filled} loading={session.pending} onclick={() => session.submit(step.id, { blanks })}>
				Check
			</Button>
		{/if}
	</div>
</div>
