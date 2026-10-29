#!/usr/bin/env python3
"""Prepare and back up an inactive epoch. No activation, imports, transfers, or inference."""
import argparse
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid
from urllib.parse import urlencode

spec = importlib.util.spec_from_file_location('infrastructure', Path(__file__).with_name('persistent-postgres.py'))
pg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pg)
ROOT = pg.ROOT
CONFIG = ROOT / 'postgres-data' / 'development-epoch.json'
POLICY = ROOT / 'postgres-data' / 'epoch-backup-policy.json'
PENDING = ROOT / 'postgres-data' / 'development-epoch-intent.json'


def url(database):
    return 'postgresql:///' + database + '?' + urlencode({'host': str(pg.BASE / 'socket'), 'port': pg.PORT, 'user': pg.getpass.getuser()})


def load():
    config = json.loads(pg.private_file(CONFIG).read_text())
    id = str(uuid.UUID(config['id']))
    if id != config['id'] or config['database'] != 'supa_epoch_dev_' + uuid.UUID(id).hex or config['state'] != 'prepared':
        raise ValueError('prepared_epoch_configuration_required')
    pg.identity()
    record = json.loads(pg.sql("SELECT to_jsonb(e)-'singleton'-'created_at' FROM application_epoch e", config['database']))
    if record != config['epoch'] or record['id'] != id or record['state'] != 'prepared':
        raise ValueError('epoch_database_identity_mismatch')
    if pg.sql('SHOW default_transaction_read_only', config['database']) != 'on':
        raise ValueError('prepared_database_must_remain_read_only')
    return config


def backup_policy(destination=None):
    if destination is not None:
        path = Path(destination)
        if not path.is_absolute() or '..' in path.parts or path.is_relative_to(ROOT) or path.is_relative_to('/tmp'):
            raise ValueError('independent_absolute_local_destination_required')
        destination = str(path)
    return {'strategy': 'independent_local_mount', 'destination': destination,
            'transfer_authorized': False, 'upload_enabled': False,
            'approval_required_before_copy': True, 'encryption_decision_required': True}


def prepare():
    pg.identity()
    if CONFIG.exists() or CONFIG.is_symlink() or PENDING.exists() or PENDING.is_symlink():
        raise ValueError('existing_epoch_preserved_no_replacement')
    id, namespace = uuid.uuid4(), uuid.uuid4()
    database = 'supa_epoch_dev_' + id.hex
    pg.exclusive_json(PENDING, {'id': str(id), 'database': database, 'purpose': 'inactive_development_preparation'})
    pg.command('createdb', '--template=template0', database)
    env = {k: os.environ[k] for k in ['PATH', 'HOME', 'LANG', 'CARGO_HOME', 'RUSTUP_HOME'] if k in os.environ}
    env.update(DATABASE_URL=url(database), SUPA_PREPARE_EPOCH_ID=str(id), SUPA_ACCOUNTING_NAMESPACE=str(namespace))
    r = subprocess.run(['cargo', 'run', '-p', 'supa-api', '--example', 'prepare_epoch', '--locked', '--offline', '--', '--prepare-development-epoch'],
                       cwd=ROOT, env=env, capture_output=True, timeout=300)
    if r.returncode:
        raise ValueError('epoch_preparation_failed_candidate_preserved')
    record = json.loads(pg.sql("SELECT to_jsonb(e)-'singleton'-'created_at' FROM application_epoch e", database))
    # Freeze only this new database after preparation, never the infrastructure/test databases.
    pg.sql(f'ALTER DATABASE "{database}" SET default_transaction_read_only=on', 'postgres')
    config = {'id': str(id), 'database': database, 'state': 'prepared', 'epoch': record,
              'historical_recovered': False, 'activation_approved': False,
              'live_inference': False, 'continuous_scheduling': False, 'live_budgets_provisioned': 0}
    pg.exclusive_json(CONFIG, config)
    if not POLICY.exists():
        pg.exclusive_json(POLICY, backup_policy())
    return config


def backup():
    config = load()
    database = config['database']
    before = pg.fingerprint(database)
    folder = ROOT / 'postgres-data' / 'epoch-backups' / str(uuid.uuid4())
    pg.private_directory(folder)
    pg.command('pg_dump', '--format=custom', '--file', folder / 'database.dump', database=database)
    if pg.fingerprint(database) != before:
        raise ValueError('source_changed_backup_unverified')
    pg.exclusive_json(folder / 'manifest.json', {'epoch_id': config['id'], 'state': 'prepared',
                     'database_sha256': pg.digest(folder / 'database.dump'), 'state_sha256': before,
                     'transfer_authorized': False, 'historical_recovered': False})
    return {'backup_id': folder.name, 'epoch_id': config['id'], 'uploaded': False}


def verify(id):
    if str(uuid.UUID(id)) != id:
        raise ValueError('canonical_backup_uuid_required')
    config = load()
    folder = ROOT / 'postgres-data' / 'epoch-backups' / id
    manifest = json.loads(pg.private_file(folder / 'manifest.json').read_text())
    if manifest['epoch_id'] != config['id'] or manifest['state'] != 'prepared' or pg.digest(folder / 'database.dump') != manifest['database_sha256']:
        raise ValueError('backup_identity_or_checksum_mismatch')
    destination = 'supa_epoch_restore_test_' + uuid.uuid4().hex
    pg.command('createdb', '--template=template0', destination)
    pg.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', destination, folder / 'database.dump')
    if pg.fingerprint(destination) != manifest['state_sha256']:
        raise ValueError('restored_epoch_state_mismatch')
    pg.sql(f'ALTER DATABASE "{destination}" SET default_transaction_read_only=on', 'postgres')
    result = {'epoch_id': config['id'], 'restored_database': destination, 'verified': True, 'read_only': True, 'activated': False}
    pg.exclusive_json(folder / ('verified-' + uuid.uuid4().hex + '.json'), result)
    return result


def main():
    os.umask(0o077)
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['prepare', 'status', 'backup', 'verify', 'configure-backup'])
    p.add_argument('--confirm-inactive-development', action='store_true')
    p.add_argument('--backup-id')
    p.add_argument('--destination')
    a = p.parse_args()
    pg.private_directory(ROOT / 'postgres-data')
    fd = os.open(ROOT / 'postgres-data' / 'epoch-operations.lock', os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if a.action == 'prepare':
            if not a.confirm_inactive_development:
                raise ValueError('explicit_inactive_development_confirmation_required')
            result = prepare()
        elif a.action == 'status':
            result = load()
        elif a.action == 'backup':
            result = backup()
        elif a.action == 'verify':
            result = verify(a.backup_id or '')
        else:
            load()
            if not a.destination:
                raise ValueError('independent_destination_required')
            result = backup_policy(a.destination)
            # Configure only; append an immutable proposal rather than replacing policy or copying data.
            pg.exclusive_json(ROOT / 'postgres-data' / ('backup-proposal-' + uuid.uuid4().hex + '.json'), result)
        print(json.dumps(result))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, subprocess.SubprocessError):
        print(json.dumps({'error': 'epoch_operation_failed_closed', 'activated': False}), file=sys.stderr)
        sys.exit(2)
