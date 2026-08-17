<script lang="ts">
	interface Props {
		sourceKind: string;
		sourceReference: string | null;
		version: number;
		updatedAt: string;
		synchronizedAt: string | null;
	}

	let { sourceKind, sourceReference, version, updatedAt, synchronizedAt }: Props = $props();

	function formatTimestamp(value: string): string {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value;
		return date.toLocaleString(undefined, {
			dateStyle: 'medium',
			timeStyle: 'short'
		});
	}
</script>

<!--
  Provenance UI (Milestone 48): surfaces exactly the fields the Public
  Knowledge API already returns (Milestone 39) — where a piece of
  knowledge came from, how many times it's changed, and when it was last
  updated/synchronized — so a reader can judge an answer's trustworthiness
  themselves rather than taking it on faith
  (docs/PROJECT_SPEC.md section 6: "Knowledge should support: Provenance,
  Source identification, Versioning, Freshness").
-->
<details class="provenance">
	<summary>Source & history</summary>
	<dl>
		<dt>Source</dt>
		<dd>
			{#if sourceReference}
				<!-- Always an external source URL (e.g. a GitHub repository), never
				     an internal route, so `resolve()` doesn't apply. -->
				<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
				<a href={sourceReference} rel="noopener noreferrer">{sourceReference}</a>
			{:else}
				{sourceKind}
			{/if}
		</dd>

		<dt>Version</dt>
		<dd>{version}</dd>

		<dt>Last updated</dt>
		<dd>{formatTimestamp(updatedAt)}</dd>

		{#if synchronizedAt}
			<dt>Last synchronized</dt>
			<dd>{formatTimestamp(synchronizedAt)}</dd>
		{/if}
	</dl>
</details>

<style>
	.provenance {
		margin-block-start: var(--space-2);
		font-size: var(--font-size-sm);
	}

	summary {
		cursor: pointer;
		color: var(--color-text-muted);
		width: fit-content;
	}

	summary:hover {
		color: var(--color-text);
	}

	dl {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: var(--space-1) var(--space-3);
		margin-block-start: var(--space-2);
		margin-block-end: 0;
	}

	dt {
		color: var(--color-text-muted);
		font-weight: 600;
	}

	dd {
		margin: 0;
		word-break: break-word;
	}
</style>
