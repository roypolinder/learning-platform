/** Narrowing helpers for step configs, which arrive as untyped JSON. */

export const asString = (value: unknown): string | undefined =>
	typeof value === 'string' ? value : undefined;

export const asArray = (value: unknown): unknown[] => (Array.isArray(value) ? value : []);

export const asRecord = (value: unknown): Record<string, unknown> =>
	typeof value === 'object' && value !== null && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
