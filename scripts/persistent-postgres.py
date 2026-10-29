#!/usr/bin/env python3
"""Owned, socket-only PostgreSQL 16 validation infrastructure. Never application adoption."""
import argparse
import fcntl
import getpass
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / 'postgres-data' / 'infrastructure-v1'
BIN = Path('/usr/lib/postgresql/16/bin')
PORT = 55440
DATABASE = 'supa_infrastructure_test'
ARCHIVED = 'ad91e475-f1da-495a-8247-139643d2d8b1'
IDENTITY = {'purpose': 'nonproduction_validation_only', 'historical_database': 'unrecovered',
            'archived_budget_uuid': ARCHIVED, 'archived_ceiling_microusd': 100000,
            'archived_committed_evidence_microusd': 10299, 'unresolved_hold_microusd': 4048,
            'historical_import_authorized': False, 'application_adoption_authorized': False}


def private_directory(path):
    # Walk without following symlinks, including the ignored parent directory.
    if path == ROOT:
        return
    private_directory(path.parent)
    if not path.exists() and not path.is_symlink():
        path.mkdir(mode=0o700)
    info = path.lstat()
    if path.is_symlink() or not path.is_dir() or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise ValueError('private_owned_directory_required')


def exclusive_json(path, value):
    with open(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), 'w') as f:
        json.dump(value, f, indent=2)
        f.write('\n')
        f.flush()
        os.fsync(f.fileno())


def private_file(path):
    info = path.lstat()
    if path.is_symlink() or not path.is_file() or info.st_uid != os.getuid() or info.st_mode & 0o077:
        raise ValueError('private_owned_file_required')
    return path


def environment(database='postgres'):
    return {'PATH': '/usr/bin:/bin', 'HOME': str(Path.home()), 'LANG': 'C.UTF-8',
            'PGHOST': str(BASE / 'socket'), 'PGPORT': str(PORT), 'PGUSER': getpass.getuser(),
            'PGDATABASE': database, 'PGCONNECT_TIMEOUT': '5', 'PGOPTIONS': '-c timezone=UTC',
            'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null'}


def command(binary, *args, database='postgres', timeout=90):
    r = subprocess.run([str(BIN / binary), *map(str, args)], env=environment(database),
                       capture_output=True, text=True, timeout=timeout)
    if r.returncode:
        # Never emit raw SQL, private rows, subprocess diagnostics or environment.
        raise ValueError('postgres_operation_failed_' + binary)
    return r.stdout.strip()


def sql(statement, database=DATABASE):
    return command('psql', '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1', '-c', statement,
                   database=database)


def identity():
    private_directory(BASE)
    value = json.loads(private_file(BASE / 'identity.json').read_text())
    if value != IDENTITY or (BASE / 'data' / 'PG_VERSION').read_text().strip() != '16':
        raise ValueError('owned_infrastructure_identity_mismatch')
    # Verify server identity on every connected operation; never touch another cluster.
    if Path(sql('SHOW data_directory', 'postgres')).resolve() != (BASE / 'data').resolve():
        raise ValueError('server_data_directory_mismatch')
    if sql('SHOW listen_addresses', 'postgres') != '':
        raise ValueError('network_listener_must_remain_disabled')
    for setting in ['fsync', 'synchronous_commit', 'full_page_writes', 'data_checksums']:
        if sql('SHOW ' + setting, 'postgres') != 'on':
            raise ValueError('durability_settings_must_remain_enabled')


def start():
    value = json.loads(private_file(BASE / 'identity.json').read_text())
    if value != IDENTITY:
        raise ValueError('owned_infrastructure_identity_mismatch')
    status = subprocess.run([str(BIN / 'pg_ctl'), '-D', str(BASE / 'data'), 'status'],
                            env=environment(), capture_output=True)
    if status.returncode:
        command('pg_ctl', '-D', BASE / 'data', '-l', BASE / 'server.log', '-w', 'start')
    identity()


def prepare():
    private_directory(BASE)
    # Exclude protection is a prerequisite, not a best-effort cleanup after initdb.
    ignored = subprocess.run([str(ROOT / 'scripts/git-local'), 'check-ignore', '-q', '--',
                              str(BASE / 'data/PG_VERSION')], capture_output=True)
    if ignored.returncode or any(BASE.iterdir()):
        raise ValueError('new_ignored_empty_directory_required')
    command('initdb', '-D', BASE / 'data', '--auth-local=peer', '--auth-host=reject',
            '--encoding=UTF8', '--locale=C.UTF-8', '--data-checksums')
    private_directory(BASE / 'socket')
    config = BASE / 'data' / 'postgresql.conf'
    with config.open('a') as f:
        f.write("\n# Dedicated nonproduction infrastructure; no network listener.\n")
        f.write(f"listen_addresses = ''\nport = {PORT}\nunix_socket_directories = '{BASE / 'socket'}'\n")
        f.write("unix_socket_permissions = 0700\nfsync = on\nsynchronous_commit = on\nfull_page_writes = on\n")
    exclusive_json(BASE / 'identity.json', IDENTITY)
    start()
    command('createdb', '--template=template0', DATABASE)
    sql("COMMENT ON DATABASE supa_infrastructure_test IS 'NEW NONPRODUCTION VALIDATION; HISTORICAL DATABASE UNRECOVERED'", 'postgres')


