<script lang="ts">
	import PageHeader from '#lib/components/layout/PageHeader.svelte';

	// Operators of their own instance: change these to match your deployment.
	const contact = 'info@polinderit.nl';
	const hostingProvider = 'TransIP';
	// Company behind SMTP_URL, e.g. 'Brevo' or 'Postmark'. Name it once you have chosen one.
	const mailProvider: string | null = 'TransIP';
	const backupRetentionDays = 14; // BACKUP_RETENTION_DAYS in setup.sh
	const logRetentionDays = 30; // ACCESS_LOG_RETENTION_DAYS in setup.sh
	const sessionDays = 14;
	const connectionDays = 90;
</script>

<svelte:head><title>Privacy · Learning Platform</title></svelte:head>

<div class="mx-auto max-w-3xl px-6 py-8">
	<PageHeader title="Privacy" description="What this platform stores about you, why, and for how long." />

	<div class="mt-6 flex flex-col gap-8 text-[14px] leading-relaxed text-fg-muted [&_h2]:mb-2 [&_h2]:text-[14px] [&_h2]:font-medium [&_h2]:text-fg">
		<section>
			<h2>Who is responsible for your data</h2>
			<p>
				This platform is open source and run by an individual as a personal project, not by a company. The person who runs this instance is the
				controller under the AVG (the Dutch name for the GDPR). For privacy questions or requests, email
				<a href="mailto:{contact}" class="font-medium text-fg hover:underline">{contact}</a>.
			</p>
			<p class="mt-2">
				Anyone can install the platform on their own server. If you use an instance run by someone else, that person is the controller for that
				instance, and this page describes the platform's default behaviour, not theirs. Ask them for their own privacy information.
			</p>
		</section>

		<section>
			<h2>What we process, why, and how long</h2>
			<p>We only process the personal data the platform needs. The table shows the purpose, the legal basis (article numbers refer to the AVG), and how long we keep each category.</p>
			<div class="mt-3 overflow-x-auto">
				<table class="w-full border-collapse text-left text-[13px]">
					<thead>
						<tr class="border-b border-line text-fg">
							<th class="py-2 pr-4 font-medium">Data</th>
							<th class="py-2 pr-4 font-medium">Purpose</th>
							<th class="py-2 pr-4 font-medium">Legal basis</th>
							<th class="py-2 font-medium">Kept for</th>
						</tr>
					</thead>
					<tbody class="align-top [&_td]:border-b [&_td]:border-line [&_td]:py-2 [&_td]:pr-4">
						<tr>
							<td>Account: email, display name, password hash, role, language, whether and when you confirmed your email address</td>
							<td>Creating and running your account</td>
							<td>Contract (Art. 6(1)(b))</td>
							<td>While your account exists</td>
						</tr>
						<tr>
							<td>Account emails: the address a link was sent to, a hash of the link (never the link itself), when it expires and whether it was used</td>
							<td>Confirming your email address, resetting your password, changing your email address, and telling you about changes to your account</td>
							<td>Contract (Art. 6(1)(b))</td>
							<td>While your account exists. Links stop working after 1 to 48 hours.</td>
						</tr>
						<tr>
							<td>Account overview for administrators: email, display name, sign-up and sign-in times, number of courses started</td>
							<td>Spotting fake and automated sign-ups</td>
							<td>Legitimate interest (Art. 6(1)(f))</td>
							<td>While your account exists</td>
						</tr>
						<tr>
							<td>Learning progress: courses started, answers and results per step, timestamps, course activity log</td>
							<td>Tracking your progress and showing your results</td>
							<td>Contract (Art. 6(1)(b))</td>
							<td>While your account exists</td>
						</tr>
						<tr>
							<td>AI agent connections: connection label, client name reported by the agent, token hash (never the token itself), status, last use, expiry</td>
							<td>Letting an AI agent you connect access your courses</td>
							<td>Contract (Art. 6(1)(b))</td>
							<td>While your account exists (a revoked connection stays listed as revoked). Access expires after {connectionDays} days.</td>
						</tr>
						<tr>
							<td>Courses you author (super admins): your user ID as creator</td>
							<td>Recording who created each course version</td>
							<td>Contract (Art. 6(1)(b))</td>
							<td>As long as the course version exists; the link to you is removed if your account is deleted</td>
						</tr>
						<tr>
							<td>Session cookie <span class="font-mono text-[12px]">lp_session</span></td>
							<td>Keeping you signed in</td>
							<td>Necessary for the service; no consent required for strictly necessary cookies</td>
							<td>Up to {sessionDays} days</td>
						</tr>
						<tr>
							<td>Web server logs: time, IP address, requested path and query string, response status, user agent</td>
							<td>Security, keeping the service running, detecting abuse</td>
							<td>Legitimate interest (Art. 6(1)(f))</td>
							<td>{logRetentionDays} days</td>
						</tr>
						<tr>
							<td>Traffic statistics (administrators only), generated from the logs</td>
							<td>Understanding how much the platform is used</td>
							<td>Legitimate interest (Art. 6(1)(f))</td>
							<td>Regenerated every 15 minutes from the logs, so it covers the same period</td>
						</tr>
						<tr>
							<td>Database backups: a daily full copy of the database</td>
							<td>Recovering data after a failure</td>
							<td>Legitimate interest (Art. 6(1)(f))</td>
							<td>{backupRetentionDays} days, then deleted automatically</td>
						</tr>
					</tbody>
				</table>
			</div>
		</section>

		<section>
			<h2>Cookies</h2>
			<p>
				We use one cookie, <span class="font-mono text-[13px]">lp_session</span>, to keep you signed in for up to {sessionDays} days. It is strictly
				necessary for the platform to work, so we do not ask for consent to set it. We use no advertising, tracking or third-party analytics cookies.
			</p>
		</section>

		<section>
			<h2>AI agents you connect</h2>
			<p>
				You can connect an AI agent of your choice, either with a token you create or through a sign-in (OAuth) flow. A connected agent can read course
				content, your current step and session status, and submit answers for a step. Super admins' agents can also list, validate and publish courses.
			</p>
			<p class="mt-2">
				The platform itself does not send your data to any AI model provider. Only the agent you connect receives the data it requests. Once an agent
				has that data, its provider processes it under the provider's own terms and privacy policy, which may involve processing outside the European
				Economic Area. Check those terms before you connect an agent. We do not choose or engage these providers: you do, and
				the provider is then responsible for its own processing.
			</p>
			<p class="mt-2">
				You can revoke a connection at any time, and the agent loses access immediately. Access granted through sign-in ends after {connectionDays} days,
				and each access token lasts one hour.
			</p>
		</section>

		<section>
			<h2>Who else receives your data</h2>
			<p>
				We do not sell your data or share it for marketing, and we use no third-party analytics or advertising services.
				{#if mailProvider === hostingProvider}
					The only external party involved is {hostingProvider}, which acts as a processor on our behalf: it rents us the virtual server that holds
					the database, logs and backups, and it delivers account emails, so it also receives your email address, display name and the text of
					those emails.
				{:else}
					Two external parties act as processors on our behalf: the hosting provider, {hostingProvider}, which rents us the virtual server that holds
					the database, logs and backups; and {mailProvider ?? 'an email delivery provider'}, which delivers account emails and so receives your email
					address, display name and the text of those emails.
				{/if}
				We only send account emails, never newsletters or marketing. Beyond this, the platform has no other processors.
			</p>
		</section>

		<section>
			<h2>Where your data is stored</h2>
			<p>
				All data, including backups, is stored on that one server. Apart from account emails, which pass through the email delivery provider, the
				platform does not copy it elsewhere or send it to other countries. Transfers outside the European Economic Area can only happen through an
				AI agent you connect, as described above.
			</p>
		</section>

		<section>
			<h2>Deleting your account</h2>
			<p>
				You can delete your account yourself under <a href="/settings" class="font-medium text-fg hover:underline">Settings</a>; you confirm it with
				your password. You can also email <a href="mailto:{contact}" class="font-medium text-fg hover:underline">{contact}</a>; we handle the
				request within one month. When your account is deleted:
			</p>
			<ul class="mt-2 list-disc pl-5">
				<li>
					Your account, progress, notes, project, email links and AI agent connections are removed from the live database right away, and
					connected agents lose access. We send one last email to confirm the deletion.
				</li>
				<li>Server logs containing your IP address are removed when they reach the {logRetentionDays}-day limit.</li>
				<li>Backups are not edited, so they still contain your data until they expire after {backupRetentionDays} days.</li>
				<li>Course versions you authored stay available, but are no longer linked to your account.</li>
				<li>
					An AI agent may keep any copy of data it has already received. Its provider's retention is governed by that provider's own policy.
				</li>
			</ul>
		</section>

		<section>
			<h2>Your rights</h2>
			<p>Under the AVG you have the right to:</p>
			<ul class="mt-2 list-disc pl-5">
				<li>access the personal data we hold about you (Art. 15);</li>
				<li>have inaccurate data corrected (Art. 16);</li>
				<li>have your data deleted, as described above (Art. 17);</li>
				<li>restrict processing (Art. 18) or object to processing based on legitimate interest, such as logs, statistics and backups (Art. 21);</li>
				<li>
					receive the data you provided to us in a structured, commonly used, machine-readable format, and have it sent to you or another controller,
					where processing is based on contract (Art. 20). There is no export button yet, so ask by email;
				</li>
				<li>
					withdraw consent (Art. 7). We currently do not process personal data based on consent, so this does not apply. If we add a feature that needs
					consent, we will ask for it separately and explain how to withdraw it;
				</li>
				<li>
					not be subject to decisions based solely on automated processing that have legal or similarly significant effects (Art. 22). We do not make
					such decisions.
				</li>
			</ul>
			<p class="mt-2">
				Send your request to <a href="mailto:{contact}" class="font-medium text-fg hover:underline">{contact}</a>. We respond within one month and may
				extend this by two months for complex requests, in which case we will tell you why. We may ask you to confirm your identity first.
			</p>
			<p class="mt-2">
				You can also file a complaint with the Dutch Data Protection Authority (Autoriteit Persoonsgegevens) or the supervisory authority in your country.
			</p>
		</section>

		<section>
			<h2>Security</h2>
			<p>
				Passwords and AI agent tokens are stored only as hashes, so we cannot read them back. Traffic to the platform is encrypted with HTTPS.
			</p>
		</section>

		<section>
			<h2>Changes to this policy</h2>
			<p>
				We update this page when our processing changes.
			</p>
		</section>
	</div>
</div>
