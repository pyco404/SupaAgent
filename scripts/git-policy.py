#!/usr/bin/env python3
"""Enforce the user's main-only, one-file Conventional Commit workflow."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
GIT = [str(ROOT / 'scripts/git-local')]
CONVENTIONAL = re.compile(r'^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([a-zA-Z0-9_.:/-]+\))?!?: \S.{7,}$')
OFFICIAL_REMOTE = 'https://github.com/pyco404/SupaAgent'


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


def validate_push_scope(branch, head, approved_head, dirty, fetch_urls, push_urls):
    if branch != 'main' or not approved_head or approved_head != head:
        raise ValueError('push_requires_explicit_approval_for_current_main_head')
    if dirty:
        raise ValueError('push_requires_clean_working_tree')
    if fetch_urls != [OFFICIAL_REMOTE] or push_urls != [OFFICIAL_REMOTE]:
        raise ValueError('push_requires_only_the_official_origin')


def push_check():
    head = git('rev-parse', 'HEAD').decode().strip()
    validate_push_scope(
        git('symbolic-ref', '--short', 'HEAD').decode().strip(), head,
        os.environ.get('SUPA_APPROVED_PUSH_HEAD'), bool(git('status', '--porcelain')),
        git('remote', 'get-url', '--all', 'origin').decode().splitlines(),
        git('remote', 'get-url', '--push', '--all', 'origin').decode().splitlines())
    return head


def validate_push_updates(lines, head):
    if len(lines) != 1:
        raise ValueError('push_requires_exactly_one_main_update')
    fields = lines[0].split()
    if (len(fields) != 4 or fields[0] != 'refs/heads/main'
            or fields[1] != head or fields[2] != 'refs/heads/main'
            or not re.fullmatch(r'[0-9a-f]{40}', fields[3])
            or fields[3] == '0' * 40):
        raise ValueError('push_requires_current_main_to_existing_remote_main')
    return fields[3]


def pre_push(remote, url):
    head = push_check()
    if remote != 'origin' or url != OFFICIAL_REMOTE:
        raise ValueError('push_requires_only_the_official_origin')
    previous = validate_push_updates(sys.stdin.read().splitlines(), head)
    if subprocess.run(GIT + ['merge-base', '--is-ancestor', previous, head],
                      cwd=ROOT, capture_output=True).returncode:
        raise ValueError('remote_main_changed_fetch_and_review_before_push')
    audit()
    subprocess.run([sys.executable, str(ROOT / 'scripts/scan-secrets.py')], cwd=ROOT, check=True)
    if push_check() != head:
        raise ValueError('main_changed_during_push_checks')


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
    head = 'a' * 40
    official = [OFFICIAL_REMOTE]
    validate_push_scope('main', head, head, False, official, official)
    count += 1
    for values in [
        ('main', head, None, False, official, official),
        ('main', head, 'b' * 40, False, official, official),
        ('feature', head, head, False, official, official),
        ('main', head, head, True, official, official),
        ('main', head, head, False, ['https://unexpected.invalid/repo'], official),
        ('main', head, head, False, official, official * 2),
    ]:
        try:
            validate_push_scope(*values)
        except ValueError:
            count += 1
        else:
            raise AssertionError('unsafe_push_scope_accepted')
    update = f'refs/heads/main {head} refs/heads/main ' + 'b' * 40
    validate_push_updates([update], head)
    count += 1
    for lines in [[], [update, update], [update.replace('refs/heads/main', 'refs/heads/archive')],
                  [update.replace(head, 'c' * 40)], [update.replace('b' * 40, '0' * 40)], ['malformed']]:
        try:
            validate_push_updates(lines, head)
        except ValueError:
            count += 1
        else:
            raise AssertionError('unsafe_push_update_accepted')
    print(json.dumps({'policy_self_tests_passed': count}))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['pre-commit', 'commit-msg', 'pre-push', 'push-check', 'audit', 'self-test'])
    parser.add_argument('message_file', nargs='?')
    parser.add_argument('remote_url', nargs='?')
    args = parser.parse_args()
    try:
        if args.action == 'pre-commit':
            pre_commit()
        elif args.action == 'commit-msg':
            if not args.message_file:
                raise ValueError('message_file_required')
            commit_msg(args.message_file)
        elif args.action == 'pre-push':
            pre_push(args.message_file, args.remote_url)
        elif args.action == 'push-check':
            push_check()
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
