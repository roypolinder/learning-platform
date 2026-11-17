import { t, type MessageKey } from '#lib/i18n/index.svelte.ts';

export function stepTypeLabel(type: string): string {
	const key = `step.${type}` as MessageKey;
	const label = t(key);
	return label === key ? type.replaceAll('_', ' ') : label;
}
