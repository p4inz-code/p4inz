// API Integration (Milestone 49): the one typed client every page that
// talks to the P4inz API goes through, replacing Knowledge Explorer's
// (Milestone 47) page-local fetch wrapper now that a second consumer
// would otherwise have to duplicate it.

const DEFAULT_API_BASE_URL = 'http://localhost:8080';

/** The API's base URL — configurable per deployment via `VITE_API_BASE_URL`
 * (the website is deployed independently of the API, per `docs/development/
 * implementation_plan.md` section 11). */
export const API_BASE_URL =
	(import.meta.env.VITE_API_BASE_URL as string | undefined) ?? DEFAULT_API_BASE_URL;

interface ApiErrorBody {
	code: string;
	message: string;
}

/** Thrown when the API responds with a non-2xx status. `message` is
 * always the API's own safe, user-facing error message
 * (`crates/api/src/error.rs`'s `ApiError` — `Internal` errors are never
 * more specific than "internal error", so this never leaks anything the
 * API itself wouldn't already show). */
export class ApiError extends Error {
	code: string;

	constructor(message: string, code: string) {
		super(message);
		this.code = code;
	}
}

/**
 * Calls a P4inz API endpoint and parses the JSON response, throwing
 * {@link ApiError} on any non-2xx status. `fetchImpl` is injected
 * (defaulting to the global `fetch`) so tests can supply a fake
 * implementation without a real network call — the same
 * dependency-injection-for-testability pattern used throughout the Rust
 * side of this codebase.
 */
export async function apiFetch<T>(
	path: string,
	params?: Record<string, string>,
	fetchImpl: typeof fetch = fetch
): Promise<T> {
	const url = new URL(path, API_BASE_URL);
	for (const [key, value] of Object.entries(params ?? {})) {
		url.searchParams.set(key, value);
	}

	const response = await fetchImpl(url.toString());

	if (!response.ok) {
		const body = (await response.json().catch(() => null)) as ApiErrorBody | null;
		throw new ApiError(
			body?.message ?? `request failed with status ${response.status}`,
			body?.code ?? 'unknown'
		);
	}

	return (await response.json()) as T;
}
