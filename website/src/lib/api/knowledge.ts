import { apiFetch } from './client';

export interface KnowledgeItem {
	id: string;
	category: string;
	title: string;
	body: string;
	source_kind: string;
	source_reference: string | null;
	version: number;
	created_at: string;
	updated_at: string;
	synchronized_at: string | null;
}

export interface SearchResponse {
	query: string;
	results: KnowledgeItem[];
}

/** Searches published knowledge via `GET /v1/knowledge/search`
 * (`crates/api/src/knowledge.rs`, Milestone 39). */
export function searchKnowledge(query: string, fetchImpl?: typeof fetch): Promise<SearchResponse> {
	return apiFetch<SearchResponse>('/v1/knowledge/search', { q: query }, fetchImpl);
}
