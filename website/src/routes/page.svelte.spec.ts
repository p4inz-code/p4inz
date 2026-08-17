import { describe, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Page from './+page.svelte';

describe('/+page.svelte', () => {
	test('renders the product name', async () => {
		const screen = render(Page);

		await expect.element(screen.getByRole('heading', { name: 'P4inz' })).toBeVisible();
	});

	test('links to the real GitHub repository', async () => {
		const screen = render(Page);

		const link = screen.getByRole('link', { name: 'View on GitHub' });
		await expect.element(link).toHaveAttribute('href', 'https://github.com/p4inz-code/p4inz');
	});
});
