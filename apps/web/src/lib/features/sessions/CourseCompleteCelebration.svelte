<script lang="ts">
	import { t } from '#lib/i18n/index.svelte.ts';
	import Button from '#lib/components/ui/Button.svelte';
	import Icon from '#lib/components/ui/Icon.svelte';

	interface Props {
		courseTitle: string;
		stepCount: number;
		courseId: string;
		onclose: () => void;
	}

	let { courseTitle, stepCount, courseId, onclose }: Props = $props();

	let canvas = $state<HTMLCanvasElement | null>(null);
	let dialog = $state<HTMLDivElement | null>(null);

	const colors = ['#5b5bd6', '#18794e', '#f0a04b', '#f2555a', '#4cc38a', '#7b7bef', '#facc15'];

	$effect(() => {
		dialog?.focus();
		if (!canvas || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
		const context = canvas.getContext('2d');
		if (!context) return;

		const ratio = window.devicePixelRatio || 1;
		const resize = () => {
			canvas!.width = window.innerWidth * ratio;
			canvas!.height = window.innerHeight * ratio;
			context.setTransform(ratio, 0, 0, ratio, 0, 0);
		};
		resize();
		window.addEventListener('resize', resize);

		const width = window.innerWidth;
		const pieces = Array.from({ length: 180 }, (_, index) => {
			const fromLeft = index % 2 === 0;
			const angle = (fromLeft ? -60 : -120) * (Math.PI / 180) + (Math.random() - 0.5) * 0.8;
			const speed = 9 + Math.random() * 9;
			return {
				x: fromLeft ? 0 : width,
				y: window.innerHeight * 0.75,
				vx: Math.cos(angle) * speed,
				vy: Math.sin(angle) * speed,
				size: 5 + Math.random() * 6,
				rotation: Math.random() * Math.PI,
				spin: (Math.random() - 0.5) * 0.3,
				color: colors[index % colors.length],
				delay: Math.floor(Math.random() * 20)
			};
		});

		let frame = 0;
		let handle = 0;
		const draw = () => {
			frame++;
			context.clearRect(0, 0, window.innerWidth, window.innerHeight);
			let alive = false;
			for (const piece of pieces) {
				if (frame < piece.delay) {
					alive = true;
					continue;
				}
				piece.vy += 0.25;
				piece.vx *= 0.99;
				piece.x += piece.vx;
				piece.y += piece.vy;
				piece.rotation += piece.spin;
				if (piece.y > window.innerHeight + 20) continue;
				alive = true;
				context.save();
				context.translate(piece.x, piece.y);
				context.rotate(piece.rotation);
				context.fillStyle = piece.color;
				context.fillRect(-piece.size / 2, -piece.size / 4, piece.size, piece.size / 2);
				context.restore();
			}
			if (alive) handle = requestAnimationFrame(draw);
		};
		handle = requestAnimationFrame(draw);

		return () => {
			cancelAnimationFrame(handle);
			window.removeEventListener('resize', resize);
		};
	});

	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') onclose();
	}
</script>

<svelte:window {onkeydown} />

<div class="fixed inset-0 z-50 grid place-items-center bg-bg/70 p-6 backdrop-blur-sm">
	<canvas bind:this={canvas} class="pointer-events-none fixed inset-0 size-full" aria-hidden="true"></canvas>
	<div
		bind:this={dialog}
		role="dialog"
		aria-modal="true"
		aria-labelledby="celebration-title"
		tabindex="-1"
		class="relative w-full max-w-md rounded-lg border border-line bg-surface p-8 text-center shadow-xl focus:outline-none"
	>
		<button
			type="button"
			onclick={onclose}
			class="absolute top-3 right-3 rounded p-1 text-fg-subtle hover:bg-surface-hover hover:text-fg"
			aria-label={t('common.close')}
		>
			<Icon name="x" size={14} />
		</button>
		<div class="mx-auto grid size-14 place-items-center rounded-full bg-success-soft text-success">
			<Icon name="check" size={28} strokeWidth={2} />
		</div>
		<h2 id="celebration-title" class="mt-5 text-[22px] font-semibold tracking-tight">{t('celebration.title')}</h2>
		<p class="mt-2 text-[14px] text-fg-muted">
			{t('celebration.body', { course: courseTitle, count: stepCount })}
		</p>
		<div class="mt-7 flex flex-col gap-2 sm:flex-row sm:justify-center">
			<Button variant="primary" href="/courses">{t('celebration.next')} <Icon name="arrow-right" size={14} /></Button>
			<Button href="/courses/{courseId}">{t('celebration.overview')}</Button>
		</div>
		<button type="button" onclick={onclose} class="mt-4 text-[12px] text-fg-subtle hover:text-fg">{t('celebration.keep')}</button>
	</div>
</div>
