<script lang="ts">
	import { Container } from '$lib';
</script>

<svelte:head>
	<title>Documentation — P4inz</title>
	<meta
		name="description"
		content="How to use P4inz, what it knows about, and how it handles data."
	/>
</svelte:head>

<Container>
	<h1>Documentation</h1>

	<section>
		<h2>Talking to P4inz</h2>
		<p>
			Mention P4inz in a server channel, or send it a direct message, and ask your question in
			plain language. There's nothing to memorize — no slash commands or special syntax are
			required for this.
		</p>
	</section>

	<section>
		<h2>What P4inz knows about</h2>
		<p>P4inz can maintain information about:</p>
		<ul>
			<li>
				<strong>Northbyte Studios</strong> — organization overview, public identity, public links,
				policies, and announcements.
			</li>
			<li><strong>People</strong> — only intentionally published public information.</li>
			<li>
				<strong>Projects</strong> — name, description, status, repository, documentation, releases,
				technologies, public roadmap information, and updates.
			</li>
			<li>
				<strong>Community</strong> — public rules, public channels/categories, support information,
				applications, partnerships, feedback, notices, and frequently asked questions.
			</li>
		</ul>
	</section>

	<section>
		<h2>Where answers come from</h2>
		<p>
			P4inz distinguishes authoritative from non-authoritative information. Preferred
			authoritative sources, in order, are official repository/project documentation, official
			Northbyte Studios documentation, explicit administrator-maintained records, and official
			announcements. GitHub may be used for frequently changing project information.
		</p>
		<p>AI-generated text is never itself treated as a source of truth.</p>
	</section>

	<section>
		<h2>Privacy</h2>
		<p>
			P4inz collects only the information required for its functionality. It does not retain
			unnecessary message content, user data, identifiers, or logs containing sensitive
			information. Secrets and private community information never enter public knowledge.
		</p>
	</section>

	<section>
		<h2>Source code</h2>
		<p>
			P4inz is open source. The full source, including how questions are answered and how
			knowledge is stored, is available at
			<a href="https://github.com/p4inz-code/p4inz">github.com/p4inz-code/p4inz</a>.
		</p>
	</section>
</Container>

<style>
	h1 {
		font-size: var(--font-size-2xl);
		margin-block: var(--space-5) var(--space-4);
	}

	section {
		padding-block: var(--space-4);
		border-top: 1px solid var(--color-border);
	}

	section:first-of-type {
		border-top: none;
		padding-block-start: 0;
	}

	h2 {
		font-size: var(--font-size-lg);
		margin-block-end: var(--space-2);
	}

	li {
		margin-block-end: var(--space-1);
	}
</style>
