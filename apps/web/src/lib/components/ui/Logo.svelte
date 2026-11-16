<script lang="ts">
	/** The favicon doubles as the brand mark, so replacing `static/favicon.svg` rebrands everything. */
	let { size = 20, class: className = '' }: { size?: number; class?: string } = $props();
</script>

<img src="/favicon.svg" alt="" width={size} height={size} class="shrink-0 object-contain {className}" />
