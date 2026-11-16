<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import CodeBlock from '#lib/components/ui/CodeBlock.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseDetail, CourseStep } from '#lib/types/api.ts';
	import { helpPrompt } from './help-prompt.ts';

	let { course, step }: { course: CourseDetail; step: CourseStep | null } = $props();

	let dialog = $state<HTMLDialogElement | null>(null);
	let prompt = $derived(
		step
			? helpPrompt({
					courseTitle: course.title,
					stepTitle: step.title,
					stepId: step.id,
					runsInBrowser: Boolean(step.client_task),
					courseHelp: course.help,
					stepHelp: step.help
				})
			: ''
	);
</script>

<Button size="sm" variant="ghost" disabled={!step} onclick={() => dialog?.showModal()} aria-haspopup="dialog">
	<Icon name="help" size={14} /> Help
</Button>

<dialog
	bind:this={dialog}
	aria-labelledby="help-title"
	class="m-auto w-[min(640px,calc(100vw-2rem))] rounded-xl border border-line bg-surface p-0 text-fg shadow-xl backdrop:bg-black/40"
	onclick={(event) => {
		if (event.target === dialog) dialog?.close();
	}}
>
	<div class="flex flex-col gap-3 p-5">
		<div class="flex items-start justify-between gap-3">
			<div>
				<h2 id="help-title" class="text-[15px] font-semibold">Stuck? Ask your agent to coach you</h2>
				<p class="mt-1 text-[13px] text-fg-muted">
					Copy this prompt into your coding agent. It guides you with questions and hints instead of doing the exercise for you.
				</p>
			</div>
			<button type="button" class="text-fg-muted hover:text-fg" aria-label="Close" onclick={() => dialog?.close()}>
				<Icon name="x" />
			</button>
		</div>
		{#if step}
			<CodeBlock code={prompt} filename="Coaching prompt" wrap />
		{/if}
	</div>
</dialog>
