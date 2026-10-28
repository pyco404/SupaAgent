export function readResource(path: string, headers: Headers | Record<string, string>, fetcher?: typeof fetch): Promise<{status: number; body: unknown}>;
export function runAutonomy(path: string, headers: Headers | Record<string, string>, body: string, fetcher?: typeof fetch): Promise<{status: number; body: unknown}>;
