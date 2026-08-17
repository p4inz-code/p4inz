<script lang="ts">
	import type { Snippet } from 'svelte';

	let { children }: { children: Snippet } = $props();
</script>

<!--
  Design System (Milestone 44): the shared content-width/padding wrapper
  every page uses, so line lengths stay readable (an accessibility and
  readability concern, not just aesthetic) without each page redeclaring
  the same max-width/padding rules.
-->
<div class="container">
	{@render children()}
</div>

<style>
	.container {
		max-width: var(--max-content-width);
		margin-inline: auto;
		padding-inline: var(--space-4);
	}
</style>
