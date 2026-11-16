/**
 * Runs a step's `client_task` against the learner's own service from the browser, so
 * services on localhost work without being exposed. Only raw responses are reported; the
 * platform decides whether they are correct.
 */

export interface HttpRequestSpec {
	method?: string;
	path: string;
	query?: Record<string, string>;
	json?: unknown;
}

export interface HttpClientTask {
	kind: 'http';
	default_url?: string | null;
	timeout_ms: number;
	parallel: boolean;
	cases: { id: string; label: string; request: HttpRequestSpec }[];
}

export interface McpClientTask {
	kind: 'mcp';
	default_url?: string | null;
	path: string;
	timeout_ms: number;
	calls: { id: string; label: string; tool: string; arguments: unknown }[];
}

export type ClientTask = HttpClientTask | McpClientTask;

interface Reply {
	status?: number;
	latency_ms?: number;
	body?: string;
	error?: string;
	headers?: Headers;
}

const MAX_BODY_CHARS = 128 * 1024;
const PARALLEL_REQUESTS = 4;
const NONCE_TOKEN = '{{nonce}}';
const ASSET = /^\{\{asset:([A-Za-z0-9._-]+)\}\}$/;
const PROTOCOL_VERSION = '2025-06-18';

export function asClientTask(value: unknown): ClientTask | null {
	if (typeof value !== 'object' || value === null) return null;
	const kind = (value as { kind?: unknown }).kind;
	return kind === 'http' || kind === 'mcp' ? (value as ClientTask) : null;
}

export function newNonce(): string {
	return crypto.randomUUID().replaceAll('-', '').slice(0, 12);
}

function substitute(value: unknown, nonce: string): unknown {
	if (typeof value === 'string') return value.replaceAll(NONCE_TOKEN, nonce);
	if (Array.isArray(value)) return value.map((item) => substitute(item, nonce));
	if (value && typeof value === 'object') {
		return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, substitute(item, nonce)]));
	}
	return value;
}

async function dataUrl(name: string, cache: Map<string, Promise<string>>): Promise<string> {
	let pending = cache.get(name);
	if (!pending) {
		pending = fetch(`/api/v1/course-assets/${encodeURIComponent(name)}`)
			.then((response) => {
				if (!response.ok) throw new Error(`asset ${name} is unavailable`);
				return response.blob();
			})
			.then(
				(blob) =>
					new Promise<string>((resolve, reject) => {
						const reader = new FileReader();
						reader.onload = () => resolve(String(reader.result));
						reader.onerror = () => reject(reader.error);
						reader.readAsDataURL(blob);
					})
			);
		cache.set(name, pending);
	}
	return pending;
}

async function withAssets(value: unknown, cache: Map<string, Promise<string>>): Promise<unknown> {
	if (typeof value === 'string') {
		const match = ASSET.exec(value);
		return match ? dataUrl(match[1], cache) : value;
	}
	if (Array.isArray(value)) return Promise.all(value.map((item) => withAssets(item, cache)));
	if (value && typeof value === 'object') {
		const entries = await Promise.all(
			Object.entries(value).map(async ([key, item]) => [key, await withAssets(item, cache)] as const)
		);
		return Object.fromEntries(entries);
	}
	return value;
}

function target(base: string, path: string): URL {
	return path ? new URL(base.replace(/\/+$/, '') + path) : new URL(base);
}

async function send(url: URL, init: RequestInit, timeoutMs: number): Promise<Reply> {
	const controller = new AbortController();
	const timer = setTimeout(() => controller.abort(), timeoutMs);
	const started = performance.now();
	try {
		const response = await fetch(url, { ...init, signal: controller.signal, credentials: 'omit' });
		const body = (await response.text()).slice(0, MAX_BODY_CHARS);
		return {
			status: response.status,
			latency_ms: Math.round(performance.now() - started),
			body,
			headers: response.headers
		};
	} catch (error) {
		return { error: error instanceof DOMException && error.name === 'AbortError' ? 'timeout' : 'network' };
	} finally {
		clearTimeout(timer);
	}
}

async function pool<T>(items: T[], limit: number, work: (item: T) => Promise<void>): Promise<void> {
	let next = 0;
	const workers = Array.from({ length: Math.min(limit, items.length) }, async () => {
		while (next < items.length) await work(items[next++]);
	});
	await Promise.all(workers);
}

export async function runHttpTask(
	task: HttpClientTask,
	baseUrl: string,
	onProgress: (done: number, total: number) => void = () => {}
): Promise<Record<string, unknown>> {
	const nonce = newNonce();
	const assets = new Map<string, Promise<string>>();
	const results: (Reply & { id: string })[] = [];
	let done = 0;

	const runCase = async (item: HttpClientTask['cases'][number]) => {
		const { request } = item;
		const url = target(baseUrl, String(substitute(request.path, nonce)));
		for (const [key, value] of Object.entries(request.query ?? {})) {
			url.searchParams.append(key, String(substitute(value, nonce)));
		}
		const headers: Record<string, string> = { accept: 'application/json' };
		let body: string | undefined;
		if (request.json !== undefined) {
			headers['content-type'] = 'application/json';
			body = JSON.stringify(await withAssets(substitute(request.json, nonce), assets));
		}
		const reply = await send(url, { method: (request.method ?? 'GET').toUpperCase(), headers, body }, task.timeout_ms);
		delete reply.headers;
		results.push({ id: item.id, ...reply });
		onProgress(++done, task.cases.length);
	};

	onProgress(0, task.cases.length);
	if (task.parallel) {
		await pool(task.cases, PARALLEL_REQUESTS, runCase);
	} else {
		for (const item of task.cases) await runCase(item);
	}
	return { url: baseUrl, nonce, results };
}

