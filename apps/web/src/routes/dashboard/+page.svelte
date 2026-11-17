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
	import { isIconName } from '#lib/components/ui/Icon.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { learnersApi } from '#lib/services/api/learners.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';
	import type { CourseSummary, Project, ProgressOverview } from '#lib/types/api.ts';

	requireAuth();

	let overview = $state<ProgressOverview | null>(null);
	let catalog = $state<CourseSummary[]>([]);
	let project = $state<Project | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		if (!auth.user) return;
		// Refetch when the language changes: course titles come back translated.
		void auth.user.locale;
		Promise.all([progressApi.overview(), coursesApi.list(), learnersApi.project()])
			.then(([progress, courses, chosen]) => {
				overview = progress;
				catalog = courses.items;
				project = chosen;
			})
			.catch((caught) => (error = caught instanceof ApiError ? caught.message : t('dashboard.load_error')));
	});

	let current = $derived(overview?.sessions.find((s) => s.session.status === 'active') ?? null);
	let others = $derived(overview?.sessions.filter((s) => s !== current) ?? []);
	let started = $derived(new Set(overview?.sessions.map((s) => s.course.id)));
	let suggestions = $derived(catalog.filter((course) => !started.has(course.id)).slice(0, 3));
	let greeting = $derived.by(() => {
		const hour = new Date().getHours();
		return t(hour < 12 ? 'dashboard.morning' : hour < 18 ? 'dashboard.afternoon' : 'dashboard.evening');
	});
</script>

<svelte:head><title>{t('dashboard.title')} · {t('app.name')}</title></svelte:head>

<div class="mx-auto max-w-4xl px-6 py-8">
	<PageHeader title="{greeting}{auth.user ? `, ${auth.user.display_name.split(' ')[0]}` : ''}" description={t('dashboard.description')} />

	{#if error}
		<div class="mt-6"><Notice tone="danger">{error}</Notice></div>
	{:else if !overview}
		<div class="mt-6 flex flex-col gap-3"><Skeleton class="h-32" /><Skeleton class="h-20" /></div>
	{:else}
		<section class="mt-6 flex items-center gap-3 rounded-lg border border-line bg-surface px-4 py-3">
			{#if project?.theme}
				<span class="grid size-8 shrink-0 place-items-center rounded-md bg-accent-soft text-accent">
					<Icon name={isIconName(project.theme.icon) ? project.theme.icon : 'globe'} />
				</span>
				<div class="min-w-0">
					<p class="text-[12px] text-fg-subtle">{t('dashboard.project')}</p>
					<p class="truncate text-[14px] font-medium">{project.theme.name} · <span class="font-normal text-fg-muted">{t(`project.stack.${project.stack}`)}</span></p>
				</div>
			{:else}
				<span class="grid size-8 shrink-0 place-items-center rounded-md bg-surface-muted text-fg-muted"><Icon name="globe" /></span>
				<p class="text-[13px] text-fg-muted">{t('dashboard.project_none')}</p>
			{/if}
		</section>

		<dl class="mt-4 grid grid-cols-3 divide-x divide-line rounded-lg border border-line bg-surface">
			{#each [[t('dashboard.stat_active'), overview.stats.active_courses], [t('dashboard.stat_completed'), overview.stats.completed_courses], [t('dashboard.stat_steps'), overview.stats.steps_completed]] as [label, value] (label)}
				<div class="px-4 py-3">
					<dt class="text-[12px] text-fg-muted">{label}</dt>
					<dd class="mt-0.5 font-mono text-[18px] font-medium">{value}</dd>
				</div>
			{/each}
		</dl>

		<section class="mt-8">
			<h2 class="mb-3 text-[13px] font-medium text-fg-muted">{t('dashboard.continue')}</h2>
			{#if current}
				<div class="rounded-lg border border-line bg-surface p-5">
					<div class="flex flex-wrap items-start justify-between gap-4">
						<div class="min-w-0">
							<p class="text-[12px] text-fg-subtle">{current.course.category}</p>
							<h3 class="mt-0.5 text-[16px] font-semibold tracking-tight">{current.course.title}</h3>
							{#if current.current_step}
								<p class="mt-1 text-[13px] text-fg-muted">{t('dashboard.next', { title: current.current_step.title })}</p>
							{/if}
						</div>
						<Button variant="primary" href="/learn/{current.session.id}">{t('dashboard.resume')} <Icon name="arrow-right" size={14} /></Button>
					</div>
					<div class="mt-4 flex items-center gap-3">
						<ProgressBar value={current.percent} />
						<span class="shrink-0 font-mono text-[12px] text-fg-muted">{current.completed_required}/{current.total_required}</span>
					</div>
				</div>
			{:else}
				<EmptyState title={t('dashboard.empty_title')} description={t('dashboard.empty_body')}>
					{#snippet actions()}<Button href={catalog[0] ? `/courses/${catalog[0].id}` : '/courses'} variant="primary">{t('dashboard.start_track')}</Button>{/snippet}
				</EmptyState>
			{/if}
		</section>

		{#if others.length}
			<section class="mt-8">
				<h2 class="mb-3 text-[13px] font-medium text-fg-muted">{t('dashboard.recent')}</h2>
				<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
					{#each others as summary (summary.session.id)}<SessionRow {summary} />{/each}
				</div>
			</section>
		{/if}

		{#if suggestions.length}
			<section class="mt-8">
				<div class="mb-3 flex items-baseline justify-between">
					<h2 class="text-[13px] font-medium text-fg-muted">{t('dashboard.next_courses')}</h2>
					<a href="/courses" class="text-[12px] text-fg-muted hover:text-fg">{t('dashboard.view_all')}</a>
				</div>
				<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
					{#each suggestions as course (course.id)}<CourseRow {course} position={catalog.indexOf(course) + 1} />{/each}
				</div>
			</section>
		{/if}
	{/if}
</div>
