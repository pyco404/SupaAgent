#!/usr/bin/env python3
"""Rehearse least-privilege runtime/restore on new databases in the local launch fixture.
Starts only a fresh socket-only cluster. Never starts the primary, imports source
application rows, or accepts a primary URL.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
PG = '/usr/lib/postgresql/16/bin/'


def main():
    assert not (ROOT/'postgres-data/infrastructure-v1/data/postmaster.pid').exists(), 'primary_must_be_stopped'
    path = ROOT/'artifacts/local-launch/runtime.json'
    info = path.lstat()
    assert not path.is_symlink() and info.st_uid == os.getuid() and not info.st_mode & 0o077
    fixture = json.loads(path.read_text())
    base = Path(fixture['base'])
    assert fixture['purpose'] == 'disposable_local_agent_launch'
    assert base.parent == Path('/tmp') and re.fullmatch(r'supa-launch-[a-f0-9]{12}',base.name)
    assert fixture['database'] == 'supa_launch_'+base.name.removeprefix('supa-launch-')+'_test'
    assert fixture['socket'] == str(base/'socket') and not base.is_symlink() and not (base/'data').is_symlink()
    binary = Path(fixture['binary'])
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == fixture['binary_sha256']
    env = {'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'LANG': 'C.UTF-8',
           'PGHOST': str(base/'socket'), 'PGPORT': '55690', 'PGUSER': 'pikoo',
           'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null',
           'PGOPTIONS': '-c timezone=UTC'}
    suffix = uuid.uuid4().hex[:12]
    database, restored, role = ('supa_stage_'+suffix+'_test', 'supa_restore_'+suffix+'_test', 'supa_app_'+suffix)
    out = ROOT/'artifacts/milestone-11'/('profile-'+suffix)
    out.mkdir(mode=0o700, parents=True)
    process = None
    created = []
    role_created = False
    cluster_started = False

    def pg(command, *args, input=None, db='postgres', extra=None, success=True):
        result = subprocess.run([PG+command, *args], input=input, env={**env, 'PGDATABASE': db, **(extra or {})}, capture_output=True, timeout=40)
        if success and result.returncode:
            raise RuntimeError('disposable_profile_'+command+'_failed')
        return result

    def sql(query, db='postgres', **kwargs):
        return pg('psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', input=query.encode(), db=db, **kwargs)

    def fingerprint(db):
        tables = json.loads(sql("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='public'", db).stdout)
        digest = hashlib.sha256()
        for table in tables:
            assert table.replace('_', '').isalnum()
            rows = sql(f'''SELECT coalesce(jsonb_agg(v ORDER BY v::text),'[]'::jsonb) FROM (SELECT to_jsonb(t) v FROM "{table}" t) x''', db).stdout
            digest.update(table.encode()+b'\n'+rows)
        return digest.hexdigest()

    def schema_fingerprint(db):
        query = """SELECT jsonb_build_object(
          'constraints',(SELECT jsonb_agg(pg_get_constraintdef(c.oid) ORDER BY n.nspname,t.relname,c.conname)
            FROM pg_constraint c JOIN pg_class t ON t.oid=c.conrelid JOIN pg_namespace n ON n.oid=t.relnamespace
            WHERE n.nspname IN ('public','epoch_guard')),
          'indexes',(SELECT jsonb_agg(indexdef ORDER BY schemaname,tablename,indexname) FROM pg_indexes WHERE schemaname IN ('public','epoch_guard')),
          'functions',(SELECT jsonb_agg(pg_get_functiondef(p.oid) ORDER BY n.nspname,p.proname,p.oid::regprocedure::text)
            FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname IN ('public','epoch_guard')),
          'triggers',(SELECT jsonb_agg(pg_get_triggerdef(t.oid) ORDER BY t.tgname) FROM pg_trigger t WHERE NOT t.tgisinternal),
          'privileges',(SELECT jsonb_agg(to_jsonb(p)-'table_catalog' ORDER BY table_schema,table_name,grantee,privilege_type)
            FROM information_schema.table_privileges p WHERE table_schema IN ('public','epoch_guard')))"""
        return hashlib.sha256(sql(query, db).stdout).hexdigest()

    try:
        assert sql('SHOW data_directory').stdout.decode().strip() == str(base/'data')
        assert sql('SHOW listen_addresses').stdout.strip() == b''
        assert sql('SELECT system_identifier::text FROM pg_control_system()').stdout.decode().strip() == fixture['system_identifier']
        # Schema/ACLs and migration metadata only: no agents, authorizations or accounting rows copied.
        schema = pg('pg_dump', '--schema-only', '--no-owner', db=fixture['database']).stdout
        migrations = pg('pg_dump', '--data-only', '--table=public._sqlx_migrations', '--no-owner', db=fixture['database']).stdout
        assert shutil.disk_usage(ROOT).free >= 250 * 1024**2, 'insufficient_disposable_cluster_headroom'
        # Preserve the existing fixture's restrictive HBA. Start a separate,
        # socket-only disposable cluster for the temporary runtime credential.
        data, sock = out/'data', out/'s'
        sock.mkdir(mode=0o700)
        pg('initdb', '-D', str(data), '--auth-local=peer', '--auth-host=reject', '--locale=C.UTF-8')
        with (data/'postgresql.conf').open('a') as config:
            config.write(f"\nlisten_addresses=''\nunix_socket_directories='{sock}'\nunix_socket_permissions=0700\nport=55449\nshared_buffers=16MB\n")
        (data/'pg_hba.conf').write_text('local all pikoo peer\nlocal all all scram-sha-256\n')
        pg('pg_ctl', '-D', str(data), '-l', str(out/'postgres.log'), '-w', 'start')
        cluster_started = True
        env.update(PGHOST=str(sock), PGPORT='55449')
        sql('CREATE ROLE supa_launch_app NOLOGIN')
        for db in (database, restored):
            sql('CREATE DATABASE '+db)
            created.append(db)
        pg('psql', '-Xq', '-v', 'ON_ERROR_STOP=1', input=schema+migrations, db=database)
        installed = json.loads(sql("SELECT json_agg(json_build_object('version',version,'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) FROM _sqlx_migrations", database).stdout)
        expected = {int(p.name.split('_',1)[0]):hashlib.sha384(p.read_bytes()).hexdigest() for p in (ROOT/'migrations').glob('*.sql')}
        assert {r['version']:r['checksum'] for r in installed if r['success']} == expected
        assert sql('SELECT count(*) FROM agents', database).stdout.strip() == b'0'
        password = secrets.token_hex(32)
        sql(f"CREATE ROLE {role} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'")
        role_created = True
        sql(f'''REVOKE CONNECT ON DATABASE {database} FROM PUBLIC;
GRANT CONNECT ON DATABASE {database} TO {role};
GRANT USAGE ON SCHEMA public TO {role};
GRANT SELECT ON ALL TABLES IN SCHEMA public TO {role};''', database)
        tables = json.loads(sql("SELECT json_agg(tablename) FROM pg_tables WHERE schemaname='public' AND tablename NOT IN ('_sqlx_migrations','application_epoch')", database).stdout)
        for table in tables:
            assert table.replace('_', '').isalnum()
            sql(f'GRANT INSERT,UPDATE,DELETE ON "{table}" TO {role}', database)
        for query in ['CREATE TABLE public.forbidden_test(id int)', 'UPDATE _sqlx_migrations SET success=false', 'DELETE FROM epoch_guard.approvals']:
            result = sql(query, database, extra={'PGUSER': role, 'PGPASSWORD': password}, success=False)
            assert result.returncode and b'permission denied' in result.stderr, 'runtime_ddl_or_guard_not_fenced'
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            port = listener.getsockname()[1]
        token, service = secrets.token_hex(32), secrets.token_hex(32)
        identity = str(uuid.uuid4())
        child = {key: env[key] for key in ('PATH', 'HOME', 'LANG')}
        child.update(DATABASE_URL=f'postgresql://{role}@localhost/{database}?'+urllib.parse.urlencode({'host':str(sock),'port':55449}),
                     PGPASSWORD=password, SUPA_BIND=f'127.0.0.1:{port}', SUPA_PRIVATE_STAGING='true',
                     SUPA_API_TOKEN=service, SUPA_OPERATOR_CREDENTIALS=json.dumps([{'id':identity,'role':'owner','token':token}]),
                     SUPA_OPERATOR_SESSION_DEADLINES=json.dumps({identity:int(time.time())+300}),
                     SUPA_PROVIDER='offline', SUPA_ALLOW_OPENAI='false', SUPA_ALLOW_ANTHROPIC='false',
                     SUPA_CONTROLLED_INFERENCE='false', SUPA_CONTROLLED_WORK='false',
                     SUPA_API_SCHEDULER_ENABLED='false', SUPA_WORKER_ENABLED='false', SUPA_WALLET_ENABLED='false')
        def call(path, auth=None):
            request = urllib.request.Request(f'http://127.0.0.1:{port}'+path, headers={'Authorization':'Bearer '+auth} if auth else {})
            try:
                with urllib.request.urlopen(request, timeout=2) as r:
                    return r.status, json.load(r)
            except urllib.error.HTTPError as e:
                return e.code, None
        with (out/'api.log').open('w') as log:
            process = subprocess.Popen([str(binary)], cwd=out, env=child, stdout=log, stderr=subprocess.STDOUT)
            runtime_ready = False
            for _ in range(60):
                try:
                    if call('/ready')[0] == 200:
                        runtime_ready = True
                        break
                except (OSError, TimeoutError):
                    pass
                if process.poll() is not None:
                    break
                time.sleep(.2)
            if runtime_ready:
                assert call('/agents')[0] == 401
                assert call('/agents', service) == (200, [])
                status = call('/operator-status', token)
                assert status[0] == 200 and status[1]['database'] == database and status[1]['epoch'] is None
                process.terminate()
                assert process.wait(timeout=20) == 0
            else:
                # Record the current blocker, not a passing runtime claim. The
                # service must not gain CREATE merely to satisfy auto-migrate.
                assert process.wait(timeout=20) != 0
                diagnostics = (out/'postgres.log').read_text()
                assert 'permission denied for schema public' in diagnostics
                assert 'CREATE TABLE IF NOT EXISTS _sqlx_migrations' in diagnostics
        before = fingerprint(database)
        schema_before = schema_fingerprint(database)
        dump = out/'synthetic-profile.dump'
        pg('pg_dump', '--format=custom', '--file', str(dump), db=database)
        dump.chmod(0o600)
        pg('pg_restore', '--single-transaction', '--exit-on-error', '--dbname', restored, str(dump))
        assert fingerprint(database) == fingerprint(restored) == before
        assert schema_fingerprint(database) == schema_fingerprint(restored) == schema_before
        report = {'passed':True,'postgres_major':16,'container':False,'schema_migrations':len(installed),
                  'restricted_runtime_start_and_auth':runtime_ready,'runtime_ddl_and_trusted_guard_denied':True,
                  'runtime_blocker':None if runtime_ready else 'sqlx_auto_migrate_requires_schema_create',
                  'backup_restore_tables_match':True,'historical_application_rows_copied':0,
                  'restored_constraints_indexes_functions_triggers_acl_match':True,
                  'paid_requests':0,'primary_accessed':False,'backup_sha256':hashlib.sha256(dump.read_bytes()).hexdigest()}
        (out/'result.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(report))
    finally:
        if process and process.poll() is None:
            process.terminate()
            process.wait(timeout=20)
        for db in reversed(created):
            sql('DROP DATABASE '+db)
        if role_created:
            sql('DROP ROLE '+role)
        if cluster_started:
            pg('pg_ctl', '-D', str(out/'data'), '-m', 'fast', '-w', 'stop')
            assert not (out/'data/postmaster.pid').exists()
            shutil.rmtree(out/'data')
            (out/'s').rmdir()


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(json.dumps({'passed':False,'error':str(error) if isinstance(error,(RuntimeError,AssertionError)) else type(error).__name__}))
        raise SystemExit(1)
