import type { Handle } from '@sveltejs/kit/hooks';

/** Only runs while prerendering the landing pages: sets `<html lang>` per language. */
export const handle: Handle = ({ event, resolve }) => {
	const lang = event.url.pathname.startsWith('/nl') ? 'nl' : 'en';
	return resolve(event, { transformPageChunk: ({ html }) => html.replace('<html lang="en">', `<html lang="${lang}">`) });
};
