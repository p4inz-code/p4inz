// place files you want to import through the `$lib` alias in this folder.
export { default as Button } from './components/Button.svelte';
export { default as Container } from './components/Container.svelte';
export { default as Provenance } from './components/Provenance.svelte';

export { ApiError, apiFetch, API_BASE_URL } from './api/client';
export { searchKnowledge, type KnowledgeItem, type SearchResponse } from './api/knowledge';
