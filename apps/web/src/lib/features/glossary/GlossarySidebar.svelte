<script lang="ts">
	import { onMount } from 'svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import { i18n, t } from '#lib/i18n/index.svelte.ts';
	import { coursesApi } from '#lib/services/api/courses.ts';
	import type { CourseDetail } from '#lib/types/api.ts';
	import { formatRelative } from '#lib/utils/format.ts';
	import { renderInlineMarkdown } from '#lib/utils/markdown.ts';
	import { NotesState } from './notes-state.svelte.ts';

	/** Key terms of the course and the learner's own notes, always one click away. */
	let { course, onclose }: { course: CourseDetail; onclose: () => void } = $props();

	let tab = $state<'terms' | 'notes'>('terms');
	let query = $state('');
	let draft = $state('');
	let editing = $state<{ id: string; body: string } | null>(null);
	// The sidebar is recreated per course page, so the initial course is the one to edit.
	const courseId = (() => course.id)();
	let viewedCourse = $state(courseId);
	let titles = $state<Record<string, string>>({});

	const notes = new NotesState(courseId);

	onMount(() => {
		void notes.load();
	});

	// Titles for the "look at notes from" picker, in the current language.
	$effect(() => {
		void i18n.locale;
		coursesApi
			.list()
			.then((result) => (titles = Object.fromEntries(result.items.map((item) => [item.id, item.title]))))
			.catch(() => {});
	});

	// Alphabetical, so a term is easy to find by scanning. Terms match before definitions.
	let terms = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const sorted = [...course.glossary].sort((a, b) => a.term.localeCompare(b.term, undefined, { sensitivity: 'base' }));
		if (!needle) return sorted;
		const inTerm = sorted.filter((entry) => entry.term.toLowerCase().includes(needle));
		const inDefinition = sorted.filter((entry) => !inTerm.includes(entry) && entry.definition.toLowerCase().includes(needle));
		return [...inTerm, ...inDefinition];
	});
	let open = $state<Set<string>>(new Set());
	// While searching, show the definitions of the matches straight away.
	let searching = $derived(query.trim().length > 0);

	function toggle(term: string) {
		const next = new Set(open);
		if (next.has(term)) next.delete(term);
		else next.add(term);
		open = next;
	}
	let readOnly = $derived(viewedCourse !== course.id);
	let shown = $derived(notes.for(viewedCourse));

	async function add(event: SubmitEvent) {
		event.preventDefault();
		if (draft.trim() && (await notes.add(draft.trim()))) draft = '';
	}

	async function saveEdit() {
		if (editing && editing.body.trim() && (await notes.update(editing.id, editing.body.trim()))) editing = null;
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && !editing) onclose();
	}
</script>

<svelte:window {onkeydown} />

<aside
	id="glossary-sidebar"
	aria-label={t('glossary.title')}
	class="fixed top-12 right-0 bottom-0 z-20 flex w-full max-w-[420px] flex-col border-l border-line bg-surface shadow-lg xl:shadow-none"
