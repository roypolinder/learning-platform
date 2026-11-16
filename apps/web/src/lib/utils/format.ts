import type { Difficulty, StepStatus } from '#lib/types/api.ts';

export function formatMinutes(minutes: number): string {
	if (minutes < 60) return `${minutes} min`;
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	return rest ? `${hours} h ${rest} min` : `${hours} h`;
}

const difficultyLabels: Record<Difficulty, string> = {
	beginner: 'Beginner',
	intermediate: 'Intermediate',
	advanced: 'Advanced'
};

export const formatDifficulty = (difficulty: Difficulty) => difficultyLabels[difficulty];

const statusLabels: Record<StepStatus, string> = {
	locked: 'Locked',
	available: 'Not started',
	in_progress: 'In progress',
	completed: 'Completed',
	failed: 'Needs another try',
	skipped: 'Skipped'
};

export const formatStepStatus = (status: StepStatus) => statusLabels[status];

const relative = new Intl.RelativeTimeFormat('en', { numeric: 'auto' });

export function formatRelative(iso: string, now = Date.now()): string {
	const seconds = Math.round((new Date(iso).getTime() - now) / 1000);
	const units: [Intl.RelativeTimeFormatUnit, number][] = [
		['day', 86400],
		['hour', 3600],
		['minute', 60]
	];
	for (const [unit, size] of units) {
		if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit);
	}
	return 'just now';
}

export function formatDate(iso: string): string {
	return new Date(iso).toLocaleDateString('en', { day: 'numeric', month: 'short', year: 'numeric' });
}