/** Minimal Streamable HTTP MCP client: JSON or SSE responses and the session header. */
class McpSession {
	private session: string | null = null;
	private protocol: string | null = null;
	private nextId = 0;

	constructor(
		private readonly endpoint: URL,
		private readonly timeoutMs: number
	) {}

	private headers(): Record<string, string> {
		const headers: Record<string, string> = {
			accept: 'application/json, text/event-stream',
			'content-type': 'application/json'
		};
		if (this.session) headers['mcp-session-id'] = this.session;
		if (this.protocol) headers['mcp-protocol-version'] = this.protocol;
		return headers;
	}

	setProtocol(version: string) {
		this.protocol = version;
	}

	/** Resolves to `{ result }`, `{ rpc_error }` or `{ error }`. */
	async request(method: string, params: unknown): Promise<{ result?: unknown; rpc_error?: unknown; error?: string }> {
		const id = ++this.nextId;
		const controller = new AbortController();
		const timer = setTimeout(() => controller.abort(), this.timeoutMs);
		try {
			const response = await fetch(this.endpoint, {
				method: 'POST',
				headers: this.headers(),
				body: JSON.stringify({ jsonrpc: '2.0', id, method, params }),
				signal: controller.signal,
				credentials: 'omit'
			});
			const session = response.headers.get('mcp-session-id');
			if (session) this.session = session;
			if (!response.ok) return { error: `\`${method}\` returned HTTP ${response.status}` };
			const contentType = response.headers.get('content-type') ?? '';
			const message = contentType.startsWith('text/event-stream')
				? await readEvent(response, id)
				: ((await response.json().catch(() => null)) as Record<string, unknown> | null);
			if (!message) return { error: `\`${method}\` did not return a JSON-RPC response` };
			if ('error' in message) return { rpc_error: message.error };
			return { result: message.result };
		} catch (error) {
			return { error: error instanceof DOMException && error.name === 'AbortError' ? 'timeout' : 'network' };
		} finally {
			clearTimeout(timer);
		}
	}

	async notify(method: string): Promise<void> {
		await fetch(this.endpoint, {
			method: 'POST',
			headers: this.headers(),
			body: JSON.stringify({ jsonrpc: '2.0', method }),
			credentials: 'omit'
		}).catch(() => undefined);
	}
}

/** Reads server-sent events until the response to `id` arrives; servers may keep streaming. */
async function readEvent(response: Response, id: number): Promise<Record<string, unknown> | null> {
	const reader = response.body?.getReader();
	if (!reader) return null;
	const decoder = new TextDecoder();
	let buffer = '';
	try {
		for (;;) {
			const { value, done } = await reader.read();
			if (done) return null;
			buffer += decoder.decode(value, { stream: true }).replaceAll('\r\n', '\n');
			let end: number;
			while ((end = buffer.indexOf('\n\n')) >= 0) {
				const data = buffer
					.slice(0, end)
					.split('\n')
					.filter((line) => line.startsWith('data:'))
					.map((line) => line.slice(5).trimStart())
					.join('\n');
				buffer = buffer.slice(end + 2);
				try {
					const message = JSON.parse(data) as Record<string, unknown>;
					if (message.id === id) return message;
				} catch {
					// Not JSON-RPC (e.g. a keep-alive comment); keep reading.
				}
			}
			if (buffer.length > MAX_BODY_CHARS) return null;
		}
	} finally {
		void reader.cancel().catch(() => undefined);
	}
}

export async function runMcpTask(
	task: McpClientTask,
	baseUrl: string,
	onProgress: (done: number, total: number) => void = () => {}
): Promise<Record<string, unknown>> {
	const nonce = newNonce();
	const base = new URL(baseUrl);
	const endpoint = base.pathname === '/' || base.pathname === '' ? target(baseUrl, task.path) : base;
	const client = new McpSession(endpoint, task.timeout_ms);
	const total = task.calls.length + 2;
	onProgress(0, total);

	const init = await client.request('initialize', {
		protocolVersion: PROTOCOL_VERSION,
		capabilities: {},
		clientInfo: { name: 'learning-platform-checker', version: '1.0.0' }
	});
	if (init.result === undefined) {
		const error = init.error ?? `initialize failed: ${JSON.stringify(init.rpc_error).slice(0, 160)}`;
		return { url: endpoint.toString(), nonce, error };
	}
	const version = (init.result as { protocolVersion?: unknown }).protocolVersion;
	if (typeof version === 'string') client.setProtocol(version);
	await client.notify('notifications/initialized');
	onProgress(1, total);

	const tools: unknown[] = [];
	let toolsError: string | undefined;
	let cursor: string | undefined;
	for (let page = 0; page < 5; page++) {
		const listed = await client.request('tools/list', cursor ? { cursor } : {});
		if (listed.result === undefined) {
			toolsError = listed.error ?? `tools/list failed: ${JSON.stringify(listed.rpc_error).slice(0, 160)}`;
			break;
		}
		const result = listed.result as { tools?: unknown[]; nextCursor?: unknown };
		tools.push(...(result.tools ?? []));
		if (typeof result.nextCursor !== 'string') break;
		cursor = result.nextCursor;
	}
	onProgress(2, total);

	const calls: Record<string, unknown>[] = [];
	for (const call of task.calls) {
		const reply = await client.request('tools/call', { name: call.tool, arguments: substitute(call.arguments, nonce) });
		calls.push({ id: call.id, ...reply });
		onProgress(calls.length + 2, total);
	}
	return {
		url: endpoint.toString(),
		nonce,
		initialize: init.result,
		...(toolsError ? { tools_error: toolsError } : { tools }),
		calls
	};
}
