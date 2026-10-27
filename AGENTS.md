# Supa Agent repository instructions

These Git rules are mandatory for all development in this workspace.

- Work directly on `main` through `./scripts/git-local`; the original `.git` is preserved and unusable.
- One file per commit. Every file gets its own descriptive Conventional Commit message.
- Stage only one explicitly named, exact file path. Never use directory-wide or all-files staging.
- Before every commit, verify `./scripts/git-local diff --cached --name-only --no-renames` contains exactly one file.
- Prefer `./scripts/commit-one <exact-relative-file-path> '<type>(<scope>): <description>'`; it checks the branch, empty starting index, exact staged file, and commit hooks.
- Never bundle files, squash commits, force-push, or bypass hooks.
- Run `python3 scripts/git-policy.py audit` to verify every commit on `main` changes exactly one file.
- Scan for secrets before commits and pushes using `python3 scripts/scan-secrets.py`. Keep private dotenv files, keys, wallet envelopes, database files and sensitive artifacts untracked.
- Preserve existing project files, migrations, database records and historical evaluation evidence. The old bundled snapshot is retained on `archive/pre-one-file-workflow`; it is historical and must not be pushed as part of `main`.
- Initial public push has NOT been approved. Do not push until the user explicitly approves setup and the first push. Subsequent authorized pushes must target `origin/main` directly, pass scanning and use no force option.
- Do not delete, replace or relocate existing Git metadata without explicit approval. Normal commits/configuration/ref updates inside the recovered repository are permitted by the requested workflow.

Existing runtime safety boundaries remain in force: no funding, real-money transactions, Mainnet operations, trading, X publishing, deployments, or paid inference without the applicable explicit authorization. Autonomous signing remains disabled.
