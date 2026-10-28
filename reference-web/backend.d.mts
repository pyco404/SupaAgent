export function readResource(path: string, headers: Headers | Record<string, string>, fetcher?: typeof fetch): Promise<{status: number; body: unknown}>;
