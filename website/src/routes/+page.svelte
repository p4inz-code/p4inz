<script lang="ts">
	import { Button, Container } from '$lib';
</script>

<svelte:head>
	<title>P4inz</title>
	<meta
		name="description"
		content="P4inz is Northbyte Studios' official community intelligence and information Discord application."
	/>
</svelte:head>

<Container>
	<section class="hero">
		<h1>P4inz</h1>
		<p class="tagline">
			Northbyte Studios' official community intelligence and information Discord application.
		</p>
		<p>
			P4inz makes accurate, current, structured information about Northbyte Studios, its
			projects, releases, community, and public resources accessible through Discord — and
			through this site.
		</p>
		<div class="actions">
			<Button href="https://github.com/p4inz-code/p4inz" variant="primary">
				View on GitHub
			</Button>
		</div>
	</section>

	<section>
		<h2>Ask, and get an answer grounded in real sources</h2>
		<p>
			Community members can ask natural-language questions and receive useful, concise,
			accurate answers based on maintained and verifiable project information — not guesses.
		</p>
	</section>

	<section>
		<h2>Built on a few non-negotiable principles</h2>
		<ul>
			<li>Truth over confidence.</li>
			<li>Sources over unsupported claims.</li>
			<li>User privacy over unnecessary data collection.</li>
			<li>Open-source and self-hostable architecture.</li>
		</ul>
	</section>
</Container>

<style>
	.hero {
		padding-block: var(--space-6) var(--space-5);
	}

	h1 {
		font-size: var(--font-size-2xl);
		margin-block-end: var(--space-2);
	}

	.tagline {
		font-size: var(--font-size-lg);
		color: var(--color-text-muted);
		margin-block-end: var(--space-3);
	}

	.actions {
		margin-block-start: var(--space-4);
	}

	section {
		padding-block: var(--space-5);
		border-top: 1px solid var(--color-border);
	}

	section:first-child {
		border-top: none;
	}
</style>
