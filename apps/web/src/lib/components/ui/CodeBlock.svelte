<script lang="ts">
	import Icon from './Icon.svelte';

	interface Props {
		code: string;
		filename?: string;
		language?: string;
		caption?: string;
		wrap?: boolean;
	}

	let { code, filename, language, caption, wrap = false }: Props = $props();
	let copied = $state(false);

	async function copy() {
		await navigator.clipboard.writeText(code);
		copied = true;
		setTimeout(() => (copied = false), 1500);
	}
</script>

<figure class="overflow-hidden rounded-lg border border-line bg-code">
	<div class="flex h-9 items-center justify-between border-b border-line px-3">
		<span class="font-mono text-[12px] text-fg-muted">{filename ?? language ?? 'snippet'}</span>
		<button
			type="button"
			onclick={copy}
			class="inline-flex items-center gap-1.5 rounded px-1.5 py-0.5 text-[12px] text-fg-muted hover:bg-surface-hover hover:text-fg"
		>
			<Icon name={copied ? 'check' : 'copy'} size={13} />
			{copied ? 'Copied' : 'Copy'}
		</button>
	</div>
	<pre class="{wrap ? 'whitespace-pre-wrap' : 'overflow-x-auto'} px-4 py-3 font-mono text-[12.5px] leading-relaxed text-fg"><code>{code}</code></pre>
	{#if caption}
		<figcaption class="border-t border-line px-3 py-2 text-[12px] text-fg-muted">{caption}</figcaption>
	{/if}
</figure>
