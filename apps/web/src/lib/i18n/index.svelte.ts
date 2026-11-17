/**
 * Interface translations. Messages are flat, typed keys: `nl.ts` must translate every key
 * in `en.ts`, so a missing translation fails `svelte-check`. Technical terms that developers
 * use in English (embedding, prompt, endpoint, MCP…) stay English in Dutch too.
 */
import { en, type MessageKey } from './en.ts';
import { nl } from './nl.ts';

export type Locale = 'en' | 'nl';
export const locales: { code: Locale; label: string; flag: string }[] = [
	{ code: 'en', label: 'English', flag: '🇬🇧' },
	{ code: 'nl', label: 'Nederlands', flag: '🇳🇱' }
];

const messages: Record<Locale, Record<MessageKey, string>> = { en, nl };
const STORAGE_KEY = 'lp-locale';

export function isLocale(value: unknown): value is Locale {
	return value === 'en' || value === 'nl';
}

/** The visitor's explicit choice, else the browser language. Never IP-based. */
function initialLocale(): Locale {
	return typeof window === 'undefined' ? 'en' : preferredLocale();
}

export function browserLocale(): Locale {
	if (typeof navigator === 'undefined') return 'en';
	const preferred = navigator.languages?.length ? navigator.languages : [navigator.language];
	for (const tag of preferred) {
		const primary = tag?.toLowerCase().split('-')[0];
		if (isLocale(primary)) return primary;
	}
	return 'en';
}

/** The visitor's explicit choice if they made one, otherwise the browser language. */
export function preferredLocale(): Locale {
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (isLocale(stored)) return stored;
	} catch {
		// Fall through to the browser language.
	}
	return browserLocale();
}

/** Text in a fixed language, e.g. for pages whose URL decides the language. */
export function translate(locale: Locale, key: MessageKey, params?: Record<string, string | number>): string {
	const template = messages[locale][key] ?? en[key] ?? key;
	if (!params) return template;
	return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}

class I18n {
	locale = $state<Locale>(initialLocale());

	/** Switches the interface language and remembers it in this browser. */
	set(locale: Locale): void {
		this.locale = locale;
		if (typeof document !== 'undefined') document.documentElement.lang = locale;
		try {
			localStorage.setItem(STORAGE_KEY, locale);
		} catch {
			// Not persisted; the choice still applies to this page view.
		}
	}

	t = (key: MessageKey, params?: Record<string, string | number>): string => translate(this.locale, key, params);
}

export const i18n = new I18n();
export const t = i18n.t;
export type { MessageKey };
