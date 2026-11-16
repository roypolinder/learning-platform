<script lang="ts">
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { ValidationOutcome } from '#lib/types/api.ts';

	let { outcome, title }: { outcome: ValidationOutcome; title?: string } = $props();

	const tones = {
		passed: { box: 'border-transparent bg-success-soft', text: 'text-success', label: 'Passed' },
		failed: { box: 'border-transparent bg-warning-soft', text: 'text-warning', label: 'Not yet' },
		pending: { box: 'border-line bg-surface-muted', text: 'text-fg-muted', label: 'Pending' }
	} as const;
	let tone = $derived(tones[outcome.status]);
</script>

<section class="rounded-lg border {tone.box} px-4 py-3" aria-live="polite">
	<div class="flex items-center justify-between gap-3">
		<p class="text-[13px] font-medium {tone.text}">{title ?? tone.label} · <span class="font-normal">{outcome.summary}</span></p>
		{#if outcome.score !== undefined && outcome.score !== null}
			<span class="font-mono text-[12px] {tone.text}">{Math.round(outcome.score * 100)}%</span>
		{/if}
	</div>
	{#if outcome.checks.length}
		<ul class="mt-2.5 flex flex-col gap-1.5 border-t border-black/5 pt-2.5 dark:border-white/5">
			{#each outcome.checks as check (check.id)}
				<li class="flex items-start gap-2 text-[13px]">
					<span class="mt-0.5 {check.passed ? 'text-success' : 'text-danger'}">
						<Icon name={check.passed ? 'check' : 'x'} size={14} />
					</span>
					<span class="text-fg">{check.label}</span>
					{#if check.detail}<span class="ml-auto shrink-0 text-[12px] text-fg-muted">{check.detail}</span>{/if}
				</li>
			{/each}
		</ul>
	{/if}
</section>
