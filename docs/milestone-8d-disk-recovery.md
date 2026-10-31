# Milestone 8D — controlled Rust build-output recovery

The explicitly authorized cleanup completed. No unrestricted `cargo clean`,
recursive directory deletion, primary PostgreSQL startup, schema migration,
credential installation, grant, agent activation, provider call, transaction,
push or deployment occurred. Existing application source and website are unchanged.

## Exact cleanup scope

Only the **3,706 regular generated files** recorded in the pre-deletion manifest
were unlinked, using directory-file-descriptor-relative exact names inside:

- `target/debug/deps`: generated dependencies and hashed test/executable aliases;
  the directory remains present and empty.
- `target/debug/examples`: generated compilation files and hashed executable aliases;
  the directory remains present with its eight named helper programs.

| Removed class | Files |
| --- | ---: |
| Cargo dependency information (`.d`) | 1,122 |
| Rust code-generation objects (`.o`) | 950 |
| Compiler metadata (`.rmeta`, including empty compiler placeholders) | 955 |
| Rust static libraries (`.rlib`) | 520 |
| Compiled Rust shared libraries (`.so`) | 26 |
| Generated hashed test/executable files and aliases | 133 |
| **Total** | **3,706** |

Classification verified regular-file type, Cargo filename patterns and file-format
signatures: ELF objects/executables/shared libraries, archive signatures, Rust
metadata or UTF-8 dependency information. Unexpected nested directories, symlinks,
special files, unrecognized names/types, source files, databases and report/log
formats were excluded or would have stopped execution. No candidate was tracked
by Git. All project source and Cargo registry dependencies remain available for
future rebuilding. No other caches or system files were removed.

Before deletion, the complete manifest recorded every candidate's path, class,
SHA-256, inode/device, size, allocated blocks, mode, owner, modification time and
hard-link count. It also recorded retained executable hashes, disk capacity,
process checks and the protected snapshot digest. Its SHA-256 is:

`788aaf1e2e726e1edab2959e00d17aad0b285768ec706b61b1bd157142354f9e`

An exclusive lock on the existing `target/debug/.cargo-lock` was required and held
through revalidation, cleanup and integrity checks. No active build or candidate
executable/mapping was identified among inspectable processes; inaccessible system
process metadata was not treated as complete host attestation. Every candidate and
protected entry was revalidated before the first unlink. Each deletion was recorded
in the private journal, whose 3,706 entries exactly match the approved manifest.
Only exact regular-file names were unlinked; no path outside the fixed scope could
be supplied. The operation was single-use and rejected changed inventories.

## Storage measurements

All free-space figures below use unprivileged available blocks, excluding ext4
root-reserved space. Filesystem capacity is 249,792,131,072 bytes.

| Measurement | Bytes | Binary units |
| --- | ---: | ---: |
| Initial sample before manifest preparation | 353,292,288 | 336.93 MiB |
| Immediately before unlinking | 237,113,344 | 226.13 MiB |
| After cleanup and integrity verification | 11,313,180,672 | 10.54 GiB |
| Observed available-space increase during cleanup | 11,076,067,328 | 10.32 GiB |
| Manifest estimate of reclaimable regular-file blocks | 11,068,469,248 | 10.31 GiB |

The two scopes originally offered about 10.93 GiB under full removal, but retaining
the eight named helper programs narrowed the deletion to approximately 10.31 GiB.
Hard-linked executable blocks retained elsewhere were excluded from the reclamation
estimate. Observed free-space change differs slightly from inode accounting because
other host filesystem activity can allocate or release blocks concurrently.

The new detailed preservation snapshot occupies approximately 101 MiB, plus the
1.7 MiB cleanup manifest and small journals/probe records. Consequently end-to-end
free-space gain from the initial sample is about
10.21 GiB, rather than the full unlink-period gain.
These newly created audit records are retained privately, not deleted or committed.

The 8C **10 GiB free-space gate is met at this measurement**. Only about
0.54 GiB remains above that conservative threshold.
Recheck before any separately approved primary preflight. Rebuilding the deleted
outputs can consume much of this space again; avoid builds during database
backup/restoration. This is not unlimited operating capacity or cutover approval.

## Preserved executables and safe startup checks

