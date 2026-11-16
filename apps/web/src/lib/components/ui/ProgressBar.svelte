<script lang="ts">
	let { value, label }: { value: number; label?: string } = $props();
	let clamped = $derived(Math.max(0, Math.min(100, value)));
</script>

<div
	class="h-1 w-full overflow-hidden rounded-full bg-surface-muted"
	role="progressbar"
	aria-valuemin={0}
	aria-valuemax={100}
	aria-valuenow={clamped}
	aria-label={label ?? 'Progress'}
>
	<div class="h-full rounded-full bg-accent transition-[width] duration-300" style:width="{clamped}%"></div>
</div>
