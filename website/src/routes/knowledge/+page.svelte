<script lang="ts">
	import {
		ApiError,
		Button,
		Container,
		Provenance,
		searchKnowledge,
		type KnowledgeItem
	} from '$lib';

	type Status = 'idle' | 'loading' | 'success' | 'error';

	let query = $state('');
	let results = $state<KnowledgeItem[]>([]);
	let status = $state<Status>('idle');
	let errorMessage = $state('');

	function preview(body: string, maxLength = 220): string {
		if (body.length <= maxLength) return body;
		return `${body.slice(0, maxLength).trimEnd()}…`;
	}

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();
		const trimmed = query.trim();
		if (!trimmed) return;

		status = 'loading';
		try {
			const response = await searchKnowledge(trimmed);
			results = response.results;
			status = 'success';
		} catch (error) {
			errorMessage = error instanceof ApiError ? error.message : 'Something went wrong.';
			status = 'error';
		}
	}
</script>

<svelte:head>
	<title>Knowledge Explorer — P4inz</title>
	<meta name="description" content="Search published P4inz knowledge." />
</svelte:head>

<Container>
	<h1>Knowledge Explorer</h1>
	<p>Search the same published knowledge P4inz answers Discord questions from.</p>

	<form onsubmit={onSubmit}>
		<label for="knowledge-search-query">Search</label>
		<div class="search-row">
			<input
				id="knowledge-search-query"
				type="search"
				bind:value={query}
				placeholder="e.g. what is P4inz?"
				autocomplete="off"
			/>
			<Button variant="primary">Search</Button>
		</div>
	</form>

	<div aria-live="polite">
		{#if status === 'loading'}
			<p>Searching…</p>
		{:else if status === 'error'}
			<p role="alert">{errorMessage}</p>
		{:else if status === 'success' && results.length === 0}
			<p>No results for "{query}".</p>
		{:else if status === 'success'}
			<ul class="results">
				{#each results as item (item.id)}
					<li>
						<article>
							<h2>{item.title}</h2>
							<p class="meta">{item.category}</p>
							<p>{preview(item.body)}</p>
							<Provenance
								sourceKind={item.source_kind}
								sourceReference={item.source_reference}
								version={item.version}
								updatedAt={item.updated_at}
								synchronizedAt={item.synchronized_at}
							/>
						</article>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</Container>

<style>
	h1 {
		font-size: var(--font-size-2xl);
		margin-block: var(--space-5) var(--space-2);
	}

	form {
		margin-block: var(--space-4);
	}

	label {
		display: block;
		font-weight: 600;
		margin-block-end: var(--space-1);
	}

	.search-row {
		display: flex;
		gap: var(--space-2);
	}

	input[type='search'] {
		flex: 1;
		border: 1px solid var(--color-border);
		border-radius: var(--radius);
		padding: var(--space-2) var(--space-3);
		font-size: var(--font-size-base);
		font-family: inherit;
		background: var(--color-bg);
		color: var(--color-text);
	}

	.results {
		list-style: none;
		margin: 0;
		padding: 0;
	}

	.results li {
		padding-block: var(--space-3);
		border-top: 1px solid var(--color-border);
	}

	.results li:first-child {
		border-top: none;
	}

	h2 {
		font-size: var(--font-size-lg);
		margin-block-end: var(--space-1);
	}

	.meta {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-transform: capitalize;
		margin-block-end: var(--space-2);
	}
</style>
