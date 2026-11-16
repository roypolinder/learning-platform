<script lang="ts">
	import type { Snippet } from 'svelte';

	type Tone = 'neutral' | 'accent' | 'success' | 'warning' | 'danger';
	let { tone = 'neutral', children }: { tone?: Tone; children: Snippet } = $props();

	const tones: Record<Tone, string> = {
		neutral: 'bg-surface-muted text-fg-muted border-line',
		accent: 'bg-accent-soft text-accent border-transparent',
		success: 'bg-success-soft text-success border-transparent',
		warning: 'bg-warning-soft text-warning border-transparent',
		danger: 'bg-danger-soft text-danger border-transparent'
	};
</script>

<span class="inline-flex h-5 items-center rounded border px-1.5 text-[11px] font-medium {tones[tone]}">
	{@render children()}
</span>
