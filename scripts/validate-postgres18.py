#!/usr/bin/env python3
"""Real PostgreSQL 18 validation in a newly created, socket-only disposable cluster.

Never accepts an existing data directory, database URL, epoch or production target.
Binaries are supplied explicitly; this script does not install software.
"""
import argparse
import concurrent.futures
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent


def version18(text):
    if not re.search(r'\(PostgreSQL\) 18\.', text):
        raise ValueError('postgresql_18_binaries_required')


def library_environment(binary, library):
    env = {key: os.environ[key] for key in ('PATH', 'HOME') if key in os.environ}
    env['LANG'] = 'C.UTF-8'
    if library:
        env['LD_LIBRARY_PATH'] = str(library.resolve(strict=True))
    result = subprocess.run([str(binary / 'postgres'), '--version'], env=env,
                            capture_output=True, text=True, check=True)
    version18(result.stdout)
    return env, result.stdout.strip()


def ready(port):
    try:
        with urllib.request.urlopen(f'http://127.0.0.1:{port}/ready', timeout=7) as response:
            return response.status
    except urllib.error.HTTPError as error:
        return error.code
    except (OSError, TimeoutError):
        return None


def free_port():
    import socket
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def wait_ready(port, process):
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError('test_service_exited_before_readiness')
        if ready(port) == 200:
            return
        time.sleep(.2)
    raise RuntimeError('test_service_readiness_timeout')


