import { t } from '#lib/i18n/index.svelte.ts';
import { ApiError } from '#lib/services/api/client.ts';

/** Translated message for errors of the account endpoints. */
export function accountErrorText(error: unknown): string {
	if (!(error instanceof ApiError)) return t('common.error');
	switch (error.code) {
		case 'invalid_password':
			return t('settings.wrong_password');
		case 'email_taken':
			return t('auth.email_taken');
		case 'last_super_admin':
			return t('settings.last_admin');
	}
	const fields = error.fieldErrors();
	if (fields.new_password || fields.password) return t('auth.password_hint');
	return Object.values(fields)[0] ?? error.message;
}
