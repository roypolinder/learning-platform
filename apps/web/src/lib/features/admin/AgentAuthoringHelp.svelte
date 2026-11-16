<script lang="ts">
	import Icon from '#lib/components/ui/Icon.svelte';

	const tools = [
		['authoring_get_guide', 'format, schema and step types'],
		['authoring_list_courses', 'what already exists'],
		['authoring_get_course', 'current YAML of a course'],
		['authoring_validate_course', 'check a draft'],
		['authoring_publish_course', 'publish it']
	] as const;
</script>

<section class="rounded-lg border border-line bg-surface p-4 text-[13px]">
	<h2 class="flex items-center gap-2 font-medium"><Icon name="terminal" size={14} /> Author with your agent</h2>
	<p class="mt-1.5 text-fg-muted">
		Your MCP connection from <a href="/settings" class="text-accent hover:underline">Settings</a> unlocks authoring tools because
		you are a super admin. Learners never see them.
	</p>
	<ul class="mt-3 flex flex-col gap-1.5">
		{#each tools as [name, purpose] (name)}
			<li><code class="font-mono text-[12px]">{name}</code> <span class="text-fg-subtle">— {purpose}</span></li>
		{/each}
	</ul>
	<p class="mt-3 text-fg-muted">Try asking your agent:</p>
	<blockquote class="mt-1.5 rounded-md bg-surface-muted px-3 py-2 text-fg">
		"Use the learning platform to draft a beginner course about Docker volumes with a quiz and a file check. Validate it, show me
		the YAML, and publish when I say so."
	</blockquote>
	<p class="mt-3 text-[12px] text-fg-subtle">
		Already connected? Reload the MCP server in VS Code to see the new tools.
	</p>
</section>
