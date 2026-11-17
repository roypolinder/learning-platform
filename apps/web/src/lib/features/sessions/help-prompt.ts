import type { Locale } from '#lib/i18n/index.svelte.ts';

interface PromptText {
	intro: (step: string, id: string, course: string) => string;
	mcp: string;
	rules: string[];
	browser: string;
	notes: string;
}

/** The prompt is addressed to the learner's agent, so it is written as the learner speaking. */
const text: Record<Locale, PromptText> = {
	en: {
		intro: (step, id, course) => `Be my coach for the exercise "${step}" (step \`${id}\`) in the course "${course}".`,
		mcp: 'If the learning-platform MCP server is connected, call `get_project` and `get_current_step` first to read my project and the exercise.',
		rules: [
			'How to help:',
			'- Do not do the exercise for me: no complete solutions, no full files, no quiz answers.',
			'- Start by asking what I have tried and where I am stuck.',
			'- Explain the concept I need, give one hint at a time, and suggest the next small step.',
			'- Short snippets (a few lines) to illustrate an idea are fine; I write the real code.',
			'- When I share code or an error, point me to what to look at and let me fix it.',
			'- If I ask for the answer, give a stronger hint instead and remind me that the goal is to learn.'
		],
		browser:
			'The platform checks my service from my browser, so it must allow CORS from the platform origin (including OPTIONS preflight). If every check says the browser got no response, check that first.',
		notes: 'Coaching notes for this exercise:'
	},
	nl: {
		intro: (step, id, course) => `Wees mijn coach bij de opdracht "${step}" (stap \`${id}\`) in de cursus "${course}".`,
		mcp: 'Is de MCP server learning-platform gekoppeld? Roep dan eerst `get_project` en `get_current_step` aan om mijn project en de opdracht te lezen.',
		rules: [
			'Praat Nederlands met me, maar houd vaktermen Engels (embedding, prompt, endpoint, tool calling…).',
			'',
			'Zo help je me:',
			'- Maak de opdracht niet voor me: geen complete oplossingen, geen volledige bestanden, geen quizantwoorden.',
			'- Vraag eerst wat ik al geprobeerd heb en waar ik vastloop.',
			'- Leg het concept uit dat ik nodig heb, geef één hint tegelijk en stel de volgende kleine stap voor.',
			'- Korte snippets (een paar regels) om een idee te laten zien zijn prima; de echte code schrijf ik zelf.',
			'- Deel ik code of een foutmelding, wijs me dan waar ik moet kijken en laat mij het oplossen.',
			'- Vraag ik om het antwoord, geef dan een sterkere hint en herinner me eraan dat het doel leren is.'
		],
		browser:
			'Het platform test mijn service vanuit mijn browser, dus die moet CORS toestaan vanaf de origin van het platform (inclusief de OPTIONS preflight). Zegt elke check dat de browser geen response kreeg? Controleer dat dan eerst.',
		notes: 'Coaching-notities voor deze opdracht (Engels):'
	}
};

/** Builds the coaching prompt learners paste into their own agent. */
export function helpPrompt(input: {
	courseTitle: string;
	stepTitle: string;
	stepId: string;
	runsInBrowser?: boolean;
	courseHelp: string | null;
	stepHelp: string | null;
	locale: Locale;
}): string {
	const copy = text[input.locale];
	const notes = [input.stepHelp, input.courseHelp]
		.filter((note): note is string => Boolean(note?.trim()))
		// Notes are hard-wrapped in YAML; reflow them so they read well when pasted.
		.map((note) => note.trim().replace(/([^\n])\n(?=[^\n\-*\d])/g, '$1 '))
		.join('\n\n');
	return [
		copy.intro(input.stepTitle, input.stepId, input.courseTitle),
		copy.mcp,
		'',
		...copy.rules,
		...(input.runsInBrowser ? ['', copy.browser] : []),
		...(notes ? ['', copy.notes, notes] : [])
	].join('\n');
}
