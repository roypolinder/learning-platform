<script lang="ts">
	import Button from '#lib/components/ui/Button.svelte';
	import CodeBlock from '#lib/components/ui/CodeBlock.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';
	import Notice from '#lib/components/ui/Notice.svelte';
	import TextField from '#lib/components/ui/TextField.svelte';
	import type { CourseStep, StepProgress } from '#lib/types/api.ts';
	import type { LearningSession } from '../learning-session.svelte.ts';
	import OutcomePanel from '../OutcomePanel.svelte';
	import { asClientTask, runHttpTask, runMcpTask } from '../service-runner.ts';

	let { step, progress, session }: { step: CourseStep; progress: StepProgress | null; session: LearningSession } = $props();

	const task = $derived(asClientTask(step.client_task));
	const isMcp = $derived(task?.kind === 'mcp');
	const origin = typeof window === 'undefined' ? '' : window.location.origin;
	const completed = $derived(progress?.status === 'completed');

	let url = $state('');
	$effect.pre(() => {
		if (!url) url = task?.default_url ?? 'http://localhost:3000';
	});
	let running = $state(false);
	let done = $state(0);
	let total = $state(0);
	let runError = $state<string | null>(null);

	const corsHeaders = $derived(
		[
			`Access-Control-Allow-Origin: ${origin}`,
			'Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS',
			`Access-Control-Allow-Headers: content-type${isMcp ? ', mcp-session-id, mcp-protocol-version' : ''}`,
			...(isMcp ? ['Access-Control-Expose-Headers: mcp-session-id'] : [])
		].join('\n')
	);

	async function run(event: SubmitEvent) {
		event.preventDefault();
		if (!task) return;
		runError = null;
		try {
			new URL(url);
		} catch {
			runError = 'Enter a full URL, for example http://localhost:3000.';
			return;
		}
		running = true;
		try {
			const report = (current: number, count: number) => {
				done = current;
				total = count;
			};
			const payload = task.kind === 'mcp' ? await runMcpTask(task, url, report) : await runHttpTask(task, url, report);
			await session.submit(step.id, payload);
		} catch (caught) {
			runError = caught instanceof Error ? caught.message : 'The checks could not run.';
		} finally {
			running = false;
		}
	}
</script>

<div class="flex flex-col gap-4">
	{#if !task}
		<Notice tone="danger">This step has no checks to run.</Notice>
	{:else if !completed}
		<form class="flex items-end gap-2" onsubmit={run}>
			<div class="flex-1">
				<TextField
					label={isMcp ? 'MCP server URL' : 'Service base URL'}
					type="url"
					bind:value={url}
					hint={isMcp
						? `Your browser connects to ${task.kind === 'mcp' ? task.path : '/mcp'} on this host, so localhost works.`
						: 'Your browser sends the test requests, so localhost works. Nothing needs to be exposed.'}
				/>
			</div>
			<Button type="submit" variant="primary" loading={running || session.pending} class="mb-[22px] h-9">Run checks</Button>
		</form>

		{#if running}
			<p class="flex items-center gap-2 text-[12px] text-fg-muted">
				<Icon name="clock" size={13} />
				Sent {done} of {total} requests. Services that call a model can take a while.
			</p>
		{/if}
		{#if runError}<Notice tone="danger">{runError}</Notice>{/if}

		<details class="rounded-lg border border-line bg-surface px-4 py-3 text-[13px]">
			<summary class="cursor-pointer font-medium">Let your browser reach the service (CORS)</summary>
			<div class="mt-2 flex flex-col gap-2 text-fg-muted">
				<p>
					The requests come from this page, so your service must allow cross-origin requests from
					<code class="font-mono text-[12px] text-fg">{origin}</code>, including the <code class="font-mono text-[12px]">OPTIONS</code>
					preflight. Most frameworks have a CORS middleware for this. The responses need these headers:
				</p>
				<CodeBlock code={corsHeaders} filename="Response headers" />
				{#if isMcp}
					<p>If your MCP SDK validates the <code class="font-mono text-[12px]">Origin</code> header, add this origin to its allowed origins.</p>
				{/if}
			</div>
		</details>

		{#if task.kind === 'http' && task.cases.length > 1}
			<details class="text-[13px] text-fg-muted">
				<summary class="cursor-pointer">What will be checked ({task.cases.length} checks)</summary>
				<ul class="mt-2 list-disc pl-5">
					{#each task.cases as item (item.id)}<li>{item.label}</li>{/each}
				</ul>
			</details>
		{/if}
	{/if}

	{#if progress?.outcome}<OutcomePanel outcome={progress.outcome} />{/if}
	{#if completed}
		<div class="flex justify-end">
			<Button onclick={() => session.advance(step.id)}>Next step <Icon name="arrow-right" size={14} /></Button>
		</div>
	{/if}
</div>