def validate(binary, library, evidence, offline=False):
    env, version = library_environment(binary, library)
    evidence.mkdir(parents=True, exist_ok=False)
    base = Path(tempfile.mkdtemp(prefix='supa-pg18-validation-'))
    (base / 'socket').mkdir(mode=0o700)
    database = 'supa_pg18_compat_test'
    user = __import__('getpass').getuser()
    env.update(PGHOST=str(base / 'socket'), PGPORT='55448', PGUSER=user,
               PGDATABASE=database, PGCONNECT_TIMEOUT='5', PGSERVICEFILE='/dev/null',
               PGOPTIONS='-c timezone=UTC')
    children = []

    def pg(name, *args, allow_failure=False):
        result = subprocess.run([str(binary / name), *map(str, args)], env=env,
                                capture_output=True, text=True, timeout=120)
        if result.returncode and not allow_failure:
            raise RuntimeError('postgres_test_operation_failed_' + name)
        return result

    def sql(query):
        return pg('psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', '-c', query).stdout.strip()

    def url(db):
        return f'postgresql://{user}@localhost/{db}?' + urllib.parse.urlencode(
            {'host': str(base / 'socket'), 'port': 55448})

    try:
        pg('initdb', '-D', base / 'data', '--auth-local=peer', '--auth-host=reject',
           '--data-checksums', '--locale=C.UTF-8')
        with (base / 'data/postgresql.conf').open('a') as config:
            config.write(f"\nlisten_addresses=''\nunix_socket_directories='{base}/socket'\n"
                         'port=55448\nmax_connections=150\n')
        (base / 'data/pg_hba.conf').write_text(
            f'local all {user} peer\nlocal all all scram-sha-256\n'
            'host all all 127.0.0.1/32 reject\nhost all all ::1/128 reject\n')
        pg('pg_ctl', '-D', base / 'data', '-l', base / 'server.log', '-w', 'start')
        pg('createdb', '--template=template0', database)
        pg('createdb', '--template=template0', 'supa_pg18_suite_test')
        service_env = {key: env[key] for key in ('PATH', 'HOME', 'LANG') if key in env}
        service_env.update(DATABASE_URL=url(database), SUPA_PROVIDER='offline',
                           SUPA_PRIVATE_STAGING='true', SUPA_API_SCHEDULER_ENABLED='false',
                           SUPA_WORKER_ENABLED='false', SUPA_ALLOW_OPENAI='false',
                           SUPA_CONTROLLED_INFERENCE='false', SUPA_WALLET_ENABLED='false',
                           SUPA_API_TOKEN=secrets.token_hex(32),
                           SUPA_OPERATOR_CREDENTIALS=json.dumps([{
                               'id': str(uuid.uuid4()), 'role': 'viewer', 'token': secrets.token_hex(32)}]))
        ports = [free_port(), free_port()]
        for name, key, port in zip(('supa-api', 'supa-runtime'),
                                   ('SUPA_BIND', 'SUPA_WORKER_BIND'), ports):
            process = subprocess.Popen([str(ROOT / 'target/debug' / name)],
                                       env={**service_env, key: f'127.0.0.1:{port}'},
                                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            children.append(process)
            wait_ready(port, process)
        checksums = json.loads(sql("SELECT json_agg(json_build_object('version',version,"
                                   "'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) "
                                   'FROM _sqlx_migrations'))
        assert [item['version'] for item in checksums] == list(range(1, 11))
        for item in checksums:
            source = next((ROOT / 'migrations').glob(f"{item['version']:04d}_*.sql"))
            assert item['success'] and hashlib.sha384(source.read_bytes()).hexdigest() == item['checksum']
        agent, key = str(uuid.uuid4()), str(uuid.uuid4())
        exact = 9007199254740993  # One above JavaScript's exact-integer range.
        sql(f"INSERT INTO agents(id,data,simulated_balance_microusd) VALUES('{agent}','{{}}',{exact})")
        assert int(sql(f"SELECT simulated_balance_microusd FROM agents WHERE id='{agent}'")) == exact
        negative = pg('psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', '-c',
                      f"UPDATE agents SET simulated_balance_microusd=-1 WHERE id='{agent}'", allow_failure=True)
        assert negative.returncode != 0
        bad_fk = pg('psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', '-c',
                    f"INSERT INTO simulated_ledger(id,agent_id,kind,amount_microusd,idempotency_key) "
                    f"VALUES('{uuid.uuid4()}','{uuid.uuid4()}','seed',1,'{uuid.uuid4()}')", allow_failure=True)
        assert bad_fk.returncode != 0

        def competing_insert(_):
            return pg('psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', '-c',
                      f"INSERT INTO simulated_ledger(id,agent_id,kind,amount_microusd,idempotency_key) "
                      f"VALUES('{uuid.uuid4()}','{agent}','seed',1,'{key}')", allow_failure=True).returncode

        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
            outcomes = list(executor.map(competing_insert, range(2)))
        assert sum(code == 0 for code in outcomes) == 1
        assert sql(f"SELECT count(*) FROM simulated_ledger WHERE agent_id='{agent}' AND idempotency_key='{key}'") == '1'
        assert sql("SELECT count(*) FROM pg_indexes WHERE schemaname='public' AND indexname "
                   "IN ('one_running_cycle','activity_by_agent','worker_lease_lookup')") == '3'
        before = sql('SELECT count(*) FROM cycles')
        pg('pg_ctl', '-D', base / 'data', '-m', 'immediate', '-w', 'stop')
        assert all(ready(port) == 503 for port in ports)
        pg('pg_ctl', '-D', base / 'data', '-l', base / 'server.log', '-w', 'start')
        for port, process in zip(ports, children):
            wait_ready(port, process)
        for process in children:
            process.terminate()
        assert [process.wait(timeout=20) for process in children] == [0, 0]
        assert sql('SELECT count(*) FROM cycles') == before == '0'
        assert sql('SELECT count(*) FROM worker_health WHERE enabled') == '0'
        guards = sql("SELECT (SELECT count(*) FROM application_epoch)+(SELECT count(*) FROM epoch_guard.authorities)"
                     '+(SELECT count(*) FROM epoch_guard.executors)+(SELECT count(*) FROM epoch_guard.approvals)')
        assert guards == '0'
        spec = importlib.util.spec_from_file_location('persistent', ROOT / 'scripts/persistent-postgres.py')
        persistent = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(persistent)
        persistent.BIN = binary
        persistent.environment = lambda db='postgres': {**env, 'PGDATABASE': db}
        fingerprint = persistent.fingerprint(database)
        schema_query = """SELECT jsonb_build_object(
          'functions',(SELECT jsonb_agg(pg_get_functiondef(p.oid) ORDER BY n.nspname,p.proname,p.oid::regprocedure::text)
            FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname IN ('public','epoch_guard')),
          'triggers',(SELECT jsonb_agg(pg_get_triggerdef(t.oid) ORDER BY t.tgname) FROM pg_trigger t WHERE NOT t.tgisinternal),
          'privileges',(SELECT coalesce(jsonb_agg(to_jsonb(p)-'table_catalog' ORDER BY table_schema,table_name,grantee,privilege_type),'[]')
            FROM information_schema.table_privileges p WHERE table_schema IN ('public','epoch_guard')))"""
        schema = sql(schema_query)
        dump = base / 'compat.dump'
        pg('pg_dump', '--format=custom', '--file', dump, database)
        dump.chmod(0o600)
        pg('createdb', '--template=template0', 'supa_pg18_restore_test')
        pg('pg_restore', '--single-transaction', '--exit-on-error', '--dbname', 'supa_pg18_restore_test', dump)
        assert fingerprint == persistent.fingerprint(database) == persistent.fingerprint('supa_pg18_restore_test')
        restored = subprocess.run([str(binary / 'psql'), '-XqAt', '-v', 'ON_ERROR_STOP=1', '-c', schema_query],
                                  env={**env, 'PGDATABASE': 'supa_pg18_restore_test'}, capture_output=True,
                                  text=True, check=True).stdout.strip()
        assert schema == restored
        test_env = {key: env[key] for key in ('PATH', 'HOME', 'LANG') if key in env}
        test_env.update(TEST_DATABASE_URL=url('supa_pg18_suite_test'))
        # The existing cached local binaries are used; CI can set debug profiles before compilation.
        # Do not change profile variables here and force an avoidable local rebuild.
        for key in ('CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_TEST_DEBUG', 'CARGO_INCREMENTAL'):
            if key in os.environ:
                test_env[key] = os.environ[key]
        args = ['cargo', 'test', '--workspace', '--locked', '--exclude', 'supa-migration-admin']
        if offline:
            args.append('--offline')
        with (evidence / 'rust.log').open('w') as log:
            result = subprocess.run(args + ['--', '--include-ignored'], cwd=ROOT, env=test_env,
                                    stdout=log, stderr=subprocess.STDOUT, timeout=2400)
        assert result.returncode == 0, 'postgres18_rust_suite_failed_see_local_log'
        tests = (evidence / 'rust.log').read_text()
        passed = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', tests)))
        assert passed > 100
        report = {'passed': True, 'postgres_version': version, 'migrations': checksums,
                  'exact_bigint_roundtrip': exact, 'negative_balance_and_foreign_key_rejected': True,
                  'concurrent_idempotency_successes': 1, 'critical_indexes_verified': True,
                  'crash_readiness_503_then_200': True, 'api_worker_sigterm_exit_codes': [0, 0],
                  'worker_disabled_no_cycles': True, 'empty_epoch_and_approval_tables': True,
                  'backup_restore_fingerprint': fingerprint, 'functions_triggers_privileges_match': True,
                  'backup_sha256': hashlib.sha256(dump.read_bytes()).hexdigest(), 'backup_mode': '0600',
                  'rust_tests_passed': passed, 'container_image_tested': False,
                  'primary_database_accessed': False, 'paid_requests': 0}
        (evidence / 'postgres18.json').write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps({key: value for key, value in report.items() if key != 'migrations'}), flush=True)
        return report
    finally:
        for process in children:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=20)
        if (base / 'data/postmaster.pid').exists():
            pg('pg_ctl', '-D', base / 'data', '-m', 'fast', '-w', 'stop')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, required=True)
    parser.add_argument('--library-dir', type=Path)
    parser.add_argument('--evidence-dir', type=Path, required=True)
    parser.add_argument('--offline', action='store_true')
    arguments = parser.parse_args()
    validate(arguments.bin_dir.resolve(strict=True), arguments.library_dir,
             arguments.evidence_dir.resolve(), arguments.offline)
