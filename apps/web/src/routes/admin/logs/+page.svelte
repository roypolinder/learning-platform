<script lang="ts">
	import { untrack } from 'svelte';
	import PageHeader from '#lib/components/layout/PageHeader.svelte';
	import Badge from '#lib/components/ui/Badge.svelte';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import Skeleton from '#lib/components/ui/Skeleton.svelte';
	import AdminGate from '#lib/features/admin/AdminGate.svelte';
	import { adminApi } from '#lib/services/api/admin.ts';
	import { ApiError } from '#lib/services/api/client.ts';
	import { auth } from '#lib/state/auth.svelte.ts';
	import type { LogEntry, LogLevel, LogsResponse } from '#lib/types/api.ts';

	const PER_PAGE = 100;
	const levels: { value: '' | LogLevel; label: string }[] = [
		{ value: '', label: 'All' },
		{ value: 'error', label: 'Errors' },
		{ value: 'warn', label: 'Warnings' },
		{ value: 'info', label: 'Events' }
	];
	const categories = ['', 'mail', 'account', 'server'];

	let data = $state<LogsResponse | null>(null);
	let error = $state<string | null>(null);
	let loading = $state(false);
	let level = $state<'' | LogLevel>('');
	let category = $state('');
	let query = $state('');
	let page = $state(1);
	let expanded = $state<string | null>(null);
	let testing = $state(false);
	let testResult = $state<{ tone: 'success' | 'danger'; text: string } | null>(null);

	async function load() {
		loading = true;
		error = null;
		try {
			data = await adminApi.logs({ level: level || undefined, category: category || undefined, q: query.trim() || undefined, page, per_page: PER_PAGE });
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'Could not load the log.';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (!auth.isSuperAdmin) return;
		void [level, category, page];
		untrack(() => void load());
	});

	function search(event: SubmitEvent) {
		event.preventDefault();
		page = 1;
		void load();
	}

	async function sendTest() {
		testing = true;
		testResult = null;
		try {
			const result = await adminApi.sendTestEmail();
			testResult = { tone: 'success', text: `Sent to ${result.to}. The mail server replied: ${result.response}` };
		} catch (e) {
			testResult = { tone: 'danger', text: e instanceof ApiError ? e.message : 'The test email could not be sent.' };
		} finally {
			testing = false;
			void load();
		}
	}

	const pages = $derived(data ? Math.max(1, Math.ceil(data.logs.total / PER_PAGE)) : 1);
	const tone = (entry: LogEntry) => (entry.level === 'error' ? 'danger' : entry.level === 'warn' ? 'warning' : 'neutral');
	const time = (iso: string) =>
		new Date(iso).toLocaleString('en-GB', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit', second: '2-digit' });
	/** The most telling fields, shown inline; everything is in the expanded view. */
	function summary(entry: LogEntry): string {
		return ['to', 'email', 'response', 'error', 'subject', 'user_id']
			.filter((key) => entry.fields[key] !== undefined)
			.slice(0, 3)
			.map((key) => `${key}: ${String(entry.fields[key])}`)
			.join(' · ');
	}
</script>

<svelte:head><title>Logs · Learning Platform</title></svelte:head>

<AdminGate>
	<div class="mx-auto max-w-6xl px-6 py-8">
		<PageHeader
			title="Logs"
			description={`Errors, warnings and events such as sent emails and account changes. Kept for ${data?.retention_days ?? 30} days.`}
		>
			{#snippet actions()}
				<Button onclick={sendTest} loading={testing}><Icon name="mail" size={14} /> Send test email</Button>
				<Button onclick={load} {loading}><Icon name="refresh" size={14} /> Reload</Button>
			{/snippet}
		</PageHeader>

		{#if testResult}<div class="mt-4"><Notice tone={testResult.tone}>{testResult.text}</Notice></div>{/if}

		{#if data}
			<div class="mt-6 flex flex-wrap gap-3 text-[13px]">
				<span class="rounded-md border border-line bg-surface px-3 py-1.5"
					><span class="font-semibold tabular-nums {data.counts.errors_24h ? 'text-danger' : ''}">{data.counts.errors_24h}</span> errors in the last 24 hours</span
				>
				<span class="rounded-md border border-line bg-surface px-3 py-1.5"
					><span class="font-semibold tabular-nums {data.counts.warnings_24h ? 'text-warning' : ''}">{data.counts.warnings_24h}</span> warnings</span
				>
			</div>
		{/if}

		<div class="mt-4 flex flex-wrap items-center gap-2">
			<div class="flex gap-1" role="radiogroup" aria-label="Level">
				{#each levels as option (option.value)}
					<button
						type="button"
						role="radio"
						aria-checked={level === option.value}
						onclick={() => ((level = option.value), (page = 1))}
						class="rounded-md border px-2.5 py-1 text-[12px] {level === option.value
							? 'border-accent bg-accent-soft font-medium text-fg'
							: 'border-line-strong bg-surface text-fg-muted hover:text-fg'}">{option.label}</button
					>
				{/each}
			</div>
			<select bind:value={category} onchange={() => (page = 1)} aria-label="Category" class="h-8 rounded-md border border-line-strong bg-surface px-2 text-[13px]">
				{#each categories as option (option)}<option value={option}>{option || 'All categories'}</option>{/each}
			</select>
			<form class="flex flex-1 gap-2" onsubmit={search}>
				<input
					type="search"
					bind:value={query}
					placeholder="Search, e.g. an email address or 550"
					aria-label="Search the log"
					class="h-8 w-full max-w-xs rounded-md border border-line-strong bg-surface px-3 text-[13px] focus:border-accent focus:outline-none"
				/>
				<Button type="submit" size="sm"><Icon name="search" size={13} /> Search</Button>
			</form>
		</div>

		<div class="mt-3">
			{#if error}
				<Notice tone="danger">{error}</Notice>
			{:else if !data}
				<Skeleton class="h-64" />
			{:else}
				<ul class="divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface">
					{#each data.logs.items as entry (entry.id)}
						<li>
							<button
								type="button"
								class="flex w-full items-start gap-3 px-3 py-2 text-left text-[13px] hover:bg-surface-hover"
								aria-expanded={expanded === entry.id}
								onclick={() => (expanded = expanded === entry.id ? null : entry.id)}
							>
								<span class="w-32 shrink-0 font-mono text-[12px] text-fg-subtle">{time(entry.created_at)}</span>
								<span class="w-16 shrink-0"><Badge tone={tone(entry)}>{entry.level}</Badge></span>
								<span class="w-20 shrink-0 text-[12px] text-fg-muted">{entry.category ?? ''}</span>
								<span class="min-w-0 flex-1">
									<span class="block">{entry.message || entry.target}</span>
									{#if summary(entry)}<span class="block truncate font-mono text-[12px] text-fg-subtle">{summary(entry)}</span>{/if}
								</span>
								<Icon name={expanded === entry.id ? 'chevron-down' : 'chevron-right'} class="mt-0.5 text-fg-subtle" />
							</button>
							{#if expanded === entry.id}
								<pre class="overflow-x-auto border-t border-line bg-code px-4 py-3 font-mono text-[12px] whitespace-pre-wrap text-fg">{JSON.stringify(
										{ time: entry.created_at, level: entry.level, target: entry.target, category: entry.category, message: entry.message, ...entry.fields },
										null,
										2
									)}</pre>
							{/if}
						</li>
					{:else}
						<li class="px-3 py-8 text-center text-[13px] text-fg-muted">Nothing logged{level || category || query ? ' that matches' : ' yet'}.</li>
					{/each}
				</ul>
				<div class="mt-3 flex items-center justify-between text-[12px] text-fg-muted">
					<span>{data.logs.total} {data.logs.total === 1 ? 'entry' : 'entries'}</span>
					{#if pages > 1}
						<div class="flex items-center gap-2">
							<Button size="sm" variant="ghost" disabled={page <= 1} onclick={() => (page -= 1)}><Icon name="arrow-left" size={13} /></Button>
							<span>Page {page} of {pages}</span>
							<Button size="sm" variant="ghost" disabled={page >= pages} onclick={() => (page += 1)}><Icon name="arrow-right" size={13} /></Button>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</AdminGate>
