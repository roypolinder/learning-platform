<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import AdminGate from '#lib/features/admin/AdminGate.svelte';
	import AgentAuthoringHelp from '#lib/features/admin/AgentAuthoringHelp.svelte';
	import { COURSE_TEMPLATE } from '#lib/features/admin/course-template.ts';
	import { adminApi } from '#lib/services/api/admin.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { AdminCourseDetail, CourseIssue, CourseValidationReport, StepTypeInfo } from '#lib/types/api.ts';
	import type { PageProps } from './$types';

	let { params }: PageProps = $props();

	let isNew = $derived(params.courseId === 'new');
	let course = $state<AdminCourseDetail | null>(null);
	let source = $state('');
	let saved = $state('');
	let loadError = $state<string | null>(null);
	let report = $state<CourseValidationReport | null>(null);
	let publishIssues = $state<CourseIssue[]>([]);
	let validating = $state(false);
	let publishing = $state(false);
	let message = $state<{ tone: 'success' | 'danger'; text: string; href?: string } | null>(null);
	let stepTypes = $state<StepTypeInfo[]>([]);
	// Set before navigating to a newly created course so its success message survives the reload.
	let keepMessage = false;

	let readOnly = $derived(!isNew && course !== null && !course.editable);
	let dirty = $derived(source !== saved);
	let ready = $derived(isNew || course !== null);
	let issues = $derived(publishIssues.length ? publishIssues : (report?.issues ?? []));

	$effect(() => {
		if (!auth.isSuperAdmin) return;
		const id = params.courseId;
		const template = page.url.searchParams.get('from');
		course = null;
		report = null;
		if (!keepMessage) message = null;
		keepMessage = false;
		publishIssues = [];
		loadError = null;
		if (id === 'new') {
			if (template) {
				adminApi
					.course(template)
					.then((original) => {
						source = original.definition.replace(/^id:\s*\S+/m, `id: ${original.id}-copy`);
						saved = '';
					})
					.catch(() => (source = COURSE_TEMPLATE));
			} else {
				source = COURSE_TEMPLATE;
				saved = '';
			}
			return;
		}
		adminApi
			.course(id)
			.then((result) => {
				course = result;
				source = result.definition;
				saved = result.definition;
			})
			.catch((e) => (loadError = e instanceof ApiError ? e.message : 'Could not load the course.'));
	});

	$effect(() => {
		if (!auth.isSuperAdmin) return;
		adminApi
			.guide()
			.then((guide) => (stepTypes = guide.step_types))
			.catch(() => {});
	});

	// Validate as the author types.
	$effect(() => {
		const current = source;
		if (!ready || readOnly || !current.trim()) return;
		const timer = setTimeout(() => void validate(current), 600);
		return () => clearTimeout(timer);
	});

	async function validate(text = source) {
		validating = true;
		try {
			const result = await adminApi.validate(text);
			if (text === source) {
				report = result;
				publishIssues = [];
			}
		} catch (e) {
			message = { tone: 'danger', text: e instanceof ApiError ? e.message : 'Validation failed.' };
		} finally {
			validating = false;
		}
	}

	async function publish() {
		publishing = true;
		message = null;
		publishIssues = [];
		try {
			const result = await adminApi.publish(source);
			saved = source;
			const labels = {
				created: 'Course created and published.',
				new_version: `Version ${result.course.version} published.`,
				updated_in_place: `Version ${result.course.version} updated.`,
				unchanged: 'No content changes; nothing new to publish.'
			};
			message = { tone: 'success', text: labels[result.status], href: `/courses/${result.course.id}` };
			if (result.course.id !== params.courseId) {
				keepMessage = true;
				await goto(`/admin/courses/${result.course.id}`, { replaceState: true });
			} else if (course) {
				course = { ...course, version: result.course.version, published: true };
			}
		} catch (e) {
			if (e instanceof ApiError && e.code === 'invalid_course' && Array.isArray(e.details)) {
				publishIssues = e.details as CourseIssue[];
			}
			message = { tone: 'danger', text: e instanceof ApiError ? e.message : 'Could not publish.' };
		} finally {
			publishing = false;
		}
	}

	function onkeydown(event: KeyboardEvent) {
		const target = event.currentTarget as HTMLTextAreaElement;
		if (event.key === 'Tab' && !event.shiftKey) {
			event.preventDefault();
			const { selectionStart, selectionEnd } = target;
			source = `${source.slice(0, selectionStart)}  ${source.slice(selectionEnd)}`;
			queueMicrotask(() => target.setSelectionRange(selectionStart + 2, selectionStart + 2));
		}
		if ((event.metaKey || event.ctrlKey) && event.key === 's') {
			event.preventDefault();
			if (!readOnly && !publishing) void publish();
		}
	}
</script>

<svelte:head><title>{isNew ? 'New course' : (course?.title ?? 'Course')} · Course studio</title></svelte:head>

