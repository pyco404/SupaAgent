# Mandatory Git workflow

The user's rule is **one file = one commit = one descriptive Conventional Commit message**. Work directly on `main`, stage each exact path separately, verify exactly one staged file, and never bundle, squash or force-push. These instructions persist in the root `AGENTS.md`.

## Recovered repository and retained history

The original empty read-only `.git` remains untouched. The recovered `.git-recovery` repository remains in place; no metadata directory was deleted, replaced or relocated. Use `./scripts/git-local` from this workspace. The wrapper selects this metadata and working tree consistently across terminals.

The old unpublished 125-file root commit `86c7bc7` is preserved on `archive/pre-one-file-workflow`. Local `main` is rebuilt as a new root and sequential one-file commits, as requested. The archive is historical evidence, is not an ancestor of the new `main`, and must not accompany a future `main` push. This is a local pre-publication history split; no squash, source deletion, metadata replacement, force-push or remote write occurred.

The original archive preserves all source, migrations and evaluation evidence. Only Git workflow instructions, helpers, ignore rules and documentation change in the new tree; Rust runtime code, financial records, private configuration and historical evaluation artifacts remain unchanged. A private before/after SHA-256 manifest verifies preservation without disclosing key fingerprints.

## Creating commits

Prefer the exact-path helper:

```bash
./scripts/commit-one crates/domain/src/wallet.rs 'fix(wallet): validate finalized payment ownership'
```

It requires `main` and an empty starting index, stages only that literal regular-file path, checks that the staged diff names exactly that file, then commits with the supplied subject and a `File:` body reference. If a check fails, it stops; it does not reset staged work, amend history or stage other files.

The equivalent manual sequence is:

```bash
./scripts/git-local add -- crates/domain/src/wallet.rs
./scripts/git-local diff --cached --name-only --no-renames
# Continue only if exactly that one file is listed.
./scripts/git-local commit -m 'fix(wallet): validate finalized payment ownership'
```

Directory/all-file staging, multiple-path staging, squashing and amendments are rejected by the wrapper. Amendments are excluded because they could combine a newly staged file with the previous commit's file. Use a new one-file follow-up commit instead. Never bypass hooks or use force options.

## Enforcement and validation

Versioned `.githooks/pre-commit` and `commit-msg` call `scripts/git-policy.py`. Local `core.hooksPath` points to these hooks; previous hooks remain in the recovered metadata unchanged. A future clone must explicitly configure its hook path because Git does not install repository hooks automatically.

- Pre-commit requires `main`, exactly one changed path with rename detection disabled, and a passing full staged-content/reachable-history secrets scan.
- Commit-message validation requires a descriptive Conventional Commit subject.
- `python3 scripts/git-policy.py audit` verifies every commit reachable from `main` independently, including the root. The retained archive is excluded from this one-file audit, but remains included in the secrets scan.
- `python3 scripts/git-policy.py self-test` checks acceptance of a compliant one-file commit and rejection of wrong-branch, empty, multi-file and malformed-message inputs.
- `python3 scripts/scan-secrets.py --self-test` retains its existing credential-detection tests.

Private dotenv files, keys, encrypted wallet envelopes and local databases/backups are excluded. SQL migration source remains eligible. Existing evidence is preserved verbatim, with each artifact getting its own descriptive commit. The scanner still checks actual local secret values internally, never prints them, and reports path/rule only. It is a targeted local tool, not a substitute for human review or a guarantee against unknown secret formats.

## Push authorization

Initial repository setup and the first public push have **not** been approved. The wrapper blocks push before Git contacts a remote, and the pre-push hook independently requires a fresh secrets scan and blocks the unapproved initial push. No bypass flag was added.

After explicit approval, any authorized implementation of the push step must preserve these rules: scan credentials, wallet material and sensitive artifacts first; push only local `main` directly to `origin/main`; never push the archive, squash, or force-push. Refresh remote state before deciding how to proceed. Do not infer approval from successful local validation.

The local validation report is `artifacts/git-workflow/validation.json`; the full private preservation/audit manifests stay under ignored `.git-recovery` metadata. No Solana RPC, funding, model inference, database operation, publication or deployment is part of this Git-only work.
