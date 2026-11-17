<script lang="ts">
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import EmptyState from '#lib/components/ui/EmptyState.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Markdown from '#lib/components/ui/Markdown.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import ProgressBar from '#lib/components/ui/ProgressBar.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import { LearningSession } from '#lib/features/sessions/learning-session.svelte.ts';
	import CourseCompleteCelebration from '#lib/features/sessions/CourseCompleteCelebration.svelte';
	import HelpButton from '#lib/features/sessions/HelpButton.svelte';
	import StepNavigation from '#lib/features/sessions/StepNavigation.svelte';
	import StepRenderer from '#lib/features/sessions/steps/StepRenderer.svelte';
	import { stepTypeLabel } from '#lib/features/sessions/step-labels.ts';
	import { goto } from '$app/navigation';
	import { requireAuth } from '#lib/state/require-auth.svelte.ts';
	import GlossarySidebar from '#lib/features/glossary/GlossarySidebar.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import { formatStepStatus } from '#lib/utils/format.ts';
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();
	requireAuth();

	let session = $state<LearningSession | null>(null);

	$effect(() => {
		const current = new LearningSession(params.sessionId);
		session = current;
		void current.load();
		return () => current.dispose();
	});

	// The API moves sessions on an outdated course version to the current one.
	$effect(() => {
		const id = session?.detail?.session.id;
		if (id && id !== params.sessionId) void goto(`/learn/${id}`, { replaceState: true });
	});

	let step = $derived(session?.selectedStep ?? null);
	let stepProgress = $derived(session?.selectedProgress ?? null);
	let previous = $derived(session?.neighbour(-1) ?? null);
	let next = $derived(session?.neighbour(1) ?? null);
	let courseDone = $derived(session?.detail?.session.status === 'completed');
	let unfinished = $derived(
		session?.steps.find(
			(candidate) => !candidate.optional && !['completed', 'skipped'].includes(session?.progressFor(candidate.id)?.status ?? '')
		) ?? null
	);

	// The terms & notes sidebar stays open across steps and visits once a learner opens it.
	const GLOSSARY_KEY = 'lp-glossary-open';
	let glossaryOpen = $state(readGlossaryPreference());
	function readGlossaryPreference(): boolean {
		try {
			return localStorage.getItem(GLOSSARY_KEY) === '1';
		} catch {
			return false;
		}
	}
	function toggleGlossary(open = !glossaryOpen) {
		glossaryOpen = open;
		try {
			localStorage.setItem(GLOSSARY_KEY, open ? '1' : '0');
		} catch {
			// Only remembered for this page view.
		}
	}

	let celebrating = $state(false);
	// null until the session has loaded, so revisiting a finished course doesn't celebrate again.
	let wasDone: boolean | null = null;
	$effect(() => {
		if (!session?.detail) {
			wasDone = null;
			return;
		}
		if (wasDone === false && courseDone) celebrating = true;
		wasDone = courseDone;
	});
</script>

<svelte:head><title>{session?.detail?.course.title ?? t('courses.title')} · {t('app.name')}</title></svelte:head>

