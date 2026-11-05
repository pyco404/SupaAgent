# Permanent agent-token persistence

`deployment/agent-token-registry.sql` is an opt-in persistence extension tested
only in a disposable PostgreSQL 18 cluster. It is not an automatic migration and
has not been applied to Railway or the protected primary database.

## Storage guarantees

- One original binding per canonical agent UUID across supported networks.
- Global `(network, mint)` uniqueness across originals and successors.
- Receipt and evidence identifiers are one-use; concurrent conflicting inserts
  fail atomically instead of replacing a prior binding.
- Successors point to the original record and preserve its full snapshot. They
  never change the original lookup, migrate balances, or convey permissions.
- Receipts, revocations and binding history reject ordinary UPDATE, DELETE and
  TRUNCATE, including ordinary table-owner DML. A database superuser or schema
  administrator remains a trusted infrastructure authority capable of DDL; these
  triggers are not cryptographic protection against that authority.
- Confirmation and revocation serialize on the receipt. Per-agent locking and
  unique indexes protect same-agent and cross-agent confirmation races.

## Trusted receipt boundary

No default runtime, verifier or PUBLIC grants are installed. A separately reviewed
trusted verifier must run the existing Rust domain validation before writing a
receipt. It must independently verify confirmed chain evidence and current purpose
qualification. Successors additionally require explicit scoped approval and
independent verification of public disclosure. Pending, failed and unknown chain
outcomes must never receive a receipt. The SQL layer does not implement an RPC
verifier or claim that a syntactically valid address proves a launch.

Receipts bind the creator, agent, original/successor kind, network, mint,
transaction, evidence, purpose, qualification and complete domain record. Their
maximum lifetime is fifteen minutes. The digest is SHA-256 of PostgreSQL's UTF-8
`jsonb::text` representation, computed inside PostgreSQL, **not** a digest of a
client JSON serializer or the work-review canonical JSON format. Records are
bounded to 16 KiB. Only public token association audit fields belong here: no
private goals, prompts, customer prose, keys or wallet envelopes.

`agent_tokens.confirm(actor, receipt, expected_record)` rechecks current owner
access, qualification/policy, receipt expiry/revocation, evidence snapshot, and
original-root identity. The future backend must supply `actor` from verified
application authentication, never request-body identity. This shared database
function is not an end-user authentication mechanism. Operators, viewers and
administrators do not gain creator approval merely through a platform role.
Application epochs remain explicitly blocked.

## Validation and current evidence

The runner accepts no arbitrary database URL or data directory. New runs require
at least 96 MiB free and use a new `/tmp/supa-token-test-<32 hex>` socket-only
cluster. Original migrations remain unchanged. Runtime-role tests grant only
schema usage and confirmation execution, with no direct table or financial writes.

```sh
python3 scripts/validate-agent-token-registry.py \
  --bin-dir artifacts/ecosystem-pg18-binaries/root/usr/lib/postgresql/18/bin \
  --library-dir artifacts/ecosystem-pg18-binaries/root/usr/lib/x86_64-linux-gnu
```

The initial corrected lifecycle run completed 24 checks, including creator/operator/
viewer boundaries, tampering, revocation, expiration, direct-write denial,
concurrent originals, duplicate mint rejection, immutable history and successor
recording. Disk exhaustion interrupted its restart phase. Logs remain under
`artifacts/agent-token-registry/55069f007b894a7aa3457c3580126d75`.
A recovery-only continuation passed six checks against the exact retained binding
snapshots: restart, immediate-stop/crash recovery, receipt reuse rejection,
restricted writes, ordinary administrator DML denial, and unchanged financial
baseline. See that directory's `recovery-validation.json`; this is a continuation,
not a claim that an uninterrupted fresh full run passed.

Recovery-only mode requires the runner's existing stopped cluster identity,
retained snapshots and local metadata. It creates no new cluster and does not reset
fixtures. Failed-run evidence is retained. Compression of the earlier failed
cluster was separately approved, lossless, hash-verified and recorded in its
`compression-manifest.json`; it does not affect the successful recovery target.

Thirty existing agent-identity domain tests also passed using the retained test
executable. No Rust source changed or fresh Rust build was required.

## Remaining integration

Before application exposure, implement trusted verification-receipt production,
authenticated Store/HTTP wiring, and an explicit least-privilege grant review.
Add end-to-end tests for that boundary, including ownership changes, revocation
races, and genuinely independently verified evidence. A live database installation
needs separate approval. No route, runtime grant, token launch, platform SUPA mint,
financial execution or social publishing is enabled by this extension.