All five core executable paths retain their exact pre-cleanup SHA-256 bytes:

- `target/debug/supa-api`
- `target/debug/supa-runtime`
- `target/debug/supa-migration-admin`
- `target/debug/supa-migration-eleven-admin`
- `target/debug/supa-migration-twelve-admin`

The five hashes also match the preserved 8C executable inventory. In addition,
these eight named helper binaries remain at `target/debug/examples/`:
`access_admin`, `auth_probe`, `controlled_fixture`, `demo`, `devnet_verify`,
`live_eval`, `prepare_epoch` and `review_cutover`. No executable was relocated.
Unlinking hashed aliases changes hard-link counts, not retained file contents.

The migration-twelve executable remains pinned to SHA-256:
`42c5a4dfb860900beb8e66c02199bc63dec2c82b6e934c71125877f4e05a6ac5`.

All **13** executables passed matching **before and after** loader/entry-point
probes from a private empty working/home directory, with a cleared environment,
no database URL, no API credentials, no approval files and no valid operational
arguments. The worker was explicitly disabled. Source review established that
these paths reject invalid arguments/missing configuration before operational work.

| Executables | Expected observed exits |
| --- | --- |
| API and disabled worker | 1: missing configuration, fail closed |
| Three migration-only commands | 2: invalid exact command arguments, fail closed |
| access_admin, auth_probe, controlled_fixture, demo, devnet_verify, live_eval | 1: invalid/unapproved invocation, fail closed |
| prepare_epoch, review_cutover | 2: invalid/unapproved invocation, fail closed |

There were no loader errors, signals, timeouts or unexpected success exits. Hashes
and exit statuses matched across the two probe runs. These checks demonstrate that
the retained executables load and enforce their non-operational guards; they do
**not** claim healthy configured API/worker services or new end-to-end operation.
No privileged helper action, signing, provider request or database connection was
requested. No Rust build/test suite was rerun, which would recreate the removed
artifacts and consume the recovered space.

## Preservation evidence

The comprehensive before/after comparison passed for **322,469 pre-existing
workspace entries** outside the cleanup scope and new private evidence directory.
It covered file content hashes and relevant ownership/mode/path metadata, including:

- All existing source, configuration, migrations and approved frontend files.
- PostgreSQL clusters, backups, private credentials and wallet material.
- Existing reports, historical accounting/evaluation evidence and artifacts.
- Original Git metadata and recovered Git metadata during the cleanup itself.
- All 13 explicitly retained executable files, including helpers inside the scope.

Access times and expected hard-link-count changes were not used as content changes.
The subsequent normal one-file documentation commit intentionally updates recovered
Git metadata; no Git metadata was deleted, replaced or relocated.

The stopped-primary guard passed before and after: **all 26 protected hashes
unchanged**, primary stopped, last SQL-verified schema 8, prepared epoch
`fd2f52e0-33fc-4c0d-893a-7b08b26ab748`, no owner grant or verified custodian, and
historical database still unrecovered. The unresolved **$0.004048 hold** and its
quarantine evidence remain unchanged. There was no primary SQL connection.

## Local evidence and remaining boundaries

Private, ignored evidence directory:
`artifacts/milestone-8d/a0fe6ec3d2fb/`.

- `cleanup-manifest.json`: detailed inventory completed before deletion.
- `protected-before.json`: protected workspace hashes and metadata.
- `deletion-journal.jsonl`: exact manifest-listed removals.
- `cleanup-result.json`: storage measurements and preservation results.
- `probes-before.json` / `probes-after.json`: all 13 executable hashes and safe exits.
- One-off `cleanup.py` / `probe.py`: exact operation and probe implementation;
  no reusable arbitrary-target cleanup command was added to project source.

The capacity blocker is cleared for consideration of a **new, separately approved
primary preflight**. Independent backup storage, encryption-recipient approval and
independent owner/custodian verification remain unresolved. No migration,
provisioning, credential/grant activation, offline agent session or worker scheduling
is authorized by this cleanup. No frontend or Rust implementation was changed.

Only this sanitized report is a tracked change. It follows the one-file Conventional
Commit workflow on `main`; final history and secrets audit results are reported
alongside repository status. No private manifests, database files, logs or other
artifacts are staged or pushed.
