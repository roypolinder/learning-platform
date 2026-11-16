import type { ApiErrorBody } from '#lib/types/api.ts';

export class ApiError extends Error {
	constructor(
		readonly status: number,
		readonly code: string,
		message: string,
		readonly details?: unknown
	) {
		super(message);
	}

	get isUnauthorized(): boolean {
		return this.status === 401;
	}

	/** Field-level validation messages keyed by field name. */
	fieldErrors(): Record<string, string> {
		if (!Array.isArray(this.details)) return {};
		const entries = this.details.flatMap((item: unknown) => {
			if (typeof item === 'object' && item && 'field' in item && 'message' in item) {
				return [[String(item.field), String(item.message)] as const];
			}
			return [];
		});
		return Object.fromEntries(entries);
	}
}

type Method = 'GET' | 'POST' | 'PATCH' | 'DELETE';

async function request<T>(method: Method, path: string, body?: unknown): Promise<T> {
	const headers: Record<string, string> = { accept: 'application/json' };
	if (method !== 'GET') headers['x-requested-with'] = 'lp-web';
	if (body !== undefined) headers['content-type'] = 'application/json';

	let response: Response;
	try {
		response = await fetch(`/api/v1${path}`, {
			method,
			headers,
			credentials: 'same-origin',
			body: body === undefined ? undefined : JSON.stringify(body)
		});
	} catch {
		throw new ApiError(0, 'network_error', 'Cannot reach the platform. Is the API running?');
	}

	if (response.status === 204) return undefined as T;
	const payload: unknown = await response.json().catch(() => null);
	if (!response.ok) {
		const error = (payload as ApiErrorBody | null)?.error;
		throw new ApiError(
			response.status,
			error?.code ?? 'unknown',
			error?.message ?? `Request failed (${response.status})`,
			error?.details
		);
	}
	return payload as T;
}

export const api = {
	get: <T>(path: string) => request<T>('GET', path),
	post: <T>(path: string, body: unknown = {}) => request<T>('POST', path, body),
	patch: <T>(path: string, body: unknown) => request<T>('PATCH', path, body),
	delete: <T>(path: string) => request<T>('DELETE', path)
};
