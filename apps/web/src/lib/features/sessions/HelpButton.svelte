<script lang="ts">
	import { i18n, t } from '#lib/i18n/index.svelte.ts';
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
					stepHelp: step.help,
					locale: i18n.locale
				})
			: ''
	);
</script>

<Button size="sm" variant="ghost" disabled={!step} onclick={() => dialog?.showModal()} aria-haspopup="dialog">
	<Icon name="help" size={14} /> {t('help.button')}
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
				<h2 id="help-title" class="text-[15px] font-semibold">{t('help.title')}</h2>
				<p class="mt-1 text-[13px] text-fg-muted">{t('help.body')}</p>
			</div>
			<button type="button" class="text-fg-muted hover:text-fg" aria-label={t('common.close')} onclick={() => dialog?.close()}>
				<Icon name="x" />
			</button>
		</div>
		{#if step}
			<CodeBlock code={prompt} filename={t('help.prompt_file')} wrap />
		{/if}
	</div>
</dialog>
