<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import EmptyState from '#lib/components/ui/EmptyState.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { CourseCatalog, type ProgressFilter } from '#lib/features/courses/catalog-state.svelte.ts';
	import CourseRow from '#lib/features/courses/CourseRow.svelte';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { Difficulty } from '#lib/types/api.ts';

	const catalog = new CourseCatalog();
	const progressFilters: { value: ProgressFilter; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'not_started', label: 'Not started' },
		{ value: 'in_progress', label: 'In progress' },
		{ value: 'completed', label: 'Completed' }
	];
	const difficulties: { value: Difficulty | null; label: string }[] = [
		{ value: null, label: 'Any level' },
		{ value: 'beginner', label: 'Beginner' },
		{ value: 'intermediate', label: 'Intermediate' },
		{ value: 'advanced', label: 'Advanced' }
	];

	$effect(() => {
		// Track filter state and sign-in (for progress); debounce typing.
		void [catalog.query, catalog.category, catalog.difficulty, auth.user?.id];
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

<svelte:head><title>Courses · Learning Platform</title></svelte:head>

<div class="mx-auto max-w-4xl px-6 py-8">
	<PageHeader title="Courses" description="Hands-on courses you complete with your own editor and AI agent." />

	<div class="mt-5 flex flex-wrap items-center gap-3">
		<label class="relative min-w-60 flex-1">
			<span class="sr-only">Search courses</span>
			<Icon name="search" size={14} class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-fg-subtle" />
			<input
				bind:value={catalog.query}
				type="search"
				placeholder="Search by title, topic or tag"
				class="h-8 w-full rounded-md border border-line-strong bg-surface pr-3 pl-8 text-[13px] placeholder:text-fg-subtle focus:border-accent focus:outline-none"
			/>
		</label>
		<select
			bind:value={catalog.difficulty}
			aria-label="Difficulty"
			class="h-8 rounded-md border border-line-strong bg-surface px-2 text-[13px] focus:border-accent focus:outline-none"
		>
			{#each difficulties as option (option.label)}<option value={option.value}>{option.label}</option>{/each}
		</select>
	</div>

	{#if auth.user}
		<div class="mt-4 flex gap-4 border-b border-line text-[13px]" role="tablist" aria-label="Your progress">
			{#each progressFilters as filter (filter.value)}
				<button
					type="button"
					role="tab"
					aria-selected={catalog.progressFilter === filter.value}
					onclick={() => (catalog.progressFilter = filter.value)}
					class="-mb-px flex items-center gap-1.5 border-b-2 border-transparent pb-2 text-fg-muted hover:text-fg aria-selected:border-fg aria-selected:font-medium aria-selected:text-fg"
				>
					{filter.label}
					<span class="rounded bg-surface-muted px-1.5 font-mono text-[11px] text-fg-subtle">{catalog.count(filter.value)}</span>
				</button>
			{/each}
		</div>
	{/if}

	<div class="mt-3 flex flex-wrap gap-1.5" role="group" aria-label="Category">
		{#each [null, ...catalog.categories] as category (category ?? 'all')}
			<button
				type="button"
				aria-pressed={catalog.category === category}
				onclick={() => (catalog.category = category)}
				class="h-7 rounded-md border border-line px-2.5 text-[12px] text-fg-muted hover:text-fg aria-pressed:border-fg aria-pressed:bg-fg aria-pressed:text-bg"
			>
				{category ?? 'All topics'}
			</button>
		{/each}
	</div>

	<div class="mt-6">
		{#if catalog.error}
			<Notice tone="danger">{catalog.error}</Notice>
		{:else if catalog.loading && !catalog.courses.length}
			<div class="flex flex-col gap-2"><Skeleton class="h-20" /><Skeleton class="h-20" /><Skeleton class="h-20" /></div>
		{:else if !catalog.visible.length}
			<EmptyState title="No courses match" description="Try a different search term or clear the filters.">
				{#snippet actions()}<button type="button" class="text-[13px] font-medium text-accent" onclick={reset}>Clear filters</button>{/snippet}
			</EmptyState>
		{:else}
			<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
				{#each catalog.visible as course (course.id)}<CourseRow {course} progress={catalog.progress[course.id]} />{/each}
			</div>
			<p class="mt-3 text-[12px] text-fg-subtle">{catalog.visible.length} course{catalog.visible.length === 1 ? '' : 's'}</p>
		{/if}
	</div>
</div>
