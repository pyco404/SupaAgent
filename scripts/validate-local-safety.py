#!/usr/bin/env python3
"""Disposable-cluster validation only. Real candidate is a read-only backup source."""
import getpass
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import uuid
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'artifacts/final-local-safety'


def module(name, file):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / file)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


owner = module('owner', 'owner-provisioning.py')
real = owner.review.epoch.pg
local = module('isolated', 'persistent-postgres.py')
BASE = ROOT / 'postgres-data/safety' / uuid.uuid4().hex[:12]
DB = 'supa_cutover_test_safety'
USER = getpass.getuser()
plan_digest = hashlib.sha256((ROOT / 'artifacts/owner-readiness/change-set.json').read_bytes()).hexdigest()


def require(value, code):
    if not value:
        raise ValueError(code)


def save(path, value):
    real.exclusive_json(path, value)


def environment(database='postgres', user=USER, password=None):
    env = {'PATH': '/usr/bin:/bin', 'HOME': str(Path.home()), 'LANG': 'C.UTF-8',
           'PGHOST': str(BASE / 's'), 'PGPORT': '55442', 'PGUSER': user,
           'PGDATABASE': database, 'PGCONNECT_TIMEOUT': '5', 'PGOPTIONS': '-c timezone=UTC',
           'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null'}
    if password is not None:
        env['PGPASSWORD'] = password
    return env


local.environment = environment


def psql(statement, database=DB, user=USER, password=None, accepted=True):
    r = subprocess.run([str(real.BIN / 'psql'), '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1'],
                       input=statement, env=environment(database, user, password), capture_output=True, text=True, timeout=30)
    if accepted:
        require(r.returncode == 0, 'isolated_sql_failed')
    else:
        require(r.returncode != 0, 'unsafe_isolated_sql_accepted')
    return r.stdout.strip()


def url(database=DB, user=USER, password=None, writable=False):
    credentials = user + (':' + password if password else '')
    query = {'host': str(BASE / 's'), 'port': 55442}
    if writable:
        query['options'] = '-cdefault_transaction_read_only=off'
    return 'postgresql://' + credentials + '@localhost/' + database + '?' + urlencode(query)


def snapshot():
    candidate = owner.review.epoch.load()
    owner.load()
    owner.credentials()
    owner.ignored()
    protected = [owner.BASE / 'identity.json', owner.BASE / 'credentials.pending.json',
                 ROOT / 'web/.env.local', real.BASE / 'data/pg_hba.conf', real.BASE / 'data/postgresql.conf']
    protected += [p for p in [ROOT / '.env.eval', ROOT / '.env.devnet'] if p.exists()]
    return {'candidate_fingerprint': real.fingerprint(candidate['database']),
            'protected': {str(p): real.digest(p) for p in protected}, 'database': candidate['database']}


def migration(database=DB, expected_success=True):
    # Existing executable rejects non-disposable database names. Only our new cluster is connected.
    env = {'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'DATABASE_URL': url(database, writable=True),
           'SUPA_API_SCHEDULER_ENABLED': 'true', 'SUPA_WORKER_ENABLED': 'true',
           'SUPA_OPERATOR_ID': str(uuid.uuid4()), 'SUPA_ALLOW_OPENAI': 'true'}
    result = subprocess.run([str(ROOT / 'target/debug/examples/review_cutover'), '--exercise-disposable', 'migrate'],
                            cwd=ROOT, env=env, capture_output=True, timeout=30)
    require((result.returncode == 0) == expected_success, 'migration_only_guard_failed')


def checksums(database, count):
    records = json.loads(psql("SELECT json_agg(json_build_object('version',version,'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) FROM _sqlx_migrations", database))
    require(len(records) == count, 'migration_inventory_mismatch')
    for record in records:
        files = list((ROOT / 'migrations').glob(f"{record['version']:04d}_*.sql"))
        require(len(files) == 1 and record['success'], 'migration_file_mismatch')
        require(hashlib.sha384(files[0].read_bytes()).hexdigest() == record['checksum'], 'sqlx_checksum_mismatch')
    return len(records)


def transition(actor, operation, approval, revision, digest, succeeds=True):
    sql = owner.render_transition(DB, actor, operation)
    path = BASE / ('transition-' + uuid.uuid4().hex + '.sql')
    owner.review.private_write(path, sql.encode())
    result = subprocess.run([str(real.BIN / 'psql'), '-Xq', '--no-password', '-v', 'ON_ERROR_STOP=1',
                             '-v', 'approval_id=' + approval, '-v', 'expected_revision=' + str(revision),
                             '-v', 'plan_sha256=' + digest, '-f', str(path)], env=environment(DB), capture_output=True, timeout=30)
    require((result.returncode == 0) == succeeds, 'transition_expectation_mismatch')


