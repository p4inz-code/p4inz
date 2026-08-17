<script lang="ts">
	import { resolve } from '$app/paths';
	import Container from './Container.svelte';
</script>

<!--
  Public Home (Milestone 45): the one piece of site-wide "product
  identity" every page shares — not page content itself, which belongs
  to whichever route renders it. The Documentation link (Milestone 46) is
  the site's only other top-level section so far.
-->
<header>
	<Container>
		<nav aria-label="Primary">
			<a class="wordmark" href={resolve('/')}>P4inz</a>
			<a href={resolve('/knowledge')}>Knowledge</a>
			<a href={resolve('/docs')}>Documentation</a>
		</nav>
	</Container>
</header>

<style>
	header {
		border-bottom: 1px solid var(--color-border);
	}

	nav {
		display: flex;
		align-items: center;
		gap: var(--space-4);
		padding-block: var(--space-3);
	}

	.wordmark {
		font-size: var(--font-size-lg);
		font-weight: 700;
		color: var(--color-text);
		text-decoration: none;
	}

	nav a:not(.wordmark) {
		color: var(--color-text-muted);
		text-decoration: none;
	}

	nav a:not(.wordmark):hover {
		color: var(--color-text);
	}
</style>
