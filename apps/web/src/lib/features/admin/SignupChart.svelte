<script lang="ts">
	let { days }: { days: { day: string; count: number }[] } = $props();

	/** The last 30 days including days without sign-ups, oldest first. */
	const series = $derived.by(() => {
		const counts = new Map(days.map((entry) => [entry.day, entry.count]));
		const today = new Date();
		return Array.from({ length: 30 }, (_, index) => {
			const date = new Date(Date.UTC(today.getUTCFullYear(), today.getUTCMonth(), today.getUTCDate() - (29 - index)));
			const day = date.toISOString().slice(0, 10);
			return { day, count: counts.get(day) ?? 0 };
		});
	});
	const max = $derived(Math.max(1, ...series.map((entry) => entry.count)));
	let hovered = $state<number | null>(null);

	const label = (day: string) => new Date(`${day}T00:00:00Z`).toLocaleDateString('en', { day: 'numeric', month: 'short', timeZone: 'UTC' });
</script>

<figure class="rounded-lg border border-line bg-surface px-4 pt-3 pb-2">
	<figcaption class="flex items-baseline justify-between text-[13px]">
		<span class="font-medium">Sign-ups per day</span>
		<span class="text-[12px] text-fg-subtle">last 30 days (UTC) · max {max}</span>
	</figcaption>
	<div class="relative mt-3 flex h-28 items-end gap-0.5 border-b border-line" role="img" aria-label="Sign-ups per day for the last 30 days">
		{#each series as entry, index (entry.day)}
			<!-- Full-height hit area, wider than the bar itself. -->
			<div
				class="flex h-full flex-1 items-end"
				role="presentation"
				onmouseenter={() => (hovered = index)}
				onmouseleave={() => (hovered = null)}
			>
				<div
					class="w-full rounded-t-[4px] {hovered === index ? 'bg-accent-hover' : 'bg-accent'}"
					style="height: {entry.count ? Math.max(4, (entry.count / max) * 100) : 0}%"
				></div>
			</div>
		{/each}
		{#if hovered !== null}
			{@const entry = series[hovered]}
			<div
				class="pointer-events-none absolute -top-1 z-10 -translate-x-1/2 -translate-y-full rounded-md border border-line bg-surface px-2 py-1 text-[12px] whitespace-nowrap shadow-sm"
				style="left: {((hovered + 0.5) / series.length) * 100}%"
			>
				<span class="text-fg-muted">{label(entry.day)}</span> <span class="font-medium text-fg">{entry.count}</span>
			</div>
		{/if}
	</div>
	<div class="mt-1 flex justify-between text-[11px] text-fg-subtle">
		<span>{label(series[0].day)}</span><span>today</span>
	</div>
	<table class="sr-only">
		<caption>Sign-ups per day</caption>
		<thead><tr><th>Day</th><th>Sign-ups</th></tr></thead>
		<tbody>{#each series as entry (entry.day)}<tr><td>{entry.day}</td><td>{entry.count}</td></tr>{/each}</tbody>
	</table>
</figure>
