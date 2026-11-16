<script lang="ts">
	import type { StepStatus } from '#lib/types/api.ts';
	import Icon from './Icon.svelte';

	let { status, current = false }: { status: StepStatus; current?: boolean } = $props();
</script>

{#if status === 'completed'}
	<span class="grid size-4 place-items-center rounded-full bg-accent text-accent-fg">
		<Icon name="check" size={10} strokeWidth={2.25} />
	</span>
{:else if status === 'skipped'}
	<span class="grid size-4 place-items-center rounded-full border border-line-strong text-fg-subtle">
		<Icon name="minus" size={10} />
	</span>
{:else if status === 'failed'}
	<span class="grid size-4 place-items-center rounded-full border-[1.5px] border-warning"></span>
{:else if status === 'locked'}
	<span class="grid size-4 place-items-center text-fg-subtle"><Icon name="lock" size={13} /></span>
{:else}
	<span
		class="grid size-4 place-items-center rounded-full border-[1.5px] {current || status === 'in_progress'
			? 'border-accent'
			: 'border-line-strong'}"
	>
		{#if status === 'in_progress'}<span class="size-1.5 rounded-full bg-accent"></span>{/if}
	</span>
{/if}
