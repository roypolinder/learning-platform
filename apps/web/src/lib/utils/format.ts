import { i18n, t } from '#lib/i18n/index.svelte.ts';
import type { Difficulty, StepStatus } from '#lib/types/api.ts';

export function formatMinutes(minutes: number): string {
	if (minutes < 60) return t('time.minutes', { minutes });
	const hours = Math.floor(minutes / 60);
	const rest = minutes % 60;
	return rest ? t('time.hours_minutes', { hours, minutes: rest }) : t('time.hours', { hours });
}

export const formatDifficulty = (difficulty: Difficulty) => t(`difficulty.${difficulty}`);

export const formatStepStatus = (status: StepStatus) => t(`status.${status}`);

export function formatRelative(iso: string, now = Date.now()): string {
	const relative = new Intl.RelativeTimeFormat(i18n.locale, { numeric: 'auto' });
	const seconds = Math.round((new Date(iso).getTime() - now) / 1000);
	const units: [Intl.RelativeTimeFormatUnit, number][] = [
		['day', 86400],
		['hour', 3600],
		['minute', 60]
	];
	for (const [unit, size] of units) {
		if (Math.abs(seconds) >= size) return relative.format(Math.round(seconds / size), unit);
	}
	return t('time.just_now');
}

export function formatDate(iso: string): string {
	return new Date(iso).toLocaleDateString(i18n.locale, { day: 'numeric', month: 'short', year: 'numeric' });
}
