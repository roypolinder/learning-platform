<script lang="ts">
	import { renderMarkdown } from '#lib/utils/markdown.ts';

	let { source, class: className = '' }: { source: string; class?: string } = $props();
	let html = $derived(renderMarkdown(source));
</script>

<!-- Markdown output escapes raw HTML; see utils/markdown.ts. -->
<div class="markdown {className}">{@html html}</div>
