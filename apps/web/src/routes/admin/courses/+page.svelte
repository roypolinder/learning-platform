<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import AdminGate from '#lib/features/admin/AdminGate.svelte';
	import AgentAuthoringHelp from '#lib/features/admin/AgentAuthoringHelp.svelte';
	import { adminApi } from '#lib/services/api/admin.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { AdminCourse } from '#lib/types/api.ts';
	import { formatRelative } from '#lib/utils/format.ts';

	let courses = $state<AdminCourse[] | null>(null);
	let error = $state<string | null>(null);

	$effect(() => {
		if (!auth.isSuperAdmin) return;
		adminApi
			.courses()
			.then((result) => (courses = result))
			.catch((e) => (error = e instanceof ApiError ? e.message : 'Could not load courses.'));
	});
</script>

<svelte:head><title>Course studio · Learning Platform</title></svelte:head>

<AdminGate>
	<div class="mx-auto max-w-5xl px-6 py-8">
		<PageHeader title="Course studio" description="Create and publish courses here, or let your AI agent write them over MCP.">
			{#snippet actions()}
				<Button variant="primary" href="/admin/courses/new"><Icon name="plus" size={14} /> New course</Button>
			{/snippet}
		</PageHeader>

		<div class="mt-6 grid gap-8 lg:grid-cols-[minmax(0,1fr)_300px]">
			<div>
				{#if error}
					<Notice tone="danger">{error}</Notice>
				{:else if !courses}
					<div class="flex flex-col gap-2"><Skeleton class="h-14" /><Skeleton class="h-14" /><Skeleton class="h-14" /></div>
				{:else}
					<div class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
						{#each courses as course (course.id)}
							<a href="/admin/courses/{course.id}" class="group flex items-center gap-4 px-4 py-3 hover:bg-surface-hover">
								<div class="min-w-0 flex-1">
									<div class="flex flex-wrap items-center gap-2">
										<span class="text-[14px] font-medium">{course.title}</span>
										{#if course.source === 'file'}
											<Badge>File</Badge>
										{:else}
											<Badge tone="accent">Authored</Badge>
										{/if}
										{#if !course.published}<Badge tone="warning">Not listed</Badge>{/if}
									</div>
									<p class="mt-0.5 flex flex-wrap gap-x-3 text-[12px] text-fg-subtle">
										<span class="font-mono">{course.id}</span>
										{#if course.version}<span class="font-mono">v{course.version}</span>{/if}
										<span>{course.step_count} steps</span>
										<span>{course.session_count} learners · {course.completed_count} completed</span>
										<span>updated {formatRelative(course.updated_at)}</span>
									</p>
								</div>
								<Icon name="chevron-right" class="text-fg-subtle transition-transform group-hover:translate-x-0.5" />
							</a>
						{:else}
							<p class="px-4 py-8 text-center text-[13px] text-fg-muted">No courses yet.</p>
						{/each}
					</div>
					<p class="mt-3 text-[12px] text-fg-subtle">
						<strong class="font-medium">File</strong> courses come from YAML in the courses directory and are read-only here.
						<strong class="font-medium">Authored</strong> courses are stored in the database and edited in the studio.
					</p>
				{/if}
			</div>
			<aside><AgentAuthoringHelp /></aside>
		</div>
	</div>
</AdminGate>
