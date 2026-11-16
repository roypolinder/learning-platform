<script lang="ts">
	import { onMount } from 'svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import CodeBlock from '#lib/components/ui/CodeBlock.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import { formatRelative } from '#lib/utils/format.ts';
	import { McpConnections } from './connection-state.svelte.ts';

	let { compact = false }: { compact?: boolean } = $props();

	const connections = new McpConnections();

	onMount(() => {
		void connections.load();
		// Connection status changes when the agent calls in; refresh while visible.
		const interval = setInterval(() => {
			if (document.visibilityState === 'visible') void connections.load();
		}, 15000);
		return () => clearInterval(interval);
	});
</script>

<section class="rounded-lg border border-line bg-surface">
	<header class="flex items-center justify-between gap-3 border-b border-line px-4 py-3">
		<div class="flex items-center gap-2.5">
			<Icon name="plug" class="text-fg-muted" />
			<div>
				<p class="text-[13px] font-medium">Agent connection</p>
				<p class="font-mono text-[11px] text-fg-subtle">{connections.endpoint || '…'}</p>
			</div>
		</div>
		{#if connections.connected}
			<span class="flex items-center gap-1.5 text-[12px] text-success">
				<span class="size-1.5 rounded-full bg-success"></span>
				Connected{connections.connected.last_seen_at ? ` · ${formatRelative(connections.connected.last_seen_at)}` : ''}
			</span>
		{:else if connections.active.length}
			<span class="flex items-center gap-1.5 text-[12px] text-fg-muted">
				<span class="size-1.5 rounded-full bg-warning"></span> Waiting for first call
			</span>
		{:else if !connections.loading}
			<span class="text-[12px] text-fg-subtle">Not connected</span>
		{/if}
	</header>

	<div class="flex flex-col gap-3 px-4 py-3">
		{#if connections.error}<Notice tone="danger">{connections.error}</Notice>{/if}

		{#if connections.config}
			<CodeBlock
				filename=".vscode/mcp.json"
				language="json"
				code={JSON.stringify(connections.config, null, 2)}
				caption="Save it in the project you open in VS Code."
			/>
		{/if}
		<ol class="flex list-decimal flex-col gap-1 pl-5 text-[13px] text-fg-muted">
			<li>Add the configuration above. It contains no secrets, so it is safe to commit.</li>
			<li>When VS Code shows that <code class="font-mono text-[12px]">learning-platform</code> needs authentication, choose <span class="font-medium text-fg">Authenticate</span>.</li>
			<li>Approve access in the browser tab that opens. Your agent can only see and update your own course sessions.</li>
		</ol>

		{#if !compact && connections.connections.length}
			<ul class="divide-y divide-line rounded-md border border-line">
				{#each connections.connections as connection (connection.id)}
					<li class="flex items-center gap-3 px-3 py-2 text-[13px]">
						<span class="size-1.5 rounded-full {connection.status === 'connected' ? 'bg-success' : connection.status === 'revoked' ? 'bg-line-strong' : 'bg-warning'}"></span>
						<span class="font-medium">{connection.label}</span>
						{#if connection.auth_method === 'oauth'}
							<span class="text-[11px] text-fg-subtle">Signed in</span>
						{:else}
							<code class="font-mono text-[11px] text-fg-subtle">{connection.token_hint}</code>
						{/if}
						<span class="ml-auto text-[12px] text-fg-muted">
							{#if connection.revoked_at}Revoked
							{:else if connection.last_seen_at}{connection.client_name ?? 'Agent'} · {formatRelative(connection.last_seen_at)}
							{:else}Never used{/if}
						</span>
						{#if !connection.revoked_at}
							<Button size="sm" variant="ghost" onclick={() => connections.revoke(connection.id)}>Revoke</Button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</section>
