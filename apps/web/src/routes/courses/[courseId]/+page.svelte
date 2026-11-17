<script lang="ts">
	import { goto } from '$app/navigation';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import EmptyState from '#lib/components/ui/EmptyState.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Markdown from '#lib/components/ui/Markdown.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import CourseOutline from '#lib/features/courses/CourseOutline.svelte';
	import { ApiError } from '#lib/services/api/client.ts';
	import { coursesApi } from '#lib/services/api/courses.ts';
	import { sessionsApi } from '#lib/services/api/sessions.ts';
	import { i18n, t, type MessageKey } from '#lib/i18n/index.svelte.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { CourseDetailResponse } from '#lib/types/api.ts';
	import { formatDifficulty, formatMinutes, formatRelative } from '#lib/utils/format.ts';
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();

	let data = $state<CourseDetailResponse | null>(null);
	let loadError = $state<ApiError | null>(null);
	let starting = $state(false);
	let startError = $state<string | null>(null);

	const extensionLabels = new Set(['mcp', 'quiz', 'filesystem', 'http', 'git', 'service']);
	const extensionLabel = (id: string) => (extensionLabels.has(id) ? t(`extension.${id}` as MessageKey) : id);

	$effect(() => {
		const id = params.courseId;
		// Refetch once auth resolves (to include the session) and when the language changes.
		void [auth.user, i18n.locale];
		coursesApi
			.get(id)
			.then((result) => (data = result))
			.catch((error) => (loadError = error instanceof ApiError ? error : new ApiError(0, 'unknown', String(error))));
	});

	async function start() {
		if (!auth.user) {
			await goto(`/login?next=${encodeURIComponent(`/courses/${params.courseId}`)}`);
			return;
		}
		starting = true;
		startError = null;
		try {
			const detail = await sessionsApi.start(params.courseId);
			await goto(`/learn/${detail.session.id}`);
		} catch (error) {
			startError = error instanceof ApiError ? error.message : t('course.start_error');
		} finally {
			starting = false;
		}
	}

	let session = $derived(data?.session ?? null);
	let actionLabel = $derived(
		t(
			!auth.user
				? 'course.sign_in_to_start'
				: session?.status === 'active'
					? 'course.continue'
					: session?.status === 'completed'
						? 'course.review'
						: 'course.start'
		)
	);
</script>

<svelte:head>
	<title>{data?.course.title ?? t('courses.title')} · {t('app.name')}</title>
	{#if data}<meta name="description" content={data.course.summary} />{/if}
</svelte:head>

<div class="mx-auto max-w-5xl px-6 py-8">
	<a href="/courses" class="inline-flex items-center gap-1.5 text-[13px] text-fg-muted hover:text-fg">
		<Icon name="arrow-left" size={14} /> {t('course.back')}
	</a>

	{#if loadError}
		<div class="mt-8">
			<EmptyState title={t(loadError.status === 404 ? 'course.not_found' : 'course.load_error')} description={loadError.message}>
				{#snippet actions()}<Button href="/courses">{t('course.browse')}</Button>{/snippet}
			</EmptyState>
		</div>
	{:else if !data}
		<div class="mt-6 flex flex-col gap-3"><Skeleton class="h-8 w-2/3" /><Skeleton class="h-4 w-1/2" /><Skeleton class="mt-6 h-64" /></div>
	{:else}
		{@const course = data.course}
		<header class="mt-5 flex flex-wrap items-start justify-between gap-6 border-b border-line pb-6">
			<div class="max-w-2xl">
				<p class="text-[12px] text-fg-subtle">{course.category}</p>
				<h1 class="mt-1 text-[24px] font-semibold tracking-tight">{course.title}</h1>
				<p class="mt-2 text-[15px] text-fg-muted">{course.summary}</p>
				<div class="mt-4 flex flex-wrap items-center gap-4 text-[13px] text-fg-muted">
					<span>{formatDifficulty(course.difficulty)}</span>
					<span class="flex items-center gap-1.5"><Icon name="clock" size={14} />{formatMinutes(course.estimated_minutes)}</span>
					<span>{t('course.steps_sections', { steps: course.step_count, sections: course.section_count })}</span>
					<span class="font-mono text-[12px] text-fg-subtle">v{course.version}</span>
				</div>
			</div>
			<div class="flex flex-col items-end gap-2">
				<Button variant="primary" loading={starting} onclick={start} class="h-9 px-4">
					{actionLabel} <Icon name="arrow-right" size={14} />
				</Button>
				{#if session}
					<p class="flex items-center gap-2 text-[12px] text-fg-subtle">
						{#if session.status === 'completed'}<Badge tone="success">{t('common.completed')}</Badge>{/if}
						{t('course.last_opened', { time: formatRelative(session.last_accessed_at) })}
					</p>
				{/if}
			</div>
		</header>
		{#if startError}<div class="mt-4"><Notice tone="danger">{startError}</Notice></div>{/if}

		<div class="mt-8 grid gap-10 lg:grid-cols-[minmax(0,1fr)_260px]">
			<div class="flex flex-col gap-8">
				<section>
					<h2 class="mb-3 text-[13px] font-medium text-fg-muted">{t('course.about')}</h2>
					<Markdown source={course.description} />
				</section>
				{#if course.objectives.length}
					<section>
						<h2 class="mb-3 text-[13px] font-medium text-fg-muted">{t('course.objectives')}</h2>
						<ul class="flex flex-col gap-2">
							{#each course.objectives as objective (objective)}
								<li class="flex items-start gap-2.5 text-[14px]"><Icon name="check" class="mt-1 text-accent" />{objective}</li>
							{/each}
						</ul>
					</section>
				{/if}
				<section>
					<h2 class="mb-3 text-[13px] font-medium text-fg-muted">{t('course.structure')}</h2>
					<CourseOutline sections={course.sections} />
				</section>
			</div>

			<aside class="flex flex-col gap-6 text-[13px]">
				{#if course.prerequisites.length}
					<section>
						<h2 class="mb-2 font-medium text-fg-muted">{t('course.before')}</h2>
						<ul class="flex flex-col gap-1.5">
							{#each course.prerequisites as prerequisite (prerequisite.title)}
								<li>
									{#if prerequisite.course}
										<a href="/courses/{prerequisite.course}" class="text-accent hover:underline">{prerequisite.title}</a>
									{:else}{prerequisite.title}{/if}
								</li>
							{/each}
						</ul>
					</section>
				{/if}
				<section>
					<h2 class="mb-2 font-medium text-fg-muted">{t('course.assessed')}</h2>
					<ul class="flex flex-col gap-1.5">
						{#each course.extensions as extension (extension)}
							<li class="flex items-center gap-2"><span class="size-1 rounded-full bg-fg-subtle"></span>{extensionLabel(extension)}</li>
						{/each}
					</ul>
				</section>
				{#if course.glossary.length}
					<section>
						<h2 class="mb-2 font-medium text-fg-muted">{t('course.key_terms')}</h2>
						<ul class="flex flex-wrap gap-1.5">
							{#each course.glossary as entry (entry.term)}
								<li class="rounded-md border border-line bg-surface px-2 py-0.5 text-[12px]" title={entry.definition}>{entry.term}</li>
							{/each}
						</ul>
					</section>
				{/if}
				{#if course.tags.length}
					<section class="flex flex-wrap gap-1.5">
						{#each course.tags as tag (tag)}<Badge>{tag}</Badge>{/each}
					</section>
				{/if}
			</aside>
		</div>
	{/if}
</div>
