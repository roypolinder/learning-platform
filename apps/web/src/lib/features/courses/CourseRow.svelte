<script lang="ts">
	import Badge from '#lib/components/ui/Badge.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import ProgressBar from '#lib/components/ui/ProgressBar.svelte';
	import type { CourseProgress, CourseSummary } from '#lib/types/api.ts';
	import { formatDifficulty, formatMinutes } from '#lib/utils/format.ts';

	let { course, progress = null }: { course: CourseSummary; progress?: CourseProgress | null } = $props();
</script>

<a href="/courses/{course.id}" class="group flex items-start gap-4 px-4 py-4 transition-colors hover:bg-surface-hover">
	<div class="min-w-0 flex-1">
		<div class="flex items-center gap-2">
			<h3 class="text-[14px] font-medium">{course.title}</h3>
			{#if progress?.state === 'completed'}
				<Badge tone="success"><Icon name="check" size={11} strokeWidth={2} class="mr-1" />Completed</Badge>
			{:else if progress?.state === 'in_progress'}
				<Badge tone="accent">In progress</Badge>
			{/if}
		</div>
		<p class="mt-0.5 line-clamp-2 text-[13px] text-fg-muted">{course.summary}</p>
		<p class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px] text-fg-subtle">
			<span>{course.category}</span>
			<span>{formatDifficulty(course.difficulty)}</span>
			<span class="flex items-center gap-1"><Icon name="clock" size={12} />{formatMinutes(course.estimated_minutes)}</span>
			<span>{course.step_count} steps</span>
		</p>
		{#if progress?.state === 'in_progress'}
			<div class="mt-2.5 flex max-w-xs items-center gap-2.5">
				<ProgressBar value={progress.percent} label="{course.title} progress" />
				<span class="shrink-0 font-mono text-[11px] text-fg-subtle">{progress.percent}%</span>
			</div>
		{/if}
	</div>
	<Icon name="chevron-right" class="mt-1 text-fg-subtle transition-transform group-hover:translate-x-0.5" />
</a>
