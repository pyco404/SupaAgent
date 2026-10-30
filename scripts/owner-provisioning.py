#!/usr/bin/env python3
"""Prepare dormant local identity material and render review SQL. Never grants or activates."""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import uuid

spec = importlib.util.spec_from_file_location('review', Path(__file__).with_name('cutover-review.py'))
review = importlib.util.module_from_spec(spec)
spec.loader.exec_module(review)
ROOT = review.ROOT
EPOCH = review.EXPECTED
BASE = ROOT / 'postgres-data/owner-provisioning'


def canonical(value):
    parsed = uuid.UUID(value)
    if str(parsed) != value or parsed.int == 0 or value in [EPOCH, review.epoch.pg.ARCHIVED]:
        raise ValueError('distinct_canonical_principal_required')
    return value


def safe_file(path):
    review.epoch.pg.private_file(path)
    if path.stat().st_nlink != 1:
        raise ValueError('single_link_private_file_required')
    return path


def load():
    review.epoch.pg.private_directory(BASE)
    identity = json.loads(safe_file(BASE / 'identity.json').read_text())
    canonical(identity['principal_id'])
    if identity['epoch_id'] != EPOCH or identity['display_name'] != 'KI Daniel' or identity['intended_role'] != 'owner' or identity['state'] != 'prepared_ungranted' or identity['permissions_granted'] is not False or identity['identity_binding_approved'] is not False or identity['authentication_enabled'] is not False:
        raise ValueError('prepared_identity_required')
    return identity


def credentials():
    value = json.loads(safe_file(BASE / 'credentials.pending.json').read_text())
    if set(value) != {'username', 'password', 'operator_token', 'service_token'} or value['username'] != 'ki-daniel':
        raise ValueError('pending_credentials_required')
    values = [value[name] for name in ['password', 'operator_token', 'service_token']]
    if any(not isinstance(v, str) or not re.fullmatch(r'[A-Za-z0-9_-]{43,86}', v) for v in values) or len(set(values)) != 3:
        raise ValueError('distinct_strong_pending_credentials_required')
    return value


def ignored():
    for name in ['identity.json', 'credentials.pending.json', 'prepare.lock']:
        path = str(BASE / name)
        if subprocess.run([str(ROOT / 'scripts/git-local'), 'check-ignore', '-q', '--', path], capture_output=True).returncode:
            raise ValueError('private_identity_must_be_ignored')
        result = subprocess.run([str(ROOT / 'scripts/git-local'), 'ls-files', '--', path], capture_output=True, check=True)
        if result.stdout:
            raise ValueError('private_identity_must_not_be_tracked')


