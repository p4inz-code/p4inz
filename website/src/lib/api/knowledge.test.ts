import { describe, expect, test, vi } from 'vitest';
import { ApiError } from './client';
import { searchKnowledge } from './knowledge';

describe('searchKnowledge', () => {
	test('returns parsed results on success', async () => {
		const body = { query: 'p4inz', results: [] };
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: true,
			json: () => Promise.resolve(body)
		} as Response);

		const result = await searchKnowledge('p4inz', fetchImpl);

		expect(result).toEqual(body);
		expect(fetchImpl).toHaveBeenCalledWith(
			expect.stringContaining('/v1/knowledge/search?q=p4inz')
		);
	});

	test('throws ApiError with the API-provided message on failure', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: false,
			status: 400,
			json: () => Promise.resolve({ code: 'validation', message: 'query must not be empty' })
		} as Response);

		await expect(searchKnowledge('', fetchImpl)).rejects.toThrow(ApiError);
		await expect(searchKnowledge('', fetchImpl)).rejects.toThrow('query must not be empty');
	});

	test('falls back to a generic message when the error body is unparseable', async () => {
		const fetchImpl = vi.fn().mockResolvedValue({
			ok: false,
			status: 500,
			json: () => Promise.reject(new Error('not json'))
		} as unknown as Response);

		await expect(searchKnowledge('x', fetchImpl)).rejects.toThrow('status 500');
	});
});