def run():
    os.umask(0o077)
    require(not (OUT / 'validation.json').exists(), 'existing_validation_evidence_preserved')
    OUT.mkdir(parents=True, exist_ok=True)
    proposed = owner.plan()
    require(proposed['api_environment']['SUPA_BIND'] == '127.0.0.1:8080', 'proposed_api_bind_changed')
    require(proposed['frontend_environment']['PORT'] == '3200', 'proposed_site_port_changed')
    before = snapshot()
    real.private_directory(BASE)
    real.private_directory(BASE / 's')
    save(BASE / 'scope.json', {'purpose': 'disposable_validation', 'real_epoch_mutation_authorized': False})
    started = False
    try:
        local.command('initdb', '-D', BASE / 'data', '--auth-local=peer', '--auth-host=reject',
                      '--encoding=UTF8', '--locale=C.UTF-8', '--data-checksums')
        with (BASE / 'data/postgresql.conf').open('a') as f:
            f.write(f"\nlisten_addresses=''\nunix_socket_directories='{BASE / 's'}'\nport=55442\npassword_encryption='scram-sha-256'\n")
        local.command('pg_ctl', '-D', BASE / 'data', '-l', BASE / 'server.log', '-w', 'start')
        started = True
        require(psql('SHOW data_directory', 'postgres') == str(BASE / 'data'), 'isolated_cluster_identity_failed')
        require(psql('SHOW listen_addresses', 'postgres') == '', 'unexpected_tcp_listener')
        real.command('pg_dump', '--format=custom', '--file', BASE / 'source.dump', database=before['database'])
        require(real.fingerprint(before['database']) == before['candidate_fingerprint'], 'source_changed_during_dump')
        real.private_file(BASE / 'source.dump')
        local.command('createdb', '--template=template0', DB)
        local.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', DB, BASE / 'source.dump')
        require(local.fingerprint(DB) == before['candidate_fingerprint'], 'clone_not_exact')
        checksums(DB, 8)
        psql(f'ALTER DATABASE {DB} SET default_transaction_read_only=on', 'postgres')
        migration()
        require(checksums(DB, 9) == 9, 'migration9_missing')
        require(psql("SELECT state||':'||revision FROM application_epoch") == 'prepared:0', 'migration_activated_epoch')
        require(psql("SELECT (SELECT count(*) FROM agent_access)+(SELECT count(*) FROM agents)+(SELECT count(*) FROM worker_health)+(SELECT count(*) FROM epoch_transitions)+(SELECT count(*) FROM live_budgets)") == '0', 'migration_granted_or_executed')
        migration()  # Safe no-op after all checksums agree.
        # The old review template has userinfo and an empty authority; SQLx rejects it.
        invalid_url = url(writable=True).replace('@localhost/', '@/')
        rejected_url = subprocess.run([str(ROOT / 'target/debug/examples/review_cutover'), '--exercise-disposable', 'migrate'],
                                      env={'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'DATABASE_URL': invalid_url},
                                      capture_output=True, timeout=30)
        require(rejected_url.returncode != 0, 'invalid_socket_url_unexpectedly_accepted')
        bad = 'supa_cutover_test_bad_checksum'
        local.command('createdb', '--template=template0', bad)
        local.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', bad, BASE / 'source.dump')
        psql("UPDATE _sqlx_migrations SET checksum=decode(repeat('00',48),'hex') WHERE version=1", bad)
        migration(bad, expected_success=False)
        require(psql('SELECT count(*) FROM _sqlx_migrations', bad) == '8', 'checksum_failure_applied_migration')
        reader, db_password = 'supa_safety_reader', secrets.token_urlsafe(36)
        psql(f"CREATE ROLE {reader} LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{db_password}'; REVOKE CONNECT,TEMPORARY ON DATABASE {DB} FROM PUBLIC; GRANT CONNECT ON DATABASE {DB} TO {reader}", 'postgres')
        psql(f"BEGIN READ WRITE; REVOKE CREATE ON SCHEMA public FROM PUBLIC; GRANT USAGE ON SCHEMA public TO {reader}; GRANT SELECT ON ALL TABLES IN SCHEMA public TO {reader}; COMMIT")
        hba = BASE / 'data/pg_hba.conf'
        original_hba = hba.read_text()
        hba.write_text(f'local {DB} {reader} scram-sha-256\n' + original_hba)
        local.command('pg_ctl', '-D', BASE / 'data', 'reload')
        psql('SELECT 1', user=reader, password='incorrect-fixture-password', accepted=False)
        require(psql('SELECT state FROM application_epoch', user=reader, password=db_password) == 'prepared', 'reader_auth_failed')
        require(psql("SELECT rolpassword LIKE 'SCRAM-SHA-256$%' FROM pg_authid WHERE rolname='supa_safety_reader'", 'postgres') == 't', 'not_scram')
        for sql in ['BEGIN READ WRITE; UPDATE application_epoch SET revision=revision+1',
                    'BEGIN READ WRITE; CREATE TABLE forbidden(id integer)',
                    'BEGIN READ WRITE; TRUNCATE epoch_transitions',
                    f'SET ROLE "{USER}"', 'CREATE TEMP TABLE forbidden(id integer)']:
            psql(sql, user=reader, password=db_password, accepted=False)
        require(psql(f"SELECT count(*) FROM pg_auth_members WHERE member=(SELECT oid FROM pg_roles WHERE rolname='{reader}')", 'postgres') == '0', 'reader_has_membership')
        attrs = psql(f"SELECT rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls FROM pg_roles WHERE rolname='{reader}'", 'postgres')
        require(attrs == 'f', 'reader_privilege_escalation')
        actor = str(uuid.uuid4())
        transition(actor, 'activate_read_only', str(uuid.uuid4()), 1, plan_digest, False)
        transition(actor, 'activate_read_only', str(uuid.uuid4()), 0, 'malformed', False)
        # Demonstrate the current gap: syntax validation is not a trusted plan binding.
        wrong_digest = '0' * 64
        require(wrong_digest != plan_digest, 'test_digest_collision')
        first_approval = str(uuid.uuid4())
        transition(actor, 'activate_read_only', first_approval, 0, wrong_digest)
        require(psql('SELECT plan_sha256 FROM epoch_transitions') == wrong_digest, 'digest_probe_not_observed')
        transition(actor, 'activate_read_only', first_approval, 0, wrong_digest, False)
        transition(actor, 'rollback', str(uuid.uuid4()), 1, plan_digest)
        activation = str(uuid.uuid4())
        transition(actor, 'activate_read_only', activation, 2, plan_digest)
        active_before = local.fingerprint(DB)
        local.command('pg_dump', '--format=custom', '--file', BASE / 'active.dump', database=DB)
        real.private_file(BASE / 'active.dump')
        restored = 'supa_safety_restored_test'
        local.command('createdb', '--template=template0', restored)
        local.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', restored, BASE / 'active.dump')
        require(local.fingerprint(restored) == active_before == local.fingerprint(DB), 'active_backup_inconsistent')
        psql(f'ALTER DATABASE {restored} SET default_transaction_read_only=on', 'postgres')
        require(psql('SELECT count(*) FROM epoch_transitions', restored) == '3', 'restored_audit_missing')
        require(psql('SELECT state FROM application_epoch', restored) == 'active_read_only', 'restored_epoch_missing')
        testdb = 'supa_safety_test'
        local.command('createdb', '--template=template0', testdb)
        env = {'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'TEST_DATABASE_URL': url(testdb)}
        with (BASE / 'rust-tests.log').open('wb') as log:
            result = subprocess.run(['cargo', 'test', '--workspace', '--all-targets', '--locked', '--offline', '--', '--include-ignored'], cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, timeout=600)
        require(result.returncode == 0, 'isolated_rust_tests_failed')
        total = sum(map(int, re.findall(r'test result: ok\. (\d+) passed', (BASE / 'rust-tests.log').read_text())))
        # Runtime configuration uses only synthetic principals and disposable SCRAM secrets.
        synthetic_token, public_token, browser_password = [secrets.token_urlsafe(48) for _ in range(3)]
        synthetic_owner = str(uuid.uuid4())
        runtime = {'root': str(BASE), 'database': DB, 'epoch_id': owner.EPOCH, 'approval_id': activation,
                   'api': {'DATABASE_URL': url(user=reader, password=db_password), 'SUPA_API_TOKEN': public_token,
                           'SUPA_PROVIDER': 'offline', 'SUPA_EPOCH_MODE': 'active_read_only',
                           'SUPA_EPOCH_READONLY_PREVIEW': 'false', 'SUPA_EXPECTED_EPOCH_ID': owner.EPOCH,
                           'SUPA_EPOCH_APPROVAL_ID': activation, 'SUPA_OPERATOR_CREDENTIALS': json.dumps([{'id': synthetic_owner, 'role': 'owner', 'token': synthetic_token}]),
                           **{k: 'false' for k in ['SUPA_ALLOW_OPENAI', 'SUPA_CONTROLLED_INFERENCE', 'SUPA_API_SCHEDULER_ENABLED', 'SUPA_WALLET_ENABLED', 'SUPA_PRIVATE_STAGING']}},
                   'synthetic_username': 'safety-owner', 'synthetic_password': browser_password, 'synthetic_token': synthetic_token}
        save(BASE / 'runtime.json', runtime)
        browser_env = {'PATH': os.environ['PATH'], 'HOME': str(Path.home())}
        if os.environ.get('SUPA_REA_CDP_PORT'):
            browser_env['SUPA_REA_CDP_PORT'] = os.environ['SUPA_REA_CDP_PORT']
        result = subprocess.run(['node', 'scripts/local-safety-browser.mjs', str(BASE / 'runtime.json')], cwd=ROOT, env=browser_env, timeout=600)
        require(result.returncode == 0, 'browser_validation_failed')
        require(snapshot() == before, 'real_protected_state_changed')
        require(checksums(DB, 9) == 9, 'isolated_checksums_changed')
        report = {'cluster': str(BASE.relative_to(ROOT)), 'disposable_cluster_only': True,
                  'service_profile': {'proposed_api_bind': '127.0.0.1:8080', 'proposed_frontend_port': 3200,
                                      'actual_tests_use_ephemeral_loopback_ports': True, 'scram_reader_authenticated': True,
                                      'wrong_password_rejected': True, 'write_ddl_temp_and_set_role_denials': 5,
                                      'reader_role_memberships': 0, 'operator_service_credentials_distinct': True,
                                      'legacy_shared_owner_configuration_absent': True,
                                      'proposed_empty_authority_url_rejected': True,
                                      'corrected_explicit_host_socket_override_verified': True},
                  'migration': {'real_successful_migrations': 8, 'clone_before': 8, 'clone_after': 9,
                                'all_sqlx_sha384_checksums_verified': True, 'corrupted_checksum_rejected': True,
                                'repeated_migration_noop': True, 'migration_did_not_activate_grant_or_start_workers': True,
                                'administrative_privileges_are_not_a_sandbox': True},
                  'backup': {'source_prepared_clone_exact': True, 'active_restore_matches_all_table_rows_schema_constraints_indexes_sequences': True,
                             'audit_records_preserved': 3, 'epoch_metadata_preserved': True, 'independent_destination': None,
                             'private_dump_mode': '0600', 'same_disk_only': True, 'historical_ledger_not_imported': True},
                  'approval_guards': {'stale_revision_rejected': True, 'malformed_digest_rejected': True,
                                      'duplicate_approval_rejected': True, 'well_formed_wrong_digest_accepted': True,
                                      'trusted_plan_digest_binding_enforced': False},
                  'rust_tests_passed': total, 'real_snapshot_unchanged': True,
                  'real_epoch': 'prepared', 'real_owner': 'prepared_ungranted', 'runtime_credentials_installed': False,
                  'primary_frontend_repointed': False, 'historical_hold_microusd': 4048, 'historical_ceiling_microusd': 100000,
                  'historical_database': 'unrecovered', 'activation_ready': False,
                  'blocking_findings': ['Plan digest is syntax-checked and recorded but not compared with a trusted approved manifest.',
                                        'Proposed DATABASE_URL userinfo with empty authority is rejected by SQLx; use a verified explicit-host socket override.',
                                        'Real-target migration-only runner and active operational profile remain unapproved/uninstalled.'],
                  'provider_requests': 0, 'uploads': 0, 'real_database_mutations': 0}
        save(BASE / 'result.json', report)
    finally:
        if started:
            local.command('pg_ctl', '-D', BASE / 'data', '-m', 'fast', '-w', 'stop')
    report['disposable_cluster_stopped'] = True
    (OUT / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'validation_complete': True, 'activation_ready': False, 'rust_tests_passed': total, 'real_epoch': 'prepared'}))


if __name__ == '__main__':
    try:
        run()
    except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        code = str(error) if isinstance(error, ValueError) and re.fullmatch(r'[a-z0-9_]+', str(error)) else type(error).__name__
        print(json.dumps({'error': 'disposable_safety_validation_failed', 'code': code, 'real_activation_attempted': False}), file=sys.stderr)
        sys.exit(2)
