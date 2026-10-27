#!/usr/bin/env python3
"""Enforce the user's main-only, one-file Conventional Commit workflow."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
GIT = [str(ROOT / 'scripts/git-local')]
CONVENTIONAL = re.compile(r'^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-zA-Z0-9_.:/-]+\))?!?: \S.{7,}$')


def git(*args):
    result = subprocess.run(GIT + list(args), cwd=ROOT, capture_output=True)
    if result.returncode:
        raise ValueError('git_policy_read_failed')
    return result.stdout


def validate(branch, paths, subject=None):
    if branch != 'main':
        raise ValueError('commits_require_main')
    if len(paths) != 1:
        raise ValueError('commit_requires_exactly_one_changed_file')
    if subject is not None and not CONVENTIONAL.fullmatch(subject):
        raise ValueError('descriptive_conventional_commit_required')


def staged():
    return [p for p in git('diff', '--cached', '--name-only', '--no-renames', '-z').split(b'\0') if p]


def pre_commit():
    validate(git('symbolic-ref', '--short', 'HEAD').decode().strip(), staged())
    # Never bypass the existing full staged-content/reachable-history secrets gate.
    subprocess.run([sys.executable, str(ROOT / 'scripts/scan-secrets.py')], cwd=ROOT, check=True)


def commit_msg(path):
    text = Path(path).read_text()
    subject = text.splitlines()[0] if text.splitlines() else ''
    validate(git('symbolic-ref', '--short', 'HEAD').decode().strip(), staged(), subject)


def audit():
    failures = []
    commits = git('rev-list', '--reverse', 'main').decode().splitlines()
    for commit in commits:
        paths = [p for p in git('diff-tree', '--root', '--no-commit-id', '--name-only', '--no-renames', '-r', '-z', commit).split(b'\0') if p]
        subject = git('show', '-s', '--format=%s', commit).decode().strip()
        try:
            validate('main', paths, subject)
        except ValueError as error:
            failures.append({'commit': commit, 'changed_files': len(paths), 'rule': str(error)})
    report = {'branch': git('symbolic-ref', '--short', 'HEAD').decode().strip(), 'main_commits': len(commits), 'one_file_commits': len(commits) - len(failures), 'failures': failures, 'passed': not failures}
    if report['branch'] != 'main':
        report['passed'] = False
    print(json.dumps(report, indent=2))
    if not report['passed']:
        raise SystemExit(1)


def self_test():
    count = 0
    validate('main', [b'crates/domain/src/wallet.rs'], 'fix(wallet): validate the full Devnet genesis hash')
    count += 1
    for branch, paths, message in [
        ('feature', [b'a'], 'feat(test): reject work outside main'),
        ('main', [], 'chore(test): reject empty commits'),
        ('main', [b'a', b'b'], 'feat(test): reject bundled files'),
        ('main', [b'a'], 'update file'),
        ('main', [b'a'], 'fix: x'),
    ]:
        try:
            validate(branch, paths, message)
        except ValueError:
            count += 1
        else:
            raise AssertionError('unsafe_workflow_accepted')
    print(json.dumps({'policy_self_tests_passed': count}))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['pre-commit', 'commit-msg', 'pre-push', 'audit', 'self-test'])
    parser.add_argument('message_file', nargs='?')
    args = parser.parse_args()
    try:
        if args.action == 'pre-commit':
            pre_commit()
        elif args.action == 'commit-msg':
            if not args.message_file:
                raise ValueError('message_file_required')
            commit_msg(args.message_file)
        elif args.action == 'pre-push':
            subprocess.run([sys.executable, str(ROOT / 'scripts/scan-secrets.py')], cwd=ROOT, check=True)
            raise ValueError('initial_public_push_requires_explicit_user_approval')
        elif args.action == 'audit':
            audit()
        else:
            self_test()
    except (ValueError, subprocess.CalledProcessError) as error:
        code = str(error) if isinstance(error, ValueError) else 'secrets_scan_failed'
        print(json.dumps({'git_policy_passed': False, 'reason': code}), file=sys.stderr)
        raise SystemExit(1)


if __name__ == '__main__':
    main()
