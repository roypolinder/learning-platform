<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import EmptyState from '#lib/components/ui/EmptyState.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import ProgressBar from '#lib/components/ui/ProgressBar.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import CourseRow from '#lib/features/courses/CourseRow.svelte';
	import SessionRow from '#lib/features/dashboard/SessionRow.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { coursesApi } from '#lib/services/api/courses.ts';
	import { progressApi } from '#lib/services/api/progress.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';
	import type { CourseSummary, ProgressOverview } from '#lib/types/api.ts';

	requireAuth();

	let overview = $state<ProgressOverview | null>(null);
	let catalog = $state<CourseSummary[]>([]);
	let error = $state<string | null>(null);

	$effect(() => {
		if (!auth.user) return;
		Promise.all([progressApi.overview(), coursesApi.list()])
			.then(([progress, courses]) => {
				overview = progress;
				catalog = courses.items;
			})
			.catch((caught) => (error = caught instanceof ApiError ? caught.message : 'Could not load your dashboard.'));
	});

	let current = $derived(overview?.sessions.find((s) => s.session.status === 'active') ?? null);
	let others = $derived(overview?.sessions.filter((s) => s !== current) ?? []);
	let started = $derived(new Set(overview?.sessions.map((s) => s.course.id)));
	let suggestions = $derived(catalog.filter((course) => !started.has(course.id)).slice(0, 3));
	let greeting = $derived.by(() => {
		const hour = new Date().getHours();
		return hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening';
	});
</script>

<svelte:head><title>Home · Learning Platform</title></svelte:head>

<div class="mx-auto max-w-4xl px-6 py-8">
	<PageHeader title="{greeting}{auth.user ? `, ${auth.user.display_name.split(' ')[0]}` : ''}" description="Pick up where you left off, or start something new." />

	{#if error}
		<div class="mt-6"><Notice tone="danger">{error}</Notice></div>
	{:else if !overview}
		<div class="mt-6 flex flex-col gap-3"><Skeleton class="h-32" /><Skeleton class="h-20" /></div>
	{:else}
		<dl class="mt-6 grid grid-cols-3 divide-x divide-line rounded-lg border border-line bg-surface">
			{#each [['In progress', overview.stats.active_courses], ['Completed', overview.stats.completed_courses], ['Steps finished', overview.stats.steps_completed]] as [label, value] (label)}
				<div class="px-4 py-3">
					<dt class="text-[12px] text-fg-muted">{label}</dt>
					<dd class="mt-0.5 font-mono text-[18px] font-medium">{value}</dd>
				</div>
			{/each}
		</dl>

		<section class="mt-8">
			<h2 class="mb-3 text-[13px] font-medium text-fg-muted">Continue learning</h2>
			{#if current}
				<div class="rounded-lg border border-line bg-surface p-5">
					<div class="flex flex-wrap items-start justify-between gap-4">
						<div class="min-w-0">
							<p class="text-[12px] text-fg-subtle">{current.course.category}</p>
							<h3 class="mt-0.5 text-[16px] font-semibold tracking-tight">{current.course.title}</h3>
							{#if current.current_step}
								<p class="mt-1 text-[13px] text-fg-muted">Next: {current.current_step.title}</p>
							{/if}
						</div>
						<Button variant="primary" href="/learn/{current.session.id}">Resume <Icon name="arrow-right" size={14} /></Button>
					</div>
					<div class="mt-4 flex items-center gap-3">
						<ProgressBar value={current.percent} />
						<span class="shrink-0 font-mono text-[12px] text-fg-muted">{current.completed_required}/{current.total_required}</span>
					</div>
				</div>
			{:else}
				<EmptyState title="Nothing in progress" description="Start a course and it will show up here.">
					{#snippet actions()}<Button href="/courses" variant="primary">Browse courses</Button>{/snippet}
				</EmptyState>
			{/if}
		</section>

		{#if others.length}
			<section class="mt-8">
				<h2 class="mb-3 text-[13px] font-medium text-fg-muted">Recently accessed</h2>
				<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
					{#each others as summary (summary.session.id)}<SessionRow {summary} />{/each}
				</div>
			</section>
		{/if}

		{#if suggestions.length}
			<section class="mt-8">
				<div class="mb-3 flex items-baseline justify-between">
					<h2 class="text-[13px] font-medium text-fg-muted">Available courses</h2>
					<a href="/courses" class="text-[12px] text-fg-muted hover:text-fg">View all</a>
				</div>
				<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
					{#each suggestions as course (course.id)}<CourseRow {course} />{/each}
				</div>
			</section>
		{/if}
	{/if}
</div>
