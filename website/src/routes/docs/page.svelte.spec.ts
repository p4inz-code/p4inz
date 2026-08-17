import { describe, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Page from './+page.svelte';

describe('/docs/+page.svelte', () => {
	test('explains how to talk to P4inz', async () => {
		const screen = render(Page);

		await expect
			.element(screen.getByRole('heading', { name: 'Talking to P4inz' }))
			.toBeVisible();
	});

	test('links to the real GitHub repository', async () => {
		const screen = render(Page);

		const link = screen.getByRole('link', { name: 'github.com/p4inz-code/p4inz' });
		await expect.element(link).toHaveAttribute('href', 'https://github.com/p4inz-code/p4inz');
	});
});
