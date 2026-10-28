"use client";

import { createContext, useCallback, useContext, useEffect, useId, useRef, useState, type ReactNode } from "react";

export type Resource<T> = { data?: T; error?: string; loading: boolean; refreshing: boolean; refresh: () => void };
type Outcome = "success" | "error" | "cancel";
type Status = { pending: number; errors: number; retrievedAt?: string; revision: number };
type Tracker = { begin: (id: string) => void; end: (id: string, outcome: Outcome) => void; status: Status };
const RecordsContext = createContext<Tracker | null>(null);

/** Track every mounted read, including nested histories. No observation or mutation is triggered. */
export function RecordsProvider({ children }: { children: ReactNode }) {
  const pending = useRef(new Set<string>()), errors = useRef(new Set<string>());
  const [status, setStatus] = useState<Status>({ pending: 0, errors: 0, revision: 0 });
  const begin = useCallback((id: string) => {
    pending.current.add(id); errors.current.delete(id);
    setStatus(previous => ({ ...previous, pending: pending.current.size, errors: errors.current.size }));
  }, []);
  const end = useCallback((id: string, outcome: Outcome) => {
    pending.current.delete(id);
    if (outcome === "error") errors.current.add(id); else errors.current.delete(id);
    const complete = outcome === "success" && pending.current.size === 0 && errors.current.size === 0;
    const counts = { pending: pending.current.size, errors: errors.current.size };
    setStatus(previous => ({ ...previous, ...counts, ...(complete ? { retrievedAt: new Date().toISOString(), revision: previous.revision + 1 } : {}) }));
  }, []);
  return <RecordsContext.Provider value={{ begin, end, status }}>{children}</RecordsContext.Provider>;
}

export function useResource<T>(path: string | null): Resource<T> {
  const tracker = useContext(RecordsContext), begin = tracker?.begin, end = tracker?.end;
  const id = useId();
  const [attempt, setAttempt] = useState(0);
  const [result, setResult] = useState<{ path: string; key: string; data?: T; error?: string }>();
  const key = `${path}:${attempt}`;
  useEffect(() => {
    if (!path) return;
    const onRefresh = () => setAttempt(value => value + 1);
    window.addEventListener("supa:refresh", onRefresh);
    return () => window.removeEventListener("supa:refresh", onRefresh);
  }, [path]);
  useEffect(() => {
    if (!path) return;
    const controller = new AbortController();
    begin?.(id);
    fetch(`/api/console/${path}`, { cache: "no-store", signal: controller.signal })
      .then(async response => {
        const json = await response.json();
        if (!response.ok) throw new Error(typeof json.error === "string" ? json.error : "request_failed");
        return json as T;
      })
      .then(data => { if (!controller.signal.aborted) { setResult({ path, key, data }); end?.(id, "success"); } })
      .catch(error => {
        if (controller.signal.aborted) return;
        setResult({ path, key, error: error instanceof Error && /^[a-z_]+$/.test(error.message) ? error.message : "connection_failed" });
        end?.(id, "error");
      });
    return () => { controller.abort(); end?.(id, "cancel"); };
  }, [path, key, id, begin, end]);
  const refresh = useCallback(() => setAttempt(value => value + 1), []);
  const data = result?.path === path ? result.data : undefined, pending = !!path && result?.key !== key;
  return { data, error: result?.key === key ? result.error : undefined, loading: pending && data === undefined, refreshing: pending && data !== undefined, refresh };
}

export function RecordsBar({ connected }: { connected: boolean }) {
  const tracker = useContext(RecordsContext)!;
  const { pending, errors, retrievedAt, revision } = tracker.status;
  return <div className="console-connection">
    <span className="connection-label"><span className={connected ? "status-dot" : "status-dot disconnected"}/>SAVED RECORDS / READ ONLY</span>
    <button disabled={pending > 0} aria-busy={pending > 0} onClick={() => window.dispatchEvent(new Event("supa:refresh"))}>{pending ? "RELOADING SAVED RECORDS…" : "RELOAD SAVED RECORDS ↻"}</button>
    <div className="records-feedback" role="status" aria-atomic="true"><span className={pending || errors ? "" : "retrieval-result"} key={revision}>{pending ? "Reading saved records…" : errors ? "Some saved records are unavailable. Check the affected section." : retrievedAt ? <>Retrieved <time dateTime={retrievedAt}>{retrievedAt.slice(0, 19).replace("T", " ")} UTC</time></> : "Waiting for saved records."}</span></div>
    <details className="records-help"><summary aria-label="About saved records"><span className="help-long">About saved records</span><span className="help-short">Details</span></summary><div className="detail-body">The latest 100 records per history are shown. Retrieval time tells you when this page received them. Reloading saved records does not query Solana, activate agents or run paid reasoning.</div></details>
  </div>;
}