def fingerprint(database):
    # One consistent transaction; include every non-system schema, not just public.
    tables = json.loads(sql("SELECT coalesce(json_agg(json_build_array(schemaname,tablename) ORDER BY schemaname,tablename),'[]') FROM pg_tables WHERE schemaname NOT LIKE 'pg_%' AND schemaname <> 'information_schema'", database))
    statements = ["BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY"]
    for schema, name in tables:
        qualified = '.'.join('"' + n.replace('"', '""') + '"' for n in [schema, name])
        statements.append(f"SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]') FROM {qualified} t")
    statements += ["SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY table_schema,table_name,ordinal_position),'[]') FROM (SELECT table_schema,table_name,column_name,ordinal_position,data_type,is_nullable,column_default FROM information_schema.columns WHERE table_schema NOT LIKE 'pg_%' AND table_schema <> 'information_schema') c",
                   "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY schema,name,definition),'[]') FROM (SELECT n.nspname AS schema,conname AS name,pg_get_constraintdef(c.oid) AS definition FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname <> 'information_schema') c",
                   "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY schemaname,indexname),'[]') FROM (SELECT schemaname,indexname,indexdef FROM pg_indexes WHERE schemaname NOT LIKE 'pg_%' AND schemaname <> 'information_schema') c",
                   "SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY schemaname,sequencename),'[]') FROM (SELECT schemaname,sequencename,start_value,min_value,max_value,increment_by,cycle,cache_size,last_value FROM pg_sequences WHERE schemaname NOT LIKE 'pg_%' AND schemaname <> 'information_schema') c", 'COMMIT']
    return hashlib.sha256((json.dumps(tables) + sql(';'.join(statements), database)).encode()).hexdigest()


def digest(path):
    return hashlib.sha256(private_file(path).read_bytes()).hexdigest()


def backup():
    identity()
    before = fingerprint(DATABASE)
    folder = BASE / 'backups' / str(uuid.uuid4())
    private_directory(folder)
    command('pg_dump', '--format=custom', '--file', folder / 'database.dump', database=DATABASE)
    command('pg_dumpall', '--globals-only', '--no-role-passwords', '--file', folder / 'roles.sql')
    if fingerprint(DATABASE) != before:
        raise ValueError('source_changed_backup_left_unverified')
    manifest = {'version': 2, 'source_database': DATABASE, 'historical_recovery': False,
                'source_system_identifier': sql('SELECT system_identifier FROM pg_control_system()', 'postgres'),
                'database_sha256': digest(folder / 'database.dump'), 'roles_sha256': digest(folder / 'roles.sql'),
                'application_state_sha256': before, 'identity': IDENTITY}
    exclusive_json(folder / 'manifest.json', manifest)
    return folder.name


def verify(backup_id):
    # Only our own local backup manifests, never arbitrary dumps/SQL from outside.
    if str(uuid.UUID(backup_id)) != backup_id:
        raise ValueError('canonical_backup_uuid_required')
    identity()
    folder = BASE / 'backups' / backup_id
    private_directory(folder)
    manifest = json.loads(private_file(folder / 'manifest.json').read_text())
    if manifest.get('version') != 2 or manifest.get('identity') != IDENTITY or manifest.get('source_database') != DATABASE:
        raise ValueError('backup_identity_mismatch')
    if digest(folder / 'database.dump') != manifest['database_sha256'] or digest(folder / 'roles.sql') != manifest['roles_sha256']:
        raise ValueError('backup_checksum_mismatch')
    destination = 'supa_restore_test_' + uuid.uuid4().hex
    command('createdb', '--template=template0', destination)
    # No --clean, --create, role import, target override, or overwrite path exists.
    command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', destination,
            folder / 'database.dump')
    matches = fingerprint(destination) == manifest['application_state_sha256']
    result = {'restored_database': destination, 'matches_all_user_schema_rows_columns_constraints_indexes_sequences': matches,
              'roles_archive_hash_verified_not_imported': True, 'historical_database_recovered': False,
              'paid_requests': 0}
    exclusive_json(folder / ('verified-' + uuid.uuid4().hex + '.json'), result)
    if not matches:
        raise ValueError('restored_public_state_mismatch')
    return result


def main():
    os.umask(0o077)
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('action', choices=['prepare', 'start', 'stop', 'status', 'backup', 'verify'])
    p.add_argument('--confirm-test-infrastructure', action='store_true')
    p.add_argument('--backup-id')
    a = p.parse_args()
    # Serialize infrastructure mutations without sharing the PostgreSQL lock files.
    private_directory(BASE)
    if a.action == 'prepare':
        if not a.confirm_test_infrastructure:
            raise ValueError('explicit_nonproduction_confirmation_required')
        prepare()
    else:
        lock_path = BASE / 'operations.lock'
        fd = os.open(lock_path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            if a.action == 'start':
                start()
            elif a.action == 'stop':
                identity()
                command('pg_ctl', '-D', BASE / 'data', '-m', 'fast', '-w', 'stop')
            elif a.action == 'status':
                identity()
                print(json.dumps({**IDENTITY, 'connected': True, 'network_listener': False,
                                  'database': DATABASE, 'fsync': sql('SHOW fsync', 'postgres')}))
            elif a.action == 'backup':
                print(json.dumps({'backup_id': backup(), 'historical_recovery': False}))
            elif a.action == 'verify':
                print(json.dumps(verify(a.backup_id or '')))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError, KeyError, json.JSONDecodeError) as e:
        code = str(e) if isinstance(e, ValueError) and str(e).replace('_', '').isalnum() else 'infrastructure_operation_failed'
        print(json.dumps({'error': code, 'historical_database_recovered': False}), file=sys.stderr)
        sys.exit(2)
