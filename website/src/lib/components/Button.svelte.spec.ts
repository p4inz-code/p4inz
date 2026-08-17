import { createRawSnippet } from 'svelte';
import { describe, expect, test, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import Button from './Button.svelte';

function textSnippet(text: string) {
	return createRawSnippet(() => ({
		render: () => `<span>${text}</span>`
	}));
}

describe('Button', () => {
	test('renders as a real <button> and responds to clicks', async () => {
		const onclick = vi.fn();
		const screen = render(Button, { onclick, children: textSnippet('Click me') });

		const button = screen.getByRole('button', { name: 'Click me' });
		await expect.element(button).toBeVisible();

		await button.click();
		expect(onclick).toHaveBeenCalledOnce();
	});

	test('renders as a link when href is given', async () => {
		const screen = render(Button, { href: '/docs', children: textSnippet('Docs') });

		const link = screen.getByRole('link', { name: 'Docs' });
		await expect.element(link).toBeVisible();
		await expect.element(link).toHaveAttribute('href', '/docs');
	});

	test('a disabled button is not clickable', async () => {
		const onclick = vi.fn();
		const screen = render(Button, {
			onclick,
			disabled: true,
			children: textSnippet('Disabled')
		});

		await expect.element(screen.getByRole('button', { name: 'Disabled' })).toBeDisabled();
	});
});
