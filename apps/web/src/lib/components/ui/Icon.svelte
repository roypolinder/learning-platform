<script lang="ts" module>
	// Stroke icons drawn on a 16px grid; kept inline to avoid an icon dependency.
	const paths = {
		check: 'M3.5 8.5l3 3 6-7',
		lock: 'M4.5 7.5h7v6h-7zM6 7.5V5.5a2 2 0 014 0v2',
		circle: 'M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11z',
		'circle-half': 'M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM8 2.5v11',
		'arrow-right': 'M3 8h10M9 4l4 4-4 4',
		'arrow-left': 'M13 8H3M7 4L3 8l4 4',
		clock: 'M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM8 5v3l2 1.5',
		search: 'M7 2.5a4.5 4.5 0 100 9 4.5 4.5 0 000-9zM10.5 10.5l3 3',
		home: 'M2.5 7L8 2.5 13.5 7v6.5h-11zM6.5 13.5v-4h3v4',
		book: 'M3 3h4a1.5 1.5 0 011.5 1.5V13A1.5 1.5 0 007 11.5H3zM13 3H9.5A1.5 1.5 0 008 4.5V13a1.5 1.5 0 011.5-1.5H13z',
		settings: 'M8 6a2 2 0 100 4 2 2 0 000-4zM8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4',
		plug: 'M6 2v3M10 2v3M4.5 5h7v2.5a3.5 3.5 0 01-7 0zM8 11v3',
		'log-out': 'M6 13.5H3.5v-11H6M10 5l3 3-3 3M13 8H6',
		x: 'M4 4l8 8M12 4l-8 8',
		copy: 'M5.5 5.5h7v8h-7zM3.5 10.5v-8h7',
		refresh: 'M13 4v3h-3M3 12V9h3M12.5 7A4.5 4.5 0 004.2 5M3.5 9a4.5 4.5 0 008.3 2',
		alert: 'M8 2.5l6 11H2zM8 6.5v3M8 11.5v.5',
		terminal: 'M2.5 3h11v10h-11zM5 6.5l2 1.5-2 1.5M8.5 10h2.5',
		'chevron-right': 'M6 4l4 4-4 4',
		help: 'M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM6.3 6.4a1.8 1.8 0 013.4.6c0 1.2-1.7 1.4-1.7 2.5M8 11.2v.3',
		'chevron-down': 'M4 6l4 4 4-4',
		minus: 'M4 8h8',
		play: 'M5 3.5v9l7-4.5z',
		edit: 'M10.5 2.5l3 3L6 13H3v-3z',
		plus: 'M8 3v10M3 8h10',
		chart: 'M2.5 13.5h11M4.5 13.5V8.5M8 13.5v-9M11.5 13.5V6.5',
		'external': 'M9 3h4v4M13 3L7.5 8.5M11 9.5V13H3V5h3.5',
		globe: 'M8 2.5a5.5 5.5 0 100 11 5.5 5.5 0 000-11zM2.5 8h11M8 2.5c1.6 1.6 2.3 3.5 2.3 5.5S9.6 11.9 8 13.5C6.4 11.9 5.7 10 5.7 8S6.4 4.1 8 2.5z',
		notes: 'M4 2.5h5.5L12 5v8.5H4zM6 7.5h4M6 10h4',
		sidebar: 'M2.5 3h11v10h-11zM10 3v10',
		grip: 'M6 4.5h.01M10 4.5h.01M6 8h.01M10 8h.01M6 11.5h.01M10 11.5h.01',
		trash: 'M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.5 9h6l.5-9',
		bike: 'M2 11a2 2 0 104 0 2 2 0 10-4 0zM10 11a2 2 0 104 0 2 2 0 10-4 0zM4 11l2.5-5h4L12 11M6.5 6L8 11M9.5 4h1.5',
		bread: 'M2.5 9.5c0-3 2.5-5 5.5-5s5.5 2 5.5 5v2h-11zM5.5 6.5v5M8 5v6.5M10.5 6.5v5',
		plant: 'M8 13.5V7M8 7C8 4.5 6 3 3.5 3 3.5 5.5 5.5 7 8 7zM8 9.5c0-2.5 2-4 4.5-4 0 2.5-2 4-4.5 4zM5 13.5h6',
		pan: 'M2.5 8h9v2.5A2.5 2.5 0 019 13H5a2.5 2.5 0 01-2.5-2.5zM11.5 9H14M5.5 6c0-1 1-1 1-2.5M8.5 6c0-1 1-1 1-2.5',
		house: 'M2.5 8L8 3l5.5 5M4 7v6.5h8V7M6.5 13.5v-3h3v3',
		dumbbell: 'M1.5 8h13M3.5 5.5v5M5.5 4.5v7M10.5 4.5v7M12.5 5.5v5',
		hanger: 'M8 6.5V5.8A1.5 1.5 0 106.5 4.3M8 6.5l-6 4h12z',
		paw: 'M8 9c-2 0-3.5 2-3.5 3.3 0 1 1 1.2 1.8 1 .6-.2 1.1-.3 1.7-.3s1.1.1 1.7.3c.8.2 1.8 0 1.8-1C11.5 11 10 9 8 9zM3.5 7.5a1 1 0 102 0 1 1 0 10-2 0zM10.5 7.5a1 1 0 102 0 1 1 0 10-2 0zM5.5 4.5a1 1 0 102 0 1 1 0 10-2 0zM8.5 4.5a1 1 0 102 0 1 1 0 10-2 0z',
		music: 'M6 12a1.5 1.5 0 11-3 0 1.5 1.5 0 013 0zM6 12V3.5l7-1.5V10.5M13 10.5a1.5 1.5 0 11-3 0 1.5 1.5 0 013 0z'
	} as const;

	export function isIconName(name: string | null | undefined): name is IconName {
		return Boolean(name && name in paths);
	}

	export type IconName = keyof typeof paths;
</script>

<script lang="ts">
	interface Props {
		name: IconName;
		size?: number;
		class?: string;
		strokeWidth?: number;
	}

	let { name, size = 16, class: className = '', strokeWidth = 1.5 }: Props = $props();
</script>

<svg
	width={size}
	height={size}
	viewBox="0 0 16 16"
	fill="none"
	stroke="currentColor"
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	class="shrink-0 {className}"
	aria-hidden="true"
>
	<path d={paths[name]} />
</svg>
