/**
 * Reads the token of an emailed link (`/page#token=…`) and removes it from the address bar.
 * The fragment is never sent to the server, so the token stays out of access logs.
 */
export function takeLinkToken(): string | null {
	if (typeof window === 'undefined') return null;
	const token = new URLSearchParams(window.location.hash.slice(1)).get('token');
	if (token) history.replaceState(history.state, '', window.location.pathname);
	return token;
}