{#if session?.loadError}
	<div class="mx-auto max-w-xl px-6 py-16">
		<EmptyState
			title={t(session.loadError.status === 404 ? 'learn.not_found' : 'learn.load_error')}
			description={session.loadError.status === 404 ? t('learn.not_found_body') : session.loadError.message}
		>
			{#snippet actions()}<Button href="/courses">{t('course.browse')}</Button>{/snippet}
		</EmptyState>
	</div>
{:else if !session?.detail}
	<div class="grid grid-cols-[260px_1fr] gap-8 p-6">
		<Skeleton class="h-96" />
		<div class="flex flex-col gap-3"><Skeleton class="h-8 w-1/2" /><Skeleton class="h-64" /></div>
	</div>
{:else}
	{@const detail = session.detail}
	<div class="sticky top-0 z-10 flex h-12 items-center gap-4 border-b border-line bg-surface/95 px-5 backdrop-blur">
		<a href="/courses/{detail.course.id}" class="flex min-w-0 items-center gap-2 text-[13px] text-fg-muted hover:text-fg">
			<Icon name="arrow-left" size={14} />
			<span class="truncate font-medium text-fg">{detail.course.title}</span>
		</a>
		<div class="ml-auto flex items-center gap-1">
			<HelpButton course={detail.course} {step} />
			<Button size="sm" variant="ghost" onclick={() => toggleGlossary()} aria-expanded={glossaryOpen} aria-controls="glossary-sidebar" title={t('glossary.open')}>
				<Icon name="notes" size={14} /> <span class="hidden sm:inline">{t('learn.glossary')}</span>
			</Button>
		</div>
		<div class="hidden w-56 items-center gap-3 md:flex">
			<ProgressBar value={detail.progress.percent} label={t('learn.progress')} />
			<span class="shrink-0 font-mono text-[12px] text-fg-muted">{detail.progress.percent}%</span>
		</div>
	</div>

	<div class="grid lg:grid-cols-[272px_minmax(0,1fr)] {glossaryOpen ? 'xl:pr-[420px]' : ''}">
		<aside class="hidden border-r border-line px-3 py-5 lg:block">
			<div class="sticky top-17"><StepNavigation {session} /></div>
		</aside>

		<article class="mx-auto w-full max-w-[720px] px-6 py-8">
			{#if courseDone}
				<div class="mb-6">
					<Notice tone="success" title={t('learn.course_completed')}>
						{t('learn.course_completed_body')}
						<a href="/courses" class="ml-1 font-medium underline underline-offset-2">{t('learn.next_course')}</a>
					</Notice>
				</div>
			{/if}

			{#if step}
				<header class="mb-8 border-b border-line pb-6">
					<p class="flex items-center gap-2 text-[12px] text-fg-subtle">
						<span>{session.selectedSection?.title}</span>
						<Icon name="chevron-right" size={12} />
						<span>{stepTypeLabel(step.type)}</span>
						{#if step.estimated_minutes}<span>· {step.estimated_minutes} min</span>{/if}
					</p>
					<div class="mt-1.5 flex items-start justify-between gap-4">
						<h1 class="text-[22px] font-semibold tracking-tight">{step.title}</h1>
						{#if stepProgress}
							<Badge tone={stepProgress.status === 'completed' ? 'success' : stepProgress.status === 'failed' ? 'warning' : 'neutral'}>
								{formatStepStatus(stepProgress.status)}
							</Badge>
						{/if}
					</div>
					{#if step.description}<p class="mt-1 text-[14px] text-fg-muted">{step.description}</p>{/if}
				</header>

				{#if session.remoteCompletion === step.id}
					<div class="mb-5"><Notice tone="success">{t('learn.agent_completed')}</Notice></div>
				{/if}

				{#if step.body}<Markdown source={step.body} class="mb-6" />{/if}

				{#if session.error}<div class="mb-4"><Notice tone="danger">{session.error}</Notice></div>{/if}

				{#key step.id}
					<StepRenderer {step} progress={stepProgress} {session} />
				{/key}

				<footer class="mt-10 flex justify-between border-t border-line pt-4 text-[13px]">
					{#if previous}
						<button type="button" class="flex items-center gap-1.5 text-fg-muted hover:text-fg" onclick={() => session?.select(previous.id)}>
							<Icon name="arrow-left" size={14} /> {previous.title}
						</button>
					{:else}<span></span>{/if}
					{#if next}
						<button type="button" class="flex items-center gap-1.5 text-fg-muted hover:text-fg" onclick={() => session?.select(next.id)}>
							{next.title} <Icon name="arrow-right" size={14} />
						</button>
					{:else if courseDone}
						<Button variant="primary" size="sm" onclick={() => (celebrating = true)}>
							{t('learn.finish')} <Icon name="check" size={14} />
						</Button>
					{:else if unfinished && unfinished.id !== step.id}
						<button type="button" class="flex items-center gap-1.5 text-fg-muted hover:text-fg" onclick={() => session?.select(unfinished.id)}>
							{t('learn.unfinished', { title: unfinished.title })} <Icon name="arrow-right" size={14} />
						</button>
					{/if}
				</footer>
			{/if}
		</article>
	</div>

	{#if glossaryOpen}
		<GlossarySidebar course={detail.course} onclose={() => toggleGlossary(false)} />
	{/if}

	{#if celebrating}
		<CourseCompleteCelebration
			courseTitle={detail.course.title}
			courseId={detail.course.id}
			stepCount={detail.progress.completed_required}
			onclose={() => (celebrating = false)}
		/>
	{/if}
{/if}