def prepare():
    ignored()
    review.epoch.pg.private_directory(BASE)
    fd = os.open(BASE / 'prepare.lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'r+') as lock:
        safe_file(BASE / 'prepare.lock')
        fcntl.flock(lock, fcntl.LOCK_EX)
        if not (BASE / 'identity.json').exists() and not (BASE / 'identity.json').is_symlink():
            if (BASE / 'credentials.pending.json').exists() or (BASE / 'credentials.pending.json').is_symlink():
                raise ValueError('missing_identity_requires_explicit_recovery')
            identity = {'epoch_id': EPOCH, 'principal_id': str(uuid.uuid4()), 'display_name': 'KI Daniel',
                        'intended_role': 'owner', 'state': 'prepared_ungranted', 'permissions_granted': False,
                        'identity_binding_approved': False, 'authentication_enabled': False}
            review.epoch.pg.exclusive_json(BASE / 'identity.json', identity)
        identity = load()
        if not (BASE / 'credentials.pending.json').exists() and not (BASE / 'credentials.pending.json').is_symlink():
            review.epoch.pg.exclusive_json(BASE / 'credentials.pending.json', {
                'username': 'ki-daniel', 'password': secrets.token_urlsafe(36),
                'operator_token': secrets.token_urlsafe(48), 'service_token': secrets.token_urlsafe(48)})
        credentials()
        # Persist directory entries. Nothing is copied into live environment files.
        directory = os.open(BASE, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
        return public_identity(identity)


def dotenv(values):
    # Node parseEnv preserves JSON backslash escapes; single-quote JSON rosters instead.
    if any(not re.fullmatch(r'[A-Z][A-Z0-9_]*', k) or not isinstance(v, str)
           or any(c in v for c in ["'", "\n", "\r", "\0"]) for k, v in values.items()):
        raise ValueError('safe_dotenv_values_required')
    return ''.join(k + "='" + v + "'\n" for k, v in values.items()).encode()


def public_identity(identity):
    return {'epoch_id': identity['epoch_id'], 'principal_id': identity['principal_id'], 'intended_role': 'owner',
            'state': 'prepared_ungranted', 'permissions_granted': False, 'authentication_enabled': False,
            'private_credentials': 'postgres-data/owner-provisioning/credentials.pending.json',
            'identity_binding_approved': False}


def plan():
    identity = load()
    credentials()
    candidate = review.plan()
    # Do not rewrite the previous milestone's unassigned roster or historical artifacts.
    candidate['named_operators'] = [public_identity(identity)]
    candidate['owner_binding_requires_approval'] = True
    candidate['owner_grant'] = {
        'mechanism': 'explicit server-only credential roster; no principal table exists',
        'api_mapping': {'id': identity['principal_id'], 'role': 'owner', 'token': '<pending_operator_token>'},
        'site_mapping': {'username': 'ki-daniel', 'password': '<pending_password>', 'api_token': '<same_pending_operator_token>'},
        'load_into_services': False, 'agent_ownership_grants': [],
        'legacy_shared_owner_mapping': 'unset SUPA_OPERATOR_ID and single-account fallback',
        'public_service_token': '<distinct_pending_service_token>'}
    candidate['blockers'] = ['explicit approval of owner UUID binding and credential installation',
        'approved SELECT-only database role and local authentication setup',
        'approved migrations 9 and 10 on the real candidate', 'independent trusted approval authority and database-bound executor', 'approval-bound transition parameters and private configuration snapshots',
        'operational backup profile and explicit acceptance of same-disk-only backups',
        'fresh explicit approval for activation and primary frontend repointing']
    return candidate


def render_transition(database, principal, operation):
    """No database I/O: exact psql transaction, independently approved plan bytes and revision."""
    canonical(principal)
    real_database = 'supa_epoch_dev_' + uuid.UUID(EPOCH).hex
    if database != real_database and not re.fullmatch(r'supa_cutover_test_[a-z0-9_]+', database):
        raise ValueError('review_target_required')
    if operation not in ['activate_read_only', 'rollback']:
        raise ValueError('review_operation_required')
    return f'''-- REVIEW ONLY. Requires migration 10 and an independent immutable approval.
-- Supply approval_id, expected_revision and the exact approved plan_text; never a caller hash alone.
\\set ON_ERROR_STOP on
BEGIN READ WRITE;
SET LOCAL statement_timeout = '10s';
SET LOCAL lock_timeout = '5s';
DO $target$ BEGIN IF current_database()<>'{database}' THEN RAISE EXCEPTION 'wrong_epoch_database'; END IF; END; $target$;
SELECT epoch_guard.execute_transition(
    :'approval_id'::uuid, '{principal}'::uuid, 'owner', '{EPOCH}'::uuid,
    :'expected_revision'::bigint, '{operation}', :'plan_text');
COMMIT;
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['prepare', 'status', 'plan', 'render-activation', 'render-rollback'])
    args = parser.parse_args()
    os.umask(0o077)
    if args.command == 'prepare':
        result = prepare()
    elif args.command == 'status':
        result = public_identity(load())
    elif args.command == 'plan':
        result = plan()
    else:
        candidate = plan()
        print(render_transition(candidate['database'], candidate['named_operators'][0]['principal_id'],
                                'activate_read_only' if args.command == 'render-activation' else 'rollback'))
        return
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError):
        print(json.dumps({'error': 'owner_preparation_failed_closed', 'permissions_granted': False, 'activated': False}), file=sys.stderr)
        sys.exit(2)
