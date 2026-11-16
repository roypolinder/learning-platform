<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import { asArray, asRecord, asString } from '#lib/utils/config.ts';
	import { renderInlineMarkdown } from '#lib/utils/markdown.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import OutcomePanel from '../OutcomePanel.svelte';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	interface Question {
		id: string;
		prompt: string;
		multiple: boolean;
		options: { id: string; text: string }[];
	}

	let single = $derived(step.type === 'multiple_choice');
	let questions: Question[] = $derived.by(() => {
		const raw = single ? [{ ...step.config, id: 'answer' }] : asArray(step.config.questions);
		return raw.map((value) => {
			const record = asRecord(value);
			return {
				id: asString(record.id) ?? '',
				prompt: asString(record.prompt) ?? '',
				multiple: record.multiple === true,
				options: asArray(record.options).map((option) => {
					const o = asRecord(option);
					return { id: asString(o.id) ?? '', text: asString(o.text) ?? '' };
				})
			};
		});
	});

	let answers = $state<Record<string, string[]>>({});
	let completed = $derived(progress?.status === 'completed');
	let complete = $derived(questions.every((q) => (answers[q.id]?.length ?? 0) > 0));

	interface Feedback {
		question: string;
		correct: boolean;
		explanation: string | null;
		selected: { option: string; explanation: string }[];
	}
	let feedback = $derived.by(() => {
		const questionsFeedback = asArray(asRecord(progress?.outcome?.feedback).questions);
		return new Map(
			questionsFeedback.map((entry) => {
				const record = asRecord(entry);
				const item: Feedback = {
					question: asString(record.question) ?? '',
					correct: record.correct === true,
					explanation: asString(record.explanation) ?? null,
					selected: asArray(record.selected).map((s) => {
						const r = asRecord(s);
						return { option: asString(r.option) ?? '', explanation: asString(r.explanation) ?? '' };
					})
				};
				return [item.question, item];
			})
		);
	});

	function toggle(question: Question, optionId: string) {
		const current = answers[question.id] ?? [];
		answers[question.id] = question.multiple
			? current.includes(optionId)
				? current.filter((id) => id !== optionId)
				: [...current, optionId]
			: [optionId];
	}

	async function submit() {
		const payload = single ? { selected: answers.answer ?? [] } : { answers };
		await session.submit(step.id, payload);
	}
</script>

<div class="flex flex-col gap-6">
	{#each questions as question, index (question.id)}
		{@const result = feedback.get(question.id)}
		<fieldset class="flex flex-col gap-2" disabled={completed || session.pending}>
			<legend class="mb-2 text-[14px] font-medium">
				{#if !single}<span class="mr-2 font-mono text-[12px] text-fg-subtle">{index + 1}</span>{/if}
				{@html renderInlineMarkdown(question.prompt)}
				{#if question.multiple}<span class="ml-1 text-[12px] font-normal text-fg-subtle">Select all that apply</span>{/if}
			</legend>
			{#each question.options as option (option.id)}
				{@const selected = answers[question.id]?.includes(option.id) ?? false}
				<label
					class="flex cursor-pointer items-center gap-3 rounded-md border px-3 py-2.5 text-[14px] transition-colors {selected
						? 'border-accent bg-accent-soft'
						: 'border-line bg-surface hover:border-line-strong'}"
				>
					<input
						type={question.multiple ? 'checkbox' : 'radio'}
						name={`${step.id}-${question.id}`}
						checked={selected}
						onchange={() => toggle(question, option.id)}
						class="accent-[var(--accent)]"
					/>
					{@html renderInlineMarkdown(option.text)}
				</label>
			{/each}
			{#if result && !completed}
				<p class="flex items-start gap-1.5 text-[13px] {result.correct ? 'text-success' : 'text-warning'}">
					<Icon name={result.correct ? 'check' : 'alert'} size={14} class="mt-0.5" />
					{result.correct ? 'Correct.' : 'Not quite.'}
					{#each result.selected as note (note.option)} {note.explanation}{/each}
				</p>
			{:else if result?.explanation}
				<p class="text-[13px] text-fg-muted">{result.explanation}</p>
			{/if}
		</fieldset>
	{/each}

	{#if progress?.outcome}<OutcomePanel outcome={progress.outcome} />{/if}

	<div class="flex items-center justify-end gap-3">
		{#if completed}
			<Button onclick={() => session.advance(step.id)}>Next step <Icon name="arrow-right" size={14} /></Button>
		{:else}
			{#if progress?.attempts}<span class="text-[12px] text-fg-subtle">Attempt {progress.attempts + 1}</span>{/if}
			<Button variant="primary" disabled={!complete} loading={session.pending} onclick={submit}>Check answers</Button>
		{/if}
	</div>
</div>
