<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
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
				<p class="text-[13px] font-medium">{t('mcp.title')}</p>
				<p class="font-mono text-[11px] text-fg-subtle">{connections.endpoint || '…'}</p>
			</div>
		</div>
		{#if connections.connected}
			<span class="flex items-center gap-1.5 text-[12px] text-success">
				<span class="size-1.5 rounded-full bg-success"></span>
				{t('mcp.connected')}{connections.connected.last_seen_at ? ` · ${formatRelative(connections.connected.last_seen_at)}` : ''}
			</span>
		{:else if connections.active.length}
			<span class="flex items-center gap-1.5 text-[12px] text-fg-muted">
				<span class="size-1.5 rounded-full bg-warning"></span> {t('mcp.waiting')}
			</span>
		{:else if !connections.loading}
			<span class="text-[12px] text-fg-subtle">{t('mcp.not_connected')}</span>
		{/if}
	</header>

	<div class="flex flex-col gap-3 px-4 py-3">
		{#if connections.error}<Notice tone="danger">{connections.error}</Notice>{/if}

		{#if connections.config}
			<CodeBlock
				filename=".mcp.json"
				language="json"
				code={JSON.stringify(connections.config, null, 2)}
				caption={t('mcp.config_caption')}
			/>
		{/if}
		<ol class="flex list-decimal flex-col gap-1 pl-5 text-[13px] text-fg-muted">
			<li>{t('mcp.step_1')}</li>
			<li>{t('mcp.step_2')}</li>
			<li>{t('mcp.step_3')}</li>
		</ol>

		{#if !compact && connections.connections.length}
			<ul class="divide-y divide-line rounded-md border border-line">
				{#each connections.connections as connection (connection.id)}
					<li class="flex items-center gap-3 px-3 py-2 text-[13px]">
						<span class="size-1.5 rounded-full {connection.status === 'connected' ? 'bg-success' : connection.status === 'revoked' ? 'bg-line-strong' : 'bg-warning'}"></span>
						<span class="font-medium">{connection.label}</span>
						{#if connection.auth_method === 'oauth'}
							<span class="text-[11px] text-fg-subtle">{t('mcp.signed_in')}</span>
						{:else}
							<code class="font-mono text-[11px] text-fg-subtle">{connection.token_hint}</code>
						{/if}
						<span class="ml-auto text-[12px] text-fg-muted">
							{#if connection.revoked_at}{t('mcp.revoked')}
							{:else if connection.last_seen_at}{connection.client_name ?? t('mcp.agent')} · {formatRelative(connection.last_seen_at)}
							{:else}{t('mcp.never_used')}{/if}
						</span>
						{#if !connection.revoked_at}
							<Button size="sm" variant="ghost" onclick={() => connections.revoke(connection.id)}>{t('mcp.revoke')}</Button>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</section>
