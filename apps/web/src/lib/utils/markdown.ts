import { Marked } from 'marked';

const escapeHtml = (value: string) =>
	value.replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);

// Course content is authored in the repository, but raw HTML is still escaped so
// a course file can never inject markup or scripts into the app.
const marked = new Marked({
	gfm: true,
	renderer: {
		html: ({ text }) => escapeHtml(text),
		link: ({ href, text }) => {
			const safe = /^(https?:|mailto:|\/|#)/i.test(href) ? href : '#';
			const external = /^https?:/i.test(safe);
			return `<a href="${escapeHtml(safe)}"${external ? ' target="_blank" rel="noopener noreferrer"' : ''}>${text}</a>`;
		}
	}
});

export function renderMarkdown(source: string): string {
	return marked.parse(source, { async: false });
}

export function renderInlineMarkdown(source: string): string {
	return marked.parseInline(source, { async: false });
}
