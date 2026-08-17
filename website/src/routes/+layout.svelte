<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';
	import '$lib/styles/tokens.css';
	import Header from '$lib/components/Header.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

<!--
  Accessibility/Performance (Milestone 50): a skip link lets keyboard and
  screen-reader users jump straight past the header nav to page content —
  without it, every page load forces tabbing through the same nav links
  first. Visually hidden until focused, matching the standard pattern.
-->
<a class="skip-link" href="#main-content">Skip to content</a>

<Header />

<main id="main-content" tabindex="-1">
	{@render children()}
</main>

<style>
	.skip-link {
		position: absolute;
		left: -9999px;
		top: 0;
		z-index: 100;
		padding: var(--space-2) var(--space-3);
		background: var(--color-accent);
		color: var(--color-accent-contrast);
		border-radius: var(--radius);
	}

	.skip-link:focus {
		left: var(--space-2);
		top: var(--space-2);
	}
</style>
