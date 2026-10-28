#!/usr/bin/env python3
"""Redacted local Git gate: staged blobs, reachable history, known local secrets and key formats.

No third-party scanner is installed. This targeted scanner is a gate, not proof that
arbitrary unknown secrets cannot exist. Findings contain paths/rules only.
"""
import argparse
import base64
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
GIT = [str(ROOT / 'scripts/git-local')]
PATTERNS = {
    'openai_key': re.compile(rb'\bsk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{24,}'),
    'github_token': re.compile(rb'\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})'),
    'aws_access_key': re.compile(rb'\b(?:AKIA|ASIA)[A-Z0-9]{16}\b'),
    'private_key_pem': re.compile(rb'-----BEGIN (?:RSA |EC |OPENSSH |ENCRYPTED )?PRIVATE KEY-----'),
    'credential_url': re.compile(rb'(?:postgres(?:ql)?|https?)://[^\s:/]+:([^\s@/]+)@'),
    'secret_assignment': re.compile(rb'(?i)(?:api[_-]?key|access[_-]?token|private[_-]?key|master[_-]?key|password|secret)\s*["\x27]?\s*[:=]\s*["\x27]([A-Za-z0-9_+/=-]{24,})'),
}
PLACEHOLDERS = {b'change-me', b'your-local-password', b'password', b'local-password', b'your-password'}
SECRET_NAME = re.compile(r'(?i)(api.?key|secret|password|private.?key|api.?token|access.?token|database.?url)$')

