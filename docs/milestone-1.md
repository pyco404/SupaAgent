# Milestone 1 validation baseline

The preceding implementation turn was interrupted before its completion report was written. Milestone 2 began by inspecting and revalidating the existing six-crate implementation rather than replacing it.

The inherited baseline passed Cargo formatting, Clippy with warnings denied, and 9 unit tests. After restarting the existing isolated PostgreSQL 16 cluster, all 14 opt-in integration tests passed: 11 runtime/PostgreSQL tests, one HTTP test, and two local OpenAI wire-contract tests. The initial integration attempt failed because the database was stopped; it passed after the cluster was started. No live OpenAI call was made.

That baseline already implemented lifecycle persistence, independent simulated treasuries, permissioned tools, bounded reasoning cycles, PostgreSQL recovery leases, personality, draft-only generation, authenticated loopback HTTP routes, and documentation. Milestone 2 extends these components with token/cost accounting, retry/resource budgets, provenance-aware learning, goal assessment, economic context, conservation and stronger lifecycle fencing.

Git remains an empty read-only `.git` placeholder. It was not deleted, overwritten or moved. No commits could be created. See [the Milestone 2 report](milestone-2.md) for the final implementation, validation and demonstration evidence.
