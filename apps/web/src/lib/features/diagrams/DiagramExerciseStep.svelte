<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Markdown from '#lib/components/ui/Markdown.svelte';
	import OutcomePanel from '#lib/features/sessions/OutcomePanel.svelte';
	import type { LearningSession } from '#lib/features/sessions/learning-session.svelte.ts';
	import { t } from '#lib/i18n/index.svelte.ts';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import { asArray, asRecord, asString } from '#lib/utils/config.ts';
	import DiagramCanvas from './DiagramCanvas.svelte';
	import { parseDiagram } from './layout.ts';
	import { nodeClass } from './node-style.ts';

	/**
	 * `slots`: drag labels into the empty boxes of a diagram.
	 * `order`: put labels in sequence. Both work with drag and drop or tap-to-place.
	 */
	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	let mode = $derived(step.config.mode === 'order' ? 'order' : 'slots');
	let diagram = $derived(parseDiagram(step.config));
	let chips = $derived(
		asArray(step.config.chips).map((chip) => {
			const record = asRecord(chip);
			return { id: asString(record.id) ?? '', label: asString(record.label) ?? '' };
		})
	);
	let slotIds = $derived(mode === 'order' ? chips.map((_, index) => String(index + 1)) : diagram.nodes.filter((n) => n.slot).map((n) => n.id));
	let labels = $derived(new Map(chips.map((chip) => [chip.id, chip.label])));

	/** Slot (node id, or position for `order`) → chip id. A passed attempt is shown again. */
	let placements = $state<Record<string, string>>(initialPlacements());

	function initialPlacements(): Record<string, string> {
		const feedback = asRecord(progress?.status === 'completed' ? progress.outcome?.feedback : null);
		if (step.config.mode === 'order') {
			return Object.fromEntries(asArray(feedback.order).map((chip, index) => [String(index + 1), asString(chip) ?? '']));
		}
		return Object.fromEntries(Object.entries(asRecord(feedback.slots)).map(([slot, chip]) => [slot, asString(chip) ?? '']));
	}
	let picked = $state<string | null>(null);
	let dragOver = $state<string | null>(null);

	let completed = $derived(progress?.status === 'completed');
	let placed = $derived(new Set(Object.values(placements)));
	let tray = $derived(chips.filter((chip) => !placed.has(chip.id)));
	let full = $derived(slotIds.every((id) => placements[id]));
	let results = $derived(new Map(progress?.outcome?.checks.map((check) => [check.id, check.passed]) ?? []));
	// Results describe the last submission; hide them once the learner changes something.
	let showResults = $state(true);

	function place(slot: string, chipId: string) {
		if (completed) return;
		const next = Object.fromEntries(Object.entries(placements).filter(([, chip]) => chip !== chipId));
		next[slot] = chipId;
		placements = next;
		picked = null;
		showResults = false;
	}

	function clear(slot: string) {
		if (completed) return;
		const { [slot]: _removed, ...rest } = placements;
		placements = rest;
		showResults = false;
	}

	function activateSlot(slot: string) {
		if (picked) place(slot, picked);
		else if (placements[slot]) clear(slot);
	}

	function drop(event: DragEvent, slot: string) {
		event.preventDefault();
		dragOver = null;
		const chipId = event.dataTransfer?.getData('text/plain');
		if (chipId && labels.has(chipId)) place(slot, chipId);
	}

	async function submit() {
		const payload =
			mode === 'order' ? { order: slotIds.map((id) => placements[id] ?? '') } : { slots: placements };
		await session.submit(step.id, payload);
		showResults = true;
	}

	function slotState(slot: string): 'right' | 'wrong' | null {
		if (completed) return 'right';
		if (!showResults || !results.has(slot)) return null;
		return results.get(slot) ? 'right' : 'wrong';
	}
</script>

