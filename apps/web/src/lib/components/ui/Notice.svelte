<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from './Icon.svelte';

	type Tone = 'info' | 'success' | 'warning' | 'danger';
	let { tone = 'info', title, children }: { tone?: Tone; title?: string; children?: Snippet } = $props();

	const tones: Record<Tone, string> = {
		info: 'border-line bg-surface-muted text-fg-muted',
		success: 'border-transparent bg-success-soft text-success',
		warning: 'border-transparent bg-warning-soft text-warning',
		danger: 'border-transparent bg-danger-soft text-danger'
	};
</script>

<div class="flex gap-2.5 rounded-md border px-3 py-2.5 text-[13px] {tones[tone]}" role={tone === 'danger' ? 'alert' : 'status'}>
	<Icon name={tone === 'success' ? 'check' : 'alert'} class="mt-0.5" />
	<div class="min-w-0 flex-1">
		{#if title}<p class="font-medium">{title}</p>{/if}
		{#if children}<div class={title ? 'mt-0.5 opacity-90' : ''}>{@render children()}</div>{/if}
	</div>
</div>
