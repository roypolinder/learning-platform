<script lang="ts">
	import Badge from '#lib/components/ui/Badge.svelte';
	import ProgressBar from '#lib/components/ui/ProgressBar.svelte';
	import { t } from '#lib/i18n/index.svelte.ts';
	import type { SessionSummary } from '#lib/types/api.ts';
	import { formatRelative } from '#lib/utils/format.ts';

	let { summary }: { summary: SessionSummary } = $props();
</script>

<a href="/learn/{summary.session.id}" class="flex items-center gap-4 px-4 py-3 hover:bg-surface-hover">
	<div class="min-w-0 flex-1">
		<p class="truncate text-[14px] font-medium">{summary.course.title}</p>
		<p class="text-[12px] text-fg-subtle">{t('course.last_opened', { time: formatRelative(summary.session.last_accessed_at) })}</p>
	</div>
	{#if summary.session.status === 'completed'}
		<Badge tone="success">{t('common.completed')}</Badge>
	{:else}
		<div class="flex w-40 items-center gap-2">
			<ProgressBar value={summary.percent} />
			<span class="font-mono text-[11px] text-fg-muted">{summary.percent}%</span>
		</div>
	{/if}
</a>
