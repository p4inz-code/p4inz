import { describe, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Page from './+page.svelte';
import { ApiError, type SearchResponse } from '$lib';

const { searchKnowledgeMock } = vi.hoisted(() => ({ searchKnowledgeMock: vi.fn() }));

vi.mock('$lib', async () => {
	const actual = await vi.importActual<typeof import('$lib')>('$lib');
	return { ...actual, searchKnowledge: searchKnowledgeMock };
});

describe('/knowledge/+page.svelte', () => {
	test('submitting a query renders the returned results', async () => {
		const response: SearchResponse = {
			query: 'p4inz',
			results: [
				{
					id: '1',
					category: 'projects',
					title: 'P4inz',
					body: 'A Discord bot.',
					source_kind: 'administrator',
					source_reference: null,
					version: 1,
					created_at: '2026-01-01T00:00:00Z',
					updated_at: '2026-01-01T00:00:00Z',
					synchronized_at: null
				}
			]
		};
		searchKnowledgeMock.mockResolvedValueOnce(response);

		const screen = render(Page);
		await screen.getByLabelText('Search').fill('p4inz');
		await screen.getByRole('button', { name: 'Search' }).click();

		await expect
			.element(screen.getByRole('heading', { name: 'P4inz', level: 2 }))
			.toBeVisible();
		await expect.element(screen.getByText('A Discord bot.')).toBeVisible();

		// Provenance UI (Milestone 48): each result exposes its source and
		// version history via a disclosure.
		await expect.element(screen.getByText('Source & history')).toBeVisible();
	});

	test('an empty result set shows a no-results message', async () => {
		searchKnowledgeMock.mockResolvedValueOnce({ query: 'nothing', results: [] });

		const screen = render(Page);
		await screen.getByLabelText('Search').fill('nothing');
		await screen.getByRole('button', { name: 'Search' }).click();

		await expect.element(screen.getByText('No results for "nothing".')).toBeVisible();
	});

	test('a failed search shows the error message as an alert', async () => {
		searchKnowledgeMock.mockRejectedValueOnce(
			new ApiError('search index unavailable', 'unavailable')
		);

		const screen = render(Page);
		await screen.getByLabelText('Search').fill('p4inz');
		await screen.getByRole('button', { name: 'Search' }).click();

		await expect
			.element(screen.getByRole('alert'))
			.toHaveTextContent('search index unavailable');
	});
});