<svelte:window
	onbeforeunload={(event) => {
		if (dirty && !readOnly) event.preventDefault();
	}}
/>

<AdminGate>
	<div class="mx-auto max-w-6xl px-6 py-8">
		<a href="/admin/courses" class="inline-flex items-center gap-1.5 text-[13px] text-fg-muted hover:text-fg">
			<Icon name="arrow-left" size={14} /> Course studio
		</a>

		{#if loadError}
			<div class="mt-6"><Notice tone="danger">{loadError}</Notice></div>
		{:else if !ready}
			<div class="mt-6 flex flex-col gap-3"><Skeleton class="h-8 w-1/3" /><Skeleton class="h-[60vh]" /></div>
		{:else}
			<header class="mt-4 flex flex-wrap items-center justify-between gap-4">
				<div class="min-w-0">
					<h1 class="flex flex-wrap items-center gap-2 text-[20px] font-semibold tracking-tight">
						{isNew ? 'New course' : course?.title}
						{#if course?.version}<span class="font-mono text-[13px] font-normal text-fg-subtle">v{course.version}</span>{/if}
						{#if readOnly}<Badge>File · read-only</Badge>{/if}
						{#if dirty && !readOnly}<Badge tone="warning">Unpublished changes</Badge>{/if}
					</h1>
					{#if course}
						<p class="mt-0.5 text-[12px] text-fg-subtle">
							{course.session_count} learners · {course.completed_count} completed
						</p>
					{/if}
				</div>
				<div class="flex items-center gap-2">
					{#if course?.published}
						<Button href="/courses/{course.id}" variant="ghost"><Icon name="external" size={14} /> View</Button>
					{/if}
					{#if readOnly && course}
						<Button href="/admin/courses/new?from={course.id}">Use as template</Button>
					{:else}
						<Button loading={validating} onclick={() => validate()}>Validate</Button>
						<Button variant="primary" loading={publishing} disabled={!source.trim()} onclick={publish}>Publish</Button>
					{/if}
				</div>
			</header>

			{#if readOnly}
				<div class="mt-4">
					<Notice>
						This course is defined by a YAML file in the courses directory, which stays its source of truth. Edit the file, or
						use it as a template for a new authored course.
					</Notice>
				</div>
			{/if}
			{#if message}
				<div class="mt-4">
					<Notice tone={message.tone}>
						{message.text}
						{#if message.href}<a href={message.href} class="ml-1 font-medium underline underline-offset-2">Open course</a>{/if}
					</Notice>
				</div>
			{/if}

			<div class="mt-5 grid gap-6 lg:grid-cols-[minmax(0,1fr)_300px]">
				<div class="flex min-w-0 flex-col gap-3">
					<label class="sr-only" for="course-source">Course definition (YAML)</label>
					<textarea
						id="course-source"
						bind:value={source}
						{onkeydown}
						readonly={readOnly}
						spellcheck="false"
						autocapitalize="off"
						autocomplete="off"
						class="h-[65vh] w-full resize-y rounded-lg border border-line-strong bg-code p-4 font-mono text-[12.5px] leading-relaxed focus:border-accent focus:outline-none"
					></textarea>

					{#if !readOnly}
						{#if issues.length}
							<div class="rounded-lg border border-transparent bg-danger-soft p-3 text-[13px] text-danger" role="alert">
								<p class="font-medium">{issues.length} problem{issues.length === 1 ? '' : 's'} to fix before publishing</p>
								<ul class="mt-1.5 flex flex-col gap-1">
									{#each issues as issue, index (index)}
										<li class="flex gap-2">
											<code class="shrink-0 font-mono text-[12px] opacity-80">{issue.path}</code>
											<span class="whitespace-pre-wrap">{issue.message}</span>
										</li>
									{/each}
								</ul>
							</div>
						{:else if report?.valid && report.course}
							<Notice tone="success">
								Valid: {report.course.title} · v{report.course.version} · {report.course.section_count} sections,
								{report.course.step_count} steps
							</Notice>
						{/if}
					{/if}
				</div>

				<aside class="flex flex-col gap-4">
					<AgentAuthoringHelp />
					{#if stepTypes.length}
						<section class="rounded-lg border border-line bg-surface p-4 text-[13px]">
							<h2 class="font-medium">Step types</h2>
							<p class="mt-1 text-[12px] text-fg-subtle">
								Config details per type are in <span class="font-mono">docs/course-authoring.md</span>.
							</p>
							<ul class="mt-3 flex flex-col gap-2.5">
								{#each stepTypes as type (type.name)}
									<li>
										<code class="font-mono text-[12px] text-fg">{type.name}</code>
										<span class="ml-1 text-[11px] text-fg-subtle">{type.category}</span>
										<p class="text-[12px] text-fg-muted">{type.description}</p>
									</li>
								{/each}
							</ul>
						</section>
					{/if}
				</aside>
			</div>
		{/if}
	</div>
</AdminGate>
