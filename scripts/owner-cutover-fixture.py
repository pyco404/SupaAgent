#!/usr/bin/env python3
"""Exercise rendered SQL on a disposable clone with synthetic actors only."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid


def module(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


owner = module('owner', 'owner-provisioning.py')
f = module('fixture', 'cutover-fixture.py')
pg = f.pg


def prepare():
    result = f.prepare()
    c = f.load(result['root'])
    c['plan_sha256'] = f.r.digest((owner.ROOT / 'artifacts/owner-readiness/change-set.json').read_bytes())
    c['identity_before'] = pg.digest(owner.BASE / 'identity.json')
    c['credentials_before'] = pg.digest(owner.BASE / 'credentials.pending.json')
    c['primary_config_before'] = pg.digest(owner.ROOT / 'web/.env.local')
    pg.exclusive_json(Path(c['root']) / 'owner-fixture.json', c)
    return result


def load(path):
    root = f.r.fixture_root(path)
    c = json.loads(pg.private_file(root / 'owner-fixture.json').read_text())
    if c['root'] != str(root) or not c['database'].startswith('supa_cutover_test_'):
        raise ValueError('disposable_owner_fixture_required')
    if c['synthetic_actor_id'] == owner.load()['principal_id']:
        raise ValueError('never_install_real_prepared_owner_in_fixture')
    pg.identity()
    return c


def apply(c, operation, approval, revision, target=None, digest=None):
    # Connection is fixed to our disposable database, even when testing a wrong target.
    if not c['database'].startswith('supa_cutover_test_'):
        raise ValueError('disposable_connection_required')
    root = Path(c['root'])
    path = root / ('review-' + uuid.uuid4().hex + '.sql')
    f.r.private_write(path, owner.render_transition(target or c['database'], c['synthetic_actor_id'], operation).encode())
    pg.command('psql', '-Xq', '--no-password', '-v', 'ON_ERROR_STOP=1',
               '-v', 'approval_id=' + approval, '-v', 'expected_revision=' + str(revision),
               '-v', 'plan_sha256=' + (digest or c['plan_sha256']), '-f', path, database=c['database'])


def activate(c):
    rejected = 0
    for kwargs in [dict(revision=1), dict(revision=0, digest='invalid'),
                   dict(revision=0, target='supa_epoch_dev_' + uuid.UUID(owner.EPOCH).hex)]:
        try:
            apply(c, 'activate_read_only', str(uuid.uuid4()), **kwargs)
        except ValueError:
            rejected += 1
    if rejected != 3:
        raise ValueError('invalid_activation_accepted')
    apply(c, 'activate_read_only', c['activation_approval'], 0)
    try:
        apply(c, 'activate_read_only', c['activation_approval'], 0)
    except ValueError:
        rejected += 1
    if rejected != 4:
        raise ValueError('duplicate_activation_accepted')
    root = Path(c['root'])
    from shlex import split
    api = pg.private_file(root / 'api.env').read_bytes()
    site = pg.private_file(root / 'site.env').read_bytes()
    values = {line.split('=', 1)[0]: split(line.split('=', 1)[1])[0] for line in api.decode().splitlines()}
    values.update(SUPA_EPOCH_MODE='active_read_only', SUPA_EPOCH_READONLY_PREVIEW='false', SUPA_EPOCH_APPROVAL_ID=c['activation_approval'])
    records = [f.r.install_fixture(root, 'api.env', owner.dotenv(values), f.r.digest(api)),
               f.r.install_fixture(root, 'site.env', site + b'# synthetic owner cutover fixture\n', f.r.digest(site))]
    pg.exclusive_json(root / 'owner-rollback.json', records)
    actor = pg.sql('SELECT approved_by FROM epoch_transitions', c['database'])
    if actor != c['synthetic_actor_id']:
        raise ValueError('audit_actor_mismatch')
    return {'rendered_activation_verified': True, 'unsafe_or_duplicate_requests_rejected': rejected, 'synthetic_owner_only': True}


def rollback(c):
    apply(c, 'rollback', c['rollback_approval'], 1)
    duplicate = False
    try:
        apply(c, 'rollback', c['rollback_approval'], 1)
    except ValueError:
        duplicate = True
    if not duplicate:
        raise ValueError('duplicate_rollback_accepted')
    root = Path(c['root'])
    records = json.loads(pg.private_file(root / 'owner-rollback.json').read_text())
    if not all(f.r.rollback_fixture(root, r) for r in records):
        raise ValueError('configuration_rollback_failed')
    if pg.sql('SELECT value FROM cutover_probe WHERE id=1', c['database']) != 'Non-agent fixture data must survive rollback':
        raise ValueError('data_not_preserved')
    if int(pg.sql('SELECT count(*) FROM epoch_transitions', c['database'])) != 2:
        raise ValueError('audit_not_preserved')
    for table in ['agents', 'agent_access', 'access_audit', 'schedule_authorizations', 'live_budgets', 'live_sessions', 'inference_calls', 'worker_health', 'simulated_ledger']:
        if int(pg.sql('SELECT count(*) FROM ' + table, c['database'])):
            raise ValueError('unexpected_business_state')
    if pg.fingerprint(f.e.load()['database']) != c['source_fingerprint']:
        raise ValueError('real_candidate_changed')
    for path, key in [(owner.BASE / 'identity.json', 'identity_before'),
                      (owner.BASE / 'credentials.pending.json', 'credentials_before'),
                      (owner.ROOT / 'web/.env.local', 'primary_config_before')]:
        if pg.digest(path) != c[key]:
            raise ValueError('real_identity_or_primary_configuration_changed')
    return {'prepared_again': True, 'duplicate_rollback_rejected': True, 'audit_entries': 2,
            'data_preserved': True, 'private_configuration_bytes_restored': True,
            'real_candidate_unchanged': True, 'pending_owner_unchanged': True, 'primary_frontend_unchanged': True}


if __name__ == '__main__':
    try:
        os.umask(0o077)
        if sys.argv[1:] == ['prepare']:
            result = prepare()
        elif len(sys.argv) == 3 and sys.argv[1] in ['activate', 'rollback', 'backup-verify']:
            c = load(sys.argv[2])
            result = {'activate': activate, 'rollback': rollback, 'backup-verify': f.backup_verify}[sys.argv[1]](c)
        else:
            raise ValueError('fixture_only')
        print(json.dumps(result))
    except (ValueError, OSError, KeyError, subprocess.SubprocessError):
        print(json.dumps({'error': 'owner_fixture_failed_closed', 'real_owner_granted': False, 'real_epoch_activated': False}), file=sys.stderr)
        sys.exit(2)
