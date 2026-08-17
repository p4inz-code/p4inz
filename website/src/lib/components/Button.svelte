<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';

	type Variant = 'primary' | 'secondary';

	type Props = (HTMLButtonAttributes | HTMLAnchorAttributes) & {
		variant?: Variant;
		href?: string;
		children: Snippet;
	};

	let { variant = 'primary', href, children, ...rest }: Props = $props();
</script>

<!--
  Design System (Milestone 44): the one interactive-action component
  every later page (Public Home, Knowledge Explorer, ...) reuses, rather
  than each hand-rolling its own button/link styling. Renders an <a> when
  `href` is given, a <button> otherwise — both get the same visual
  treatment and the same visible focus ring (never suppressed; see
  tokens.css), since keyboard/screen-reader users need a real focusable,
  correctly-labeled element either way (section 11/21: "Accessible").
-->
{#if href}
	<!-- `href` is an opaque passthrough prop: this generic component has no
	     fixed internal route to validate with `resolve()`, unlike a page
	     that hardcodes its own link targets. -->
	<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
	<a {href} class="btn {variant}" {...rest as HTMLAnchorAttributes}>
		{@render children()}
	</a>
{:else}
	<button class="btn {variant}" {...rest as HTMLButtonAttributes}>
		{@render children()}
	</button>
{/if}

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		border-radius: var(--radius);
		border: 1px solid transparent;
		padding: var(--space-2) var(--space-3);
		font-family: inherit;
		font-size: var(--font-size-base);
		font-weight: 600;
		line-height: 1;
		text-decoration: none;
		cursor: pointer;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease;
	}

	.btn:disabled {
		cursor: not-allowed;
		opacity: 0.6;
	}

	.primary {
		background: var(--color-accent);
		color: var(--color-accent-contrast);
	}

	.primary:hover:not(:disabled) {
		filter: brightness(1.1);
	}

	.secondary {
		background: transparent;
		color: var(--color-text);
		border-color: var(--color-border);
	}

	.secondary:hover:not(:disabled) {
		background: var(--color-bg-subtle);
	}
</style>
