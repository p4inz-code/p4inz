import { describe, expect, test, vi } from 'vitest';
import { ApiError, apiFetch } from './client';

describe('apiFetch', () => {
	test('builds a URL with query parameters against the configured base URL', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: true,
			json: () => Promise.resolve({ ok: true })
		} as Response);

		await apiFetch('/v1/example', { a: '1', b: 'two words' }, fetchImpl);

		const calledUrl = fetchImpl.mock.calls[0][0] as string;
		expect(calledUrl).toContain('/v1/example');
		expect(calledUrl).toContain('a=1');
		expect(calledUrl).toContain('b=two+words');
	});

	test('propagates the parsed response body on success', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: true,
			json: () => Promise.resolve({ value: 42 })
		} as Response);

		const result = await apiFetch<{ value: number }>('/v1/example', undefined, fetchImpl);

		expect(result).toEqual({ value: 42 });
	});

	test('carries the error code through to the thrown ApiError', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: false,
			status: 403,
			json: () => Promise.resolve({ code: 'forbidden', message: 'nope' })
		} as Response);

		try {
			await apiFetch('/v1/example', undefined, fetchImpl);
			expect.unreachable('apiFetch should have thrown');
		} catch (error) {
			expect(error).toBeInstanceOf(ApiError);
			expect((error as ApiError).code).toBe('forbidden');
			expect((error as ApiError).message).toBe('nope');
		}
	});
});
