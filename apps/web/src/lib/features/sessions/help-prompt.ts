/** Builds the coaching prompt learners paste into their own agent. */
export function helpPrompt(input: {
	courseTitle: string;
	stepTitle: string;
	stepId: string;
	runsInBrowser?: boolean;
	courseHelp: string | null;
	stepHelp: string | null;
}): string {
	const notes = [input.stepHelp, input.courseHelp]
		.filter((note): note is string => Boolean(note?.trim()))
		// Notes are hard-wrapped in YAML; reflow them so they read well when pasted.
		.map((note) => note.trim().replace(/([^\n])\n(?=[^\n\-*\d])/g, '$1 '))
		.join('\n\n');
	return [
		`Be my coach for the exercise "${input.stepTitle}" (step \`${input.stepId}\`) in the course "${input.courseTitle}".`,
		'If the learning-platform MCP server is connected, call `get_current_step` first to read the exercise.',
		'',
		'How to help:',
		'- Do not do the exercise for me: no complete solutions, no full files, no quiz answers.',
		'- Start by asking what I have tried and where I am stuck.',
		'- Explain the concept I need, give one hint at a time, and suggest the next small step.',
		'- Short snippets (a few lines) to illustrate an idea are fine; I write the real code.',
		'- When I share code or an error, point me to what to look at and let me fix it.',
		'- If I ask for the answer, give a stronger hint instead and remind me that the goal is to learn.',
		...(input.runsInBrowser
			? [
					'',
					'The platform checks my service from my browser, so it must allow CORS from the platform origin (including OPTIONS preflight). If every check says the browser got no response, check that first.'
				]
			: []),
		...(notes ? ['', 'Coaching notes for this exercise:', notes] : [])
	].join('\n');
}
