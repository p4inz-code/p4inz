import { createRawSnippet } from 'svelte';
import { describe, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Layout from './+layout.svelte';

describe('+layout.svelte', () => {
	test('provides a skip link targeting the main landmark', async () => {
		const children = createRawSnippet(() => ({ render: () => `<p>Page content</p>` }));
		const screen = render(Layout, { children });

		const skipLink = screen.getByRole('link', { name: 'Skip to content' });
		await expect.element(skipLink).toHaveAttribute('href', '#main-content');

		const main = screen.getByRole('main');
		await expect.element(main).toHaveAttribute('id', 'main-content');
		await expect.element(main).toHaveAttribute('tabindex', '-1');
	});
});
