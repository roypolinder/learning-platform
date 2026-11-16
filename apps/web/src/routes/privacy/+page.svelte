<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';

	const contact = 'info@polinderit.nl';
	const logRetentionDays = 30;
</script>

<svelte:head><title>Privacy · Learning Platform</title></svelte:head>

<div class="mx-auto max-w-3xl px-6 py-8">
	<PageHeader title="Privacy" description="What this platform stores about you, why, and for how long." />

	<div class="mt-6 flex flex-col gap-8 text-[14px] leading-relaxed text-fg-muted [&_h2]:mb-2 [&_h2]:text-[14px] [&_h2]:font-medium [&_h2]:text-fg">
		<section>
			<h2>Your account</h2>
			<p>
				When you register we store your email address, display name and a password hash (the password itself is never stored), plus your learning progress:
				which courses you started, your answers and results for each step, and when you did them. If you connect an AI agent, we store which agent
				connection belongs to your account. We use this only to run the platform for you.
			</p>
		</section>

		<section>
			<h2>Cookies</h2>
			<p>
				One cookie, <span class="font-mono text-[13px]">lp_session</span>, keeps you signed in for up to 14 days. It is required for the platform to work.
				We use no advertising, tracking or third-party analytics cookies.
			</p>
		</section>

		<section>
			<h2>Web server logs and traffic statistics</h2>
			<p>
				Our web server records every request: the time, your IP address, the page or API path requested (including any query string), the response status
				and your browser's user agent. We keep these logs for {logRetentionDays} days to keep the service secure and working, and to see how much it is used.
			</p>
			<p class="mt-2">
				Platform administrators can view traffic statistics generated from these logs. In that report IP addresses are shortened, query strings are removed
				and known crawlers are left out. No data is sent to third-party analytics services.
			</p>
		</section>

		<section>
			<h2>Where the data lives</h2>
			<p>
				Everything is stored on a virtual server rented from TransIP. Database backups are kept for a limited time on the same server. We do not sell
				or share your data with anyone else.
			</p>
		</section>

		<section>
			<h2>Your rights</h2>
			<p>
				You can ask to see, correct or delete your data, or object to how it is used, by emailing
				<a href="mailto:{contact}" class="font-medium text-fg hover:underline">{contact}</a>. Deleting your account removes your progress too.
				You can also complain to the Dutch data protection authority (Autoriteit Persoonsgegevens).
			</p>
		</section>
	</div>
</div>
