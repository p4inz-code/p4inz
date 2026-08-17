import { createRawSnippet } from 'svelte';
import { describe, expect, test } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Container from './Container.svelte';

describe('Container', () => {
	test('renders its children', async () => {
		const children = createRawSnippet(() => ({
			render: () => `<p>Inside the container</p>`
		}));
		const screen = render(Container, { children });

		await expect.element(screen.getByText('Inside the container')).toBeVisible();
	});
});