def git(*args):
    result = subprocess.run(GIT + list(args), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise RuntimeError('git_scan_read_failed')
    return result.stdout

def entropy(value):
    return -sum((value.count(c) / len(value)) * math.log2(value.count(c) / len(value)) for c in set(value))

def rules(data, known):
    found = set()
    for name, pattern in PATTERNS.items():
        for match in pattern.finditer(data):
            if name == 'credential_url' and match[1] in PLACEHOLDERS:
                continue
            if name == 'secret_assignment' and entropy(match[1]) < 4.0:
                continue
            found.add(name)
    if any(value in data for value in known):
        found.add('known_local_secret')
    # Solana CLI keypair arrays (64 u8 integers), regardless of filename.
    try:
        decoded = json.loads(data)
        if isinstance(decoded, list) and len(decoded) == 64 and all(type(x) is int and 0 <= x <= 255 for x in decoded):
            found.add('solana_private_key_array')
        if isinstance(decoded, dict) and {'ciphertext', 'nonce', 'wallet_id'} <= decoded.keys():
            found.add('encrypted_wallet_envelope')
    except (ValueError, UnicodeDecodeError):
        pass
    return sorted(found)

def protected(path):
    parts = Path(path).parts
    return any(x in {'.git', '.git-recovery', '.supa-secrets', '.supa-vault', '.aws', '.codex', '.agents', 'pgdata', 'postgres-data', 'node_modules', '.next', '.browser-profile', 'test-results', 'playwright-report'} for x in parts) or any(x.startswith('.env') and x != '.env.example' for x in parts) or Path(path).suffix in {'.pem', '.key', '.p12', '.pfx', '.db', '.sqlite', '.sqlite3', '.dump', '.backup'} or path.endswith('.sql.gz')

def local_secrets():
    known = []
    values = dict(os.environ)
    # Read only for exact-match detection; never echo values or copy them into reports.
    for path in [*ROOT.glob('.env*'), *ROOT.joinpath('web').glob('.env*')]:
        if not path.is_file() or path.name == '.env.example':
            continue
        for line in path.read_text().splitlines():
            match = re.match(r'(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)', line)
            if match:
                values[match[1]] = match[2].strip().strip('"\x27')
    for key, value in values.items():
        if SECRET_NAME.search(key) and len(value) >= 12:
            match = re.search(r'://[^:/]+:([^@/]+)@', value)
            # Passwordless local database addresses are configuration, not secret values.
            if 'DATABASE' not in key.upper() or match:
                known.append(value.encode())
            if match and len(match[1]) >= 12 and match[1].encode() not in PLACEHOLDERS:
                known.append(match[1].encode())
    custody=ROOT/'.supa-secrets'
    if custody.is_dir():
        for path in custody.iterdir():
            if path.is_file() and not path.is_symlink() and path.stat().st_size==32:
                raw=path.read_bytes()
                known.extend([raw,base64.b64encode(raw),raw.hex().encode()])
    return known

def scan():
    known = local_secrets()
    findings = []
    staged = git('ls-files', '--stage', '-z').split(b'\0')
    blob_paths = {}
    for entry in staged:
        if not entry:
            continue
        metadata, path = entry.split(b'\t', 1)
        path = path.decode()
        mode, oid, stage = metadata.decode().split()
        if protected(path) or mode not in {'100644', '100755'} or stage != '0':
            findings.append({'path': path, 'rule': 'protected_path_or_nonregular_file'})
        blob_paths.setdefault(oid, path)
    # All reachable commit trees, including historical versions of deleted/renamed files.
    commits = git('rev-list', '--all').decode().splitlines()
    history_entries = 0
    for commit in commits:
        for entry in git('ls-tree', '-rz', commit).split(b'\0'):
            if not entry:
                continue
            metadata, path = entry.split(b'\t', 1)
            mode, kind, oid = metadata.decode().split()
            path = path.decode()
            history_entries += 1
            if protected(path) or kind != 'blob' or mode not in {'100644', '100755'}:
                findings.append({'path': path, 'rule': 'historical_protected_path_or_nonregular_file'})
            if kind == 'blob':
                blob_paths.setdefault(oid, path)
    for oid, path in blob_paths.items():
        for rule in rules(git('cat-file', 'blob', oid), known):
            findings.append({'path': path, 'rule': rule})
    # Verify local sensitive paths remain ignored even if they do not exist yet.
    ignored = ['.env', '.env.eval', '.env.devnet', '.env.wallet', '.supa-secrets/devnet-master', '.supa-vault/devnet/wallet.json', '.git-recovery/config', 'local.db', 'local.sqlite3', 'data.dump', 'pgdata/PG_VERSION', 'web/.env.local', 'web/.browser-profile/Default/Cookies', 'web/.next/server/private.json', 'web/test-results/trace.zip', 'web/node_modules/package/index.js']
    for path in ignored:
        result = subprocess.run(GIT + ['check-ignore', '-q', '--no-index', path], cwd=ROOT)
        if result.returncode:
            findings.append({'path': path, 'rule': 'sensitive_path_not_ignored'})
    return {'scanner': 'local-pattern-and-known-secret-gate', 'staged_entries': sum(bool(e) for e in staged), 'reachable_commits': len(commits), 'history_tree_entries': history_entries, 'unique_blobs_scanned': len(blob_paths), 'known_local_secret_matching': True, 'sensitive_ignore_checks': len(ignored), 'findings': findings, 'passed': not findings, 'limitations': 'Targeted local scan; no third-party scanner installed; unknown secret formats and semantic disclosure require human review.'}

def self_test():
    assert rules(b'sk-proj-' + b'A' * 40, []) == ['openai_key']
    assert 'known_local_secret' in rules(b'prefix' + b'private-synthetic-value', [b'private-synthetic-value'])
    assert 'solana_private_key_array' in rules(json.dumps([1] * 64).encode(), [])
    assert 'private_key_pem' in rules(b'-----BEGIN ' + b'PRIVATE KEY-----', [])
    assert 'credential_url' in rules(b''.join([b'postgresql://', b'user', b':', b'actual-secret', b'@localhost/db']), [])
    assert not rules(b'postgresql://supa:your-local-password@localhost/db', [])
    assert protected('.env.eval') and protected('.supa-vault/devnet/x.json') and protected('local.db') and protected('backup.sql.gz') and not protected('.env.example') and not protected('migrations/0004_wallet.sql')
    assert protected('web/.env.local') and protected('web/.browser-profile/Default/Cookies') and protected('web/.next/server/private.json') and protected('web/test-results/trace.zip') and not protected('web/tests/fixtures.ts')
    print(json.dumps({'self_tests_passed': 8}))

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    try:
        report = scan()
    except Exception:
        print(json.dumps({'passed': False, 'error': 'scanner_failed_without_disclosing_private_values'}))
        raise SystemExit(1)
    output = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.write_text(output)
    print(output, end='')
    if not report['passed']:
        raise SystemExit(1)

if __name__ == '__main__':
    main()