{#snippet slotBox(slot: string, extraClass: string)}
	{@const chipId = placements[slot]}
	{@const state = slotState(slot)}
	<button
		type="button"
		disabled={completed}
		onclick={() => activateSlot(slot)}
		ondragover={(event) => {
			event.preventDefault();
			dragOver = slot;
		}}
		ondragleave={() => (dragOver = dragOver === slot ? null : dragOver)}
		ondrop={(event) => drop(event, slot)}
		aria-label={chipId ? t('diagram.remove', { label: labels.get(chipId) ?? '' }) : t('diagram.empty_slot')}
		class="flex size-full items-center justify-center gap-1.5 rounded-lg border-2 px-2 text-center text-[13px] leading-tight transition-colors
			{state === 'right'
			? 'border-success bg-success-soft font-medium'
			: state === 'wrong'
				? 'border-warning bg-warning-soft font-medium'
				: chipId
					? 'border-accent bg-accent-soft font-medium'
					: dragOver === slot || picked
						? 'border-dashed border-accent bg-surface text-fg-muted'
						: 'border-dashed border-line-strong bg-surface text-fg-subtle'} {extraClass}"
	>
		{#if chipId}
			{labels.get(chipId)}
			{#if state === 'right'}<Icon name="check" size={13} class="text-success" />{:else if state === 'wrong'}<Icon name="x" size={13} class="text-warning" />{/if}
		{:else}
			{t('diagram.empty_slot')}
		{/if}
	</button>
{/snippet}

<div class="flex flex-col gap-4">
	<div>
		<Markdown source={asString(step.config.prompt) ?? ''} class="text-[14px] font-medium" />
		{#if !completed}
			<p class="mt-1 text-[12px] text-fg-subtle">{t(mode === 'order' ? 'diagram.order_hint' : 'diagram.exercise_hint')}</p>
		{/if}
	</div>

	{#if mode === 'slots'}
		<DiagramCanvas nodes={diagram.nodes} edges={diagram.edges} label={step.title}>
			{#snippet node(placed)}
				{#if placed.slot}
					{@render slotBox(placed.id, '')}
				{:else}
					<div class="grid size-full place-items-center border px-2 text-center text-[13px] leading-tight font-medium {nodeClass(placed.kind)}">
						{placed.label}
					</div>
				{/if}
			{/snippet}
		</DiagramCanvas>
	{:else}
		<ol class="flex flex-col items-stretch gap-1.5">
			{#each slotIds as slot, index (slot)}
				<li class="flex flex-col items-center gap-1.5">
					<div class="flex w-full items-center gap-3">
						<span class="w-16 shrink-0 text-right font-mono text-[11px] text-fg-subtle">{t('diagram.position', { number: index + 1 })}</span>
						<div class="h-11 flex-1">{@render slotBox(slot, '')}</div>
					</div>
					{#if index < slotIds.length - 1}
						<svg class="ml-16 text-fg-subtle" width="12" height="14" viewBox="0 0 12 14" aria-hidden="true"><path d="M6 0v10M2 7l4 5 4-5" fill="none" stroke="currentColor" stroke-width="1.5" /></svg>
					{/if}
				</li>
			{/each}
		</ol>
	{/if}

	{#if !completed}
		<div class="rounded-lg border border-line bg-surface px-3 py-2.5">
			<p class="mb-2 text-[12px] font-medium text-fg-muted">{t('diagram.labels')}</p>
			<ul class="flex min-h-8 flex-wrap gap-1.5">
				{#each tray as chip (chip.id)}
					<li>
						<button
							type="button"
							draggable="true"
							ondragstart={(event) => {
								event.dataTransfer?.setData('text/plain', chip.id);
								picked = null;
							}}
							onclick={() => (picked = picked === chip.id ? null : chip.id)}
							aria-pressed={picked === chip.id}
							class="flex cursor-grab items-center gap-1 rounded-md border border-line-strong bg-surface px-2.5 py-1 text-[13px] hover:border-accent active:cursor-grabbing aria-pressed:border-accent aria-pressed:bg-accent-soft"
						>
							<Icon name="grip" size={12} class="text-fg-subtle" />{chip.label}
						</button>
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	{#if asString(step.config.caption)}<Markdown source={asString(step.config.caption) ?? ''} class="text-[14px] text-fg-muted" />{/if}

	{#if progress?.outcome && showResults}
		<OutcomePanel outcome={{ ...progress.outcome, checks: [] }} />
	{/if}

	<div class="flex items-center justify-end gap-3">
		{#if completed}
			<Button onclick={() => session.advance(step.id)}>{t('common.next_step')} <Icon name="arrow-right" size={14} /></Button>
		{:else}
			{#if Object.keys(placements).length}
				<Button
					variant="ghost"
					onclick={() => {
						placements = {};
						picked = null;
					}}>{t('diagram.reset')}</Button
				>
			{/if}
			<Button variant="primary" disabled={!full} loading={session.pending} onclick={submit}>{t('diagram.check')}</Button>
		{/if}
	</div>
</div>
