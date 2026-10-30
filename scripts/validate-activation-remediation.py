#!/usr/bin/env python3
"""Security regression on a new socket-only cluster; never start/connect to the primary."""
import getpass
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import uuid
from urllib.parse import urlencode, quote

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'artifacts/activation-remediation'
SOURCE = ROOT / 'postgres-data/epoch-backups/93ae651e-8bd7-4d68-92dc-a9ee29038479'
BASE = ROOT / 'postgres-data/remediation' / uuid.uuid4().hex[:12]
DB = 'supa_cutover_test_remediation'
USER = getpass.getuser()
spec = importlib.util.spec_from_file_location('disposable_pg', ROOT / 'scripts/persistent-postgres.py')
pg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pg)


def require(test, code):
    if not test:
        raise ValueError(code)


def environment(database='postgres', user=USER, password=None):
    env = {'PATH': '/usr/bin:/bin', 'HOME': str(Path.home()), 'LANG': 'C.UTF-8',
           'PGHOST': str(BASE / 's'), 'PGPORT': '55443', 'PGUSER': user, 'PGDATABASE': database,
           'PGCONNECT_TIMEOUT': '5', 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null'}
    if password is not None:
        env['PGPASSWORD'] = password
    return env


pg.environment = environment


def sql(statement, database=DB, user=USER, password=None, accepted=True):
    r = subprocess.run([str(pg.BIN / 'psql'), '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1'],
                       input=statement, env=environment(database, user, password), capture_output=True, text=True, timeout=30)
    require((r.returncode == 0) == accepted, 'isolated_sql_expectation_failed')
    return r.stdout.strip()


def url(database=DB, user=USER, password=None):
    return 'postgresql://' + quote(user, safe='') + (':' + quote(password, safe='') if password else '') + '@localhost/' + quote(database, safe='') + '?' + urlencode({'host': str(BASE / 's'), 'port': 55443})


def protected():
    primary = ROOT / 'postgres-data/infrastructure-v1/data'
    require(not (primary / 'postmaster.pid').exists(), 'primary_must_remain_stopped')
    paths = [primary / x for x in ['global/pg_control', 'postgresql.conf', 'pg_hba.conf']]
    paths += [ROOT / 'postgres-data/owner-provisioning' / x for x in ['identity.json', 'credentials.pending.json']]
    paths += [ROOT / 'web/.env.local', SOURCE / 'manifest.json', SOURCE / 'database.dump']
    paths += list((ROOT / 'artifacts/live-eval').glob('*'))
    paths += [ROOT / 'artifacts/persistent-postgres/verification.json', ROOT / 'docs/persistent-postgres-accounting.md']
    require(all(p.is_file() for p in paths), 'protected_evidence_missing')
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths if p.is_file()}


def migrate(database=DB, succeeds=True):
    env = {'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'DATABASE_URL': url(database),
           'SUPA_WORKER_ENABLED': 'true', 'SUPA_API_SCHEDULER_ENABLED': 'true', 'SUPA_ALLOW_OPENAI': 'true'}
    r = subprocess.run([str(ROOT / 'target/debug/examples/review_cutover'), '--exercise-disposable', 'migrate'],
                       cwd=ROOT, env=env, capture_output=True, timeout=30)
    require((r.returncode == 0) == succeeds, 'migration_only_expectation_failed')


def checksums(database, count):
    rows = json.loads(sql("SELECT json_agg(json_build_object('v',version,'sha',encode(checksum,'hex')) ORDER BY version) FROM _sqlx_migrations WHERE success", database))
    require(len(rows) == count, 'wrong_migration_count')
    for row in rows:
        file, = (ROOT / 'migrations').glob(f"{row['v']:04d}_*.sql")
        require(hashlib.sha384(file.read_bytes()).hexdigest() == row['sha'], 'migration_checksum_mismatch')


