<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import EmptyState from '#lib/components/ui/EmptyState.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { CourseCatalog, type ProgressFilter } from '#lib/features/courses/catalog-state.svelte.ts';
	import CourseRow from '#lib/features/courses/CourseRow.svelte';
	import { i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { Difficulty } from '#lib/types/api.ts';

	const catalog = new CourseCatalog();
	const progressFilters: { value: ProgressFilter; label: MessageKey }[] = [
		{ value: 'all', label: 'courses.filter_all' },
		{ value: 'not_started', label: 'courses.filter_not_started' },
		{ value: 'in_progress', label: 'common.in_progress' },
		{ value: 'completed', label: 'common.completed' }
	];
	const difficulties: { value: Difficulty | null; label: MessageKey }[] = [
		{ value: null, label: 'courses.any_level' },
		{ value: 'beginner', label: 'difficulty.beginner' },
		{ value: 'intermediate', label: 'difficulty.intermediate' },
		{ value: 'advanced', label: 'difficulty.advanced' }
	];

	$effect(() => {
		// Track filters, sign-in (for progress) and language (for titles); debounce typing.
		void [catalog.query, catalog.category, catalog.difficulty, auth.user?.id, i18n.locale];
		const timer = setTimeout(() => void catalog.load(), 150);
		return () => clearTimeout(timer);
	});

	function reset() {
		catalog.query = '';
		catalog.category = null;
		catalog.difficulty = null;
		catalog.progressFilter = 'all';
	}
</script>

<svelte:head><title>{t('courses.title')} · {t('app.name')}</title></svelte:head>

<div class="mx-auto max-w-4xl px-6 py-8">
	<PageHeader title={t('courses.title')} description={t('courses.description')} />

	<div class="mt-5 flex flex-wrap items-center gap-3">
		<label class="relative min-w-60 flex-1">
			<span class="sr-only">{t('courses.search')}</span>
			<Icon name="search" size={14} class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-fg-subtle" />
			<input
				bind:value={catalog.query}
				type="search"
				placeholder={t('courses.search_placeholder')}
				class="h-8 w-full rounded-md border border-line-strong bg-surface pr-3 pl-8 text-[13px] placeholder:text-fg-subtle focus:border-accent focus:outline-none"
			/>
		</label>
		<select
			bind:value={catalog.difficulty}
			aria-label={t('courses.difficulty')}
			class="h-8 rounded-md border border-line-strong bg-surface px-2 text-[13px] focus:border-accent focus:outline-none"
		>
			{#each difficulties as option (option.label)}<option value={option.value}>{t(option.label)}</option>{/each}
		</select>
	</div>

	{#if auth.user}
		<div class="mt-4 flex gap-4 border-b border-line text-[13px]" role="tablist" aria-label={t('courses.your_progress')}>
			{#each progressFilters as filter (filter.value)}
				<button
					type="button"
					role="tab"
					aria-selected={catalog.progressFilter === filter.value}
					onclick={() => (catalog.progressFilter = filter.value)}
					class="-mb-px flex items-center gap-1.5 border-b-2 border-transparent pb-2 text-fg-muted hover:text-fg aria-selected:border-fg aria-selected:font-medium aria-selected:text-fg"
				>
					{t(filter.label)}
					<span class="rounded bg-surface-muted px-1.5 font-mono text-[11px] text-fg-subtle">{catalog.count(filter.value)}</span>
				</button>
			{/each}
		</div>
	{/if}

	<div class="mt-3 flex flex-wrap gap-1.5" role="group" aria-label={t('courses.category')}>
		{#each [null, ...catalog.categories] as category (category ?? 'all')}
			<button
				type="button"
				aria-pressed={catalog.category === category}
				onclick={() => (catalog.category = category)}
				class="h-7 rounded-md border border-line px-2.5 text-[12px] text-fg-muted hover:text-fg aria-pressed:border-fg aria-pressed:bg-fg aria-pressed:text-bg"
			>
				{category ?? t('courses.all_topics')}
			</button>
		{/each}
	</div>

	<div class="mt-6">
		{#if catalog.error}
			<Notice tone="danger">{catalog.error}</Notice>
		{:else if catalog.loading && !catalog.courses.length}
			<div class="flex flex-col gap-2"><Skeleton class="h-20" /><Skeleton class="h-20" /><Skeleton class="h-20" /></div>
		{:else if !catalog.visible.length}
			<EmptyState title={t('courses.empty_title')} description={t('courses.empty_body')}>
				{#snippet actions()}<button type="button" class="text-[13px] font-medium text-accent" onclick={reset}>{t('courses.clear')}</button>{/snippet}
			</EmptyState>
		{:else}
			<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
				{#each catalog.visible as course (course.id)}<CourseRow {course} progress={catalog.progress[course.id]} position={catalog.position(course.id)} />{/each}
			</div>
			<p class="mt-3 text-[12px] text-fg-subtle">
				{catalog.visible.length === 1 ? t('courses.count_one') : t('courses.count_other', { count: catalog.visible.length })}
			</p>
		{/if}
	</div>
</div>
