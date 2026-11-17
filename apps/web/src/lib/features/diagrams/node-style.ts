import type { NodeKind } from './layout.ts';

/** Subtle per-kind styling so people, apps, models, data and tools are told apart at a glance. */
export function nodeClass(kind: NodeKind | null): string {
	switch (kind) {
		case 'person':
			return 'rounded-full border-line-strong bg-surface';
		case 'model':
			return 'rounded-lg border-accent/40 bg-accent-soft';
		case 'data':
			return 'rounded-lg border-line-strong bg-surface-muted';
		case 'tool':
			return 'rounded-lg border-dashed border-line-strong bg-surface';
		default:
			return 'rounded-lg border-line-strong bg-surface';
	}
}