def run():
    os.umask(0o077)
    require(not (OUT / 'validation.json').exists(), 'preserve_previous_evidence')
    OUT.mkdir(parents=True, exist_ok=True)
    before = protected()
    manifest = json.loads((SOURCE / 'manifest.json').read_text())
    require(hashlib.sha256((SOURCE / 'database.dump').read_bytes()).hexdigest() == manifest['database_sha256'], 'source_backup_hash_mismatch')
    pg.private_directory(BASE)
    pg.private_directory(BASE / 's')
    started = False
    try:
        pg.command('initdb', '-D', BASE / 'data', '--auth-local=peer', '--auth-host=reject', '--encoding=UTF8', '--locale=C.UTF-8', '--data-checksums')
        with (BASE / 'data/postgresql.conf').open('a') as f:
            f.write(f"\nlisten_addresses=''\nunix_socket_directories='{BASE / 's'}'\nport=55443\npassword_encryption='scram-sha-256'\nmax_connections=150\n")
        (BASE / 'data/pg_hba.conf').write_text(f'local all {USER} peer\nlocal all all scram-sha-256\nhost all all 127.0.0.1/32 reject\nhost all all ::1/128 reject\n')
        pg.command('pg_ctl', '-D', BASE / 'data', '-l', BASE / 'server.log', '-w', 'start')
        started = True
        require(sql('SHOW data_directory', 'postgres') == str(BASE / 'data'), 'wrong_cluster')
        require(sql('SHOW listen_addresses', 'postgres') == '', 'tcp_not_disabled')
        pg.command('createdb', '--template=template0', DB)
        pg.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', DB, SOURCE / 'database.dump')
        checksums(DB, 8)
        # Remove real identifiers from this disposable copy before security tests create any consent.
        epoch, namespace, actor = [str(uuid.uuid4()) for _ in range(3)]
        sql(f"UPDATE application_epoch SET id='{epoch}',accounting_namespace='{namespace}',label='Synthetic remediation fixture'")
        migrate(); checksums(DB, 10); migrate()
        require(sql('SELECT state||revision FROM application_epoch') == 'prepared0', 'migration_activated_state')
        require(sql('SELECT (SELECT count(*) FROM agents)+(SELECT count(*) FROM agent_access)+(SELECT count(*) FROM epoch_guard.approvals)+(SELECT count(*) FROM epoch_guard.executors)+(SELECT count(*) FROM epoch_guard.authorities)+(SELECT count(*) FROM live_budgets)+(SELECT count(*) FROM worker_health)') == '0', 'migration_granted_authority')
        bad = 'supa_cutover_test_bad_checksum'
        pg.command('createdb', '--template=template0', bad)
        pg.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', bad, SOURCE / 'database.dump')
        sql("UPDATE _sqlx_migrations SET checksum=decode(repeat('00',48),'hex') WHERE version=1", bad)
        migrate(bad, False)
        require(sql('SELECT count(*) FROM _sqlx_migrations', bad) == '8', 'bad_checksum_applied_migration')
        testdb = 'supa_remediation_test'
        pg.command('createdb', '--template=template0', testdb)
        with (BASE / 'rust-tests.log').open('wb') as log:
            r = subprocess.run(['cargo', 'test', '--workspace', '--all-targets', '--locked', '--offline', '--', '--include-ignored'],
                               cwd=ROOT, env={'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'TEST_DATABASE_URL': url(testdb)}, stdout=log, stderr=subprocess.STDOUT, timeout=600)
        require(r.returncode == 0, 'rust_tests_failed_see_private_log')
        total = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', (BASE / 'rust-tests.log').read_text())))
        passwords = {x: secrets.token_urlsafe(36) + '@:/%#' for x in ['authority','executor','reader']}
        for role, password in passwords.items():
            sql(f"CREATE ROLE remediation_{role} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}'", 'postgres')
        sql(f"INSERT INTO epoch_guard.authorities VALUES('remediation_authority','{uuid.uuid4()}'); INSERT INTO epoch_guard.executors VALUES('remediation_executor','{actor}','owner'); GRANT USAGE ON SCHEMA epoch_guard TO remediation_authority,remediation_executor; GRANT EXECUTE ON FUNCTION epoch_guard.approve(uuid,text,timestamptz) TO remediation_authority; GRANT EXECUTE ON FUNCTION epoch_guard.execute_transition(uuid,uuid,text,uuid,bigint,text,text) TO remediation_executor; REVOKE CREATE ON SCHEMA public FROM PUBLIC; GRANT USAGE ON SCHEMA public TO remediation_reader; GRANT SELECT ON ALL TABLES IN SCHEMA public TO remediation_reader")
        sql(f'REVOKE CONNECT,TEMPORARY ON DATABASE {DB} FROM PUBLIC; GRANT CONNECT ON DATABASE {DB} TO remediation_authority,remediation_executor,remediation_reader; ALTER DATABASE {DB} SET default_transaction_read_only=on', 'postgres')
        sql('SELECT 1', user='remediation_reader', password='wrong', accepted=False)
        for statement in ['BEGIN READ WRITE; UPDATE application_epoch SET revision=1', 'BEGIN READ WRITE; CREATE TABLE forbidden(id integer)', 'CREATE TEMP TABLE forbidden(id integer)', f'SET ROLE "{USER}"', 'BEGIN READ WRITE; INSERT INTO epoch_guard.authorities VALUES(current_user,gen_random_uuid())']:
            sql(statement, user='remediation_reader', password=passwords['reader'], accepted=False)
        migrations = json.loads(sql("SELECT json_object_agg(version::text,encode(checksum,'hex')) FROM _sqlx_migrations WHERE success"))
        plan = json.dumps({'version':1,'database':DB,'epoch_id':epoch,'accounting_namespace':namespace,'principal_id':actor,'role':'owner','operation':'activate_read_only','expected_revision':0,'scope':'local_development_read_only','config_sha256':{'api':'a'*64,'frontend':'b'*64,'postgres':'c'*64},'migrations_sha384':migrations}, separators=(',', ':'))
        approval = str(uuid.uuid4())
        def literal(s): return "'" + s.replace("'", "''") + "'"
        sql(f"BEGIN READ WRITE; SELECT epoch_guard.approve('{approval}',{literal(plan)},clock_timestamp()+interval '30 minutes'); COMMIT", user='remediation_authority', password=passwords['authority'])
        sql(f"BEGIN READ WRITE; SELECT epoch_guard.execute_transition('{approval}','{actor}','owner','{epoch}',0,'activate_read_only',{literal(plan)}); COMMIT", user='remediation_executor', password=passwords['executor'])
        active = pg.fingerprint(DB)
        pg.command('pg_dump', '--format=custom', '--file', BASE / 'active.dump', database=DB)
        pg.private_file(BASE / 'active.dump')
        restored = 'supa_cutover_test_restored'
        pg.command('createdb', '--template=template0', restored)
        pg.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', restored, BASE / 'active.dump')
        require(pg.fingerprint(restored) == active == pg.fingerprint(DB), 'active_backup_not_exact')
        # A renamed restore preserves evidence but cannot inherit operational approval for the original database.
        sql(f"SELECT epoch_guard.verify_active('{epoch}','{approval}')", restored, accepted=False)
        checksums(restored,10)
        token, public, browser_password = [secrets.token_urlsafe(48) for _ in range(3)]
        runtime = {'root':str(BASE),'database':DB,'epoch_id':epoch,'approval_id':approval,
                   'api': {'DATABASE_URL':url(user='remediation_reader',password=passwords['reader']),'SUPA_API_TOKEN':public,'SUPA_PROVIDER':'offline','SUPA_EPOCH_MODE':'active_read_only','SUPA_EPOCH_READONLY_PREVIEW':'false','SUPA_EXPECTED_EPOCH_ID':epoch,'SUPA_EPOCH_APPROVAL_ID':approval,'SUPA_OPERATOR_CREDENTIALS':json.dumps([{'id':actor,'role':'owner','token':token}]),**{k:'false' for k in ['SUPA_ALLOW_OPENAI','SUPA_CONTROLLED_INFERENCE','SUPA_API_SCHEDULER_ENABLED','SUPA_WALLET_ENABLED','SUPA_PRIVATE_STAGING']}},
                   'synthetic_username':'remediation-owner','synthetic_password':browser_password,'synthetic_token':token}
        pg.exclusive_json(BASE / 'runtime.json', runtime)
        env = {'PATH':os.environ['PATH'],'HOME':str(Path.home()),'SUPA_VALIDATION_SUITE':'activation-remediation'}
        if os.environ.get('SUPA_REA_CDP_PORT'): env['SUPA_REA_CDP_PORT']=os.environ['SUPA_REA_CDP_PORT']
        r = subprocess.run(['node','scripts/local-safety-browser.mjs',str(BASE / 'runtime.json')],cwd=ROOT,env=env,timeout=600)
        require(r.returncode == 0, 'browser_failed')
        require(protected() == before, 'protected_primary_files_changed')
        report = {'disposable_cluster_only':True,'cluster':str(BASE.relative_to(ROOT)),'source_backup_sha256_verified':True,'primary_cluster_started':False,'primary_files_unchanged':True,'primary_database_connected':False,'real_state_last_verified':'prepared at migration 8; not queried while stopped','real_owner_granted':False,'real_approval_created':False,'runtime_credentials_installed':False,'primary_frontend_repointed':False,'migration_checksums_verified':10,'migration_only_leaves_prepared_ungranted':True,'tampered_checksum_rejected':True,'rust_tests_passed':total,'trusted_active_backup_restoration_exact':True,'renamed_restore_requires_fresh_approval':True,'backup_mode':'0600','independent_backup_destination':None,'historical_database':'unrecovered','historical_budget_ceiling_microusd':100000,'historical_unresolved_hold_microusd':4048,'provider_requests':0,'real_database_mutations':0,'tcp_disabled':True,'scram_authentication':True,'restricted_reader_denials':5}
    finally:
        if started: pg.command('pg_ctl','-D',BASE / 'data','-m','fast','-w','stop')
    report['disposable_cluster_stopped']=True
    (OUT / 'validation.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'complete':True,'rust_tests_passed':total,'primary_untouched':True}))


if __name__ == '__main__':
    run()