>
	<header class="flex items-center gap-2 border-b border-line px-4 py-2.5">
		<h2 class="text-[13px] font-semibold">{t('glossary.title')}</h2>
		<button type="button" class="ml-auto rounded p-1 text-fg-subtle hover:bg-surface-hover hover:text-fg" aria-label={t('common.close')} onclick={onclose}>
			<Icon name="x" size={14} />
		</button>
	</header>

	<div class="flex gap-4 border-b border-line px-4 text-[13px]" role="tablist">
		{#each [['terms', t('glossary.terms')], ['notes', t('glossary.notes')]] as const as [id, label] (id)}
			<button
				type="button"
				role="tab"
				aria-selected={tab === id}
				onclick={() => (tab = id)}
				class="-mb-px border-b-2 border-transparent py-2 text-fg-muted hover:text-fg aria-selected:border-fg aria-selected:font-medium aria-selected:text-fg"
			>
				{label}{#if id === 'notes' && notes.for(course.id).length}<span class="ml-1.5 rounded bg-surface-muted px-1 font-mono text-[11px] text-fg-subtle">{notes.for(course.id).length}</span>{/if}
			</button>
		{/each}
	</div>

	<div class="min-h-0 flex-1 overflow-y-auto px-4 py-3" role="tabpanel">
		{#if tab === 'terms'}
			{#if course.glossary.length}
				<label class="relative mb-3 block">
					<span class="sr-only">{t('glossary.search')}</span>
					<Icon name="search" size={13} class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-fg-subtle" />
					<input
						bind:value={query}
						type="search"
						placeholder={t('glossary.search')}
						class="h-8 w-full rounded-md border border-line-strong bg-surface pr-2 pl-7 text-[13px] placeholder:text-fg-subtle focus:border-accent focus:outline-none"
					/>
				</label>
				<ul class="-mx-2 flex flex-col">
					{#each terms as entry (entry.term)}
						{@const expanded = searching || open.has(entry.term)}
						<li>
							<button
								type="button"
								aria-expanded={expanded}
								onclick={() => toggle(entry.term)}
								class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[14px] font-medium hover:bg-surface-hover"
							>
								<Icon name="chevron-right" size={12} class="text-fg-subtle transition-transform {expanded ? 'rotate-90' : ''}" />
								{entry.term}
							</button>
							{#if expanded}
								<p class="mb-2 ml-7 mr-2 text-[14px] leading-relaxed text-fg-muted">{@html renderInlineMarkdown(entry.definition)}</p>
							{/if}
						</li>
					{/each}
				</ul>
			{:else}
				<p class="text-[13px] text-fg-muted">{t('glossary.empty')}</p>
			{/if}
		{:else}
			{#if notes.error}<div class="mb-3"><Notice tone="danger">{notes.error}</Notice></div>{/if}

			{#if notes.courses.length > 1}
				<label class="mb-3 flex flex-col gap-1 text-[12px] text-fg-muted">
					{t('notes.other_course')}
					<select
						bind:value={viewedCourse}
						class="h-8 rounded-md border border-line-strong bg-surface px-2 text-[13px] text-fg focus:border-accent focus:outline-none"
					>
						{#each notes.courses as id (id)}
							<option value={id}>{id === course.id ? `${t('notes.this_course')} · ${course.title}` : (titles[id] ?? id)}</option>
						{/each}
					</select>
				</label>
			{/if}

			{#if readOnly}
				<p class="mb-3 text-[12px] text-fg-subtle">{t('notes.read_only')}</p>
			{:else}
				<form class="mb-4 flex flex-col gap-2" onsubmit={add}>
					<textarea
						bind:value={draft}
						rows="3"
						maxlength={5000}
						placeholder={t('notes.placeholder')}
						class="resize-y rounded-md border border-line-strong bg-surface px-2.5 py-2 text-[14px] leading-relaxed placeholder:text-fg-subtle focus:border-accent focus:outline-none"
					></textarea>
					<div class="flex justify-end">
						<Button type="submit" size="sm" variant="primary" disabled={!draft.trim()} loading={notes.saving && !editing}>{t('notes.add')}</Button>
					</div>
				</form>
			{/if}

			{#if !notes.loading && !shown.length}
				<p class="text-[13px] text-fg-muted">{t('notes.empty')}</p>
			{/if}
			<ul class="flex flex-col gap-2">
				{#each [...shown].reverse() as note (note.id)}
					<li class="rounded-md border border-line px-3 py-2">
						{#if editing?.id === note.id}
							<textarea
								bind:value={editing.body}
								rows="4"
								maxlength={5000}
								class="w-full resize-y rounded-md border border-line-strong bg-surface px-2 py-1.5 text-[13px] focus:border-accent focus:outline-none"
							></textarea>
							<div class="mt-2 flex justify-end gap-2">
								<Button size="sm" variant="ghost" onclick={() => (editing = null)}>{t('common.cancel')}</Button>
								<Button size="sm" variant="primary" loading={notes.saving} onclick={saveEdit}>{t('common.save')}</Button>
							</div>
						{:else}
							<p class="text-[14px] leading-relaxed whitespace-pre-wrap">{note.body}</p>
							<div class="mt-1.5 flex items-center gap-1 text-[11px] text-fg-subtle">
								<span>{formatRelative(note.updated_at)}</span>
								{#if !readOnly}
									<button type="button" class="ml-auto rounded p-1 hover:bg-surface-hover hover:text-fg" aria-label={t('common.edit')} onclick={() => (editing = { id: note.id, body: note.body })}>
										<Icon name="edit" size={12} />
									</button>
									<button type="button" class="rounded p-1 hover:bg-surface-hover hover:text-danger" aria-label={t('common.delete')} onclick={() => notes.remove(note.id)}>
										<Icon name="trash" size={12} />
									</button>
								{/if}
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</aside>
