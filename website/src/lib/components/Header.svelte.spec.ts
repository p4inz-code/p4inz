import { describe, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Header from './Header.svelte';

describe('Header', () => {
	test('the wordmark links back to home', async () => {
		const screen = render(Header);

		const link = screen.getByRole('link', { name: 'P4inz' });
		await expect.element(link).toBeVisible();
		await expect.element(link).toHaveAttribute('href', '/');
	});

	test('links to the documentation section', async () => {
		const screen = render(Header);

		const link = screen.getByRole('link', { name: 'Documentation' });
		await expect.element(link).toBeVisible();
		await expect.element(link).toHaveAttribute('href', '/docs');
	});
});
