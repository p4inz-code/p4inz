import { describe, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Provenance from './Provenance.svelte';

describe('Provenance', () => {
	test('shows the source kind when there is no source reference', async () => {
		const screen = render(Provenance, {
			sourceKind: 'administrator',
			sourceReference: null,
			version: 3,
			updatedAt: '2026-01-15T10:30:00Z',
			synchronizedAt: null
		});

		await screen.getByText('Source & history').click();

		await expect.element(screen.getByText('administrator')).toBeVisible();
		await expect.element(screen.getByText('3')).toBeVisible();
	});

	test('links to the source reference when one is present', async () => {
		const screen = render(Provenance, {
			sourceKind: 'repository',
			sourceReference: 'https://github.com/p4inz-code/p4inz',
			version: 2,
			updatedAt: '2026-01-15T10:30:00Z',
			synchronizedAt: '2026-01-16T09:00:00Z'
		});

		await screen.getByText('Source & history').click();

		const link = screen.getByRole('link', { name: 'https://github.com/p4inz-code/p4inz' });
		await expect.element(link).toHaveAttribute('href', 'https://github.com/p4inz-code/p4inz');
		await expect.element(screen.getByText('Last synchronized')).toBeVisible();
	});

	test('omits "last synchronized" when the item was never synchronized', async () => {
		const screen = render(Provenance, {
			sourceKind: 'administrator',
			sourceReference: null,
			version: 1,
			updatedAt: '2026-01-15T10:30:00Z',
			synchronizedAt: null
		});

		await screen.getByText('Source & history').click();

		await expect.element(screen.getByText('Last synchronized')).not.toBeInTheDocument();
	});
});
