<script lang="ts">
	import type { HTMLInputAttributes } from 'svelte/elements';

	interface Props extends HTMLInputAttributes {
		label: string;
		value: string;
		error?: string;
		hint?: string;
	}

	let { label, value = $bindable(), error, hint, id, ...rest }: Props = $props();
	const generatedId = $props.id();
	let inputId = $derived(id ?? generatedId);
</script>

<div class="flex flex-col gap-1.5">
	<label for={inputId} class="text-[13px] font-medium text-fg">{label}</label>
	<input
		id={inputId}
		bind:value
		aria-invalid={error ? 'true' : undefined}
		aria-describedby={error || hint ? `${inputId}-note` : undefined}
		class="h-9 rounded-md border border-line-strong bg-surface px-3 text-[14px] text-fg placeholder:text-fg-subtle focus:border-accent focus:outline-none aria-invalid:border-danger"
		{...rest}
	/>
	{#if error}
		<p id="{inputId}-note" class="text-[12px] text-danger">{error}</p>
	{:else if hint}
		<p id="{inputId}-note" class="text-[12px] text-fg-subtle">{hint}</p>
	{/if}
</div>
