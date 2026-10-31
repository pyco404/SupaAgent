#!/usr/bin/env python3
"""Milestone 8 rehearsal. Only creates fresh disposable clusters; never starts primary."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import uuid
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parent.parent
BACKUP = ROOT / 'postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump'
BACKUP_SHA = '600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671'
EPOCH = 'fd2f52e0-33fc-4c0d-893a-7b08b26ab748'
OWNER = '6fd863e1-745d-4a06-b68e-eea69e854a69'
CUSTODIAN = 'e8c6ade3-917f-4fe4-a092-f8a58eccb487'


def module(name, file):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / file)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value


def require(condition, code):
    if not condition:
        raise ValueError(code)


def validate_plan(value):
    require(value['kind'] == 'unsigned_local_onboarding_proposal' and value['approved'] is False, 'unsigned_plan_required')
    require(value['epoch_id'] == EPOCH and value['proposed_owner_principal'] == OWNER, 'proposal_identity_mismatch')
    require(value['agent_id'] is None and value['initial_lifecycle'] == 'created', 'new_inactive_identity_only')
    require(value['live_spending_authorized'] is False and value['worker_launch_authorized'] is False and value['devnet_recipient'] is None, 'execution_not_authorized')
    request = value['agent_request']
    require(set(request) == {'name', 'capabilities', 'config'}, 'identity_override_forbidden')
    require(set(request['capabilities']) == {'read_memory', 'remember', 'evaluate_goal'}, 'offline_capabilities_only')
    require(request['config'] == {'scheduler_enabled': False, 'interval_seconds': 60, 'max_steps': 3, 'timeout_seconds': 5, 'retry_budget': 0, 'max_provider_calls': 1, 'max_cycle_tokens': 150000, 'inference_cost_microusd': 1000, 'operating_cost_microusd': 100, 'draft_cost_microusd': 1000}, 'reviewed_inactive_configuration_required')
    policy = value['future_offline_policy']
    require(policy == {'interval_seconds': 60, 'max_cycles': 3, 'simulated_budget_microusd': 3300, 'valid_for_seconds': 180}, 'reviewed_finite_limits_required')
    require(len(value['goals']) == 2 and all(isinstance(goal, str) and 1 <= len(goal) <= 1000 for goal in value['goals']), 'explicit_goals_required')
    return value


def baseline():
    strict = module('onboarding_strict', 'validate-stage-two-migrations.py')
    hashes = strict.protected()  # Offline file reads only; rejects a running primary.
    previous = json.loads((ROOT / 'artifacts/milestone-7a/protected-hashes.json').read_text())
    require(all(hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest for path, digest in previous.items()), 'previous_protected_evidence_drift')
    require(hashlib.sha256(BACKUP.read_bytes()).hexdigest() == BACKUP_SHA, 'approved_backup_hash_mismatch')
    owner = module('onboarding_owner', 'owner-provisioning.py')
    identity = owner.load()
    owner.credentials()  # Validate private-file modes and distinct values; never render them.
    owner.ignored()
    custodian = json.loads((ROOT / 'postgres-data/custodian-provisioning/identity.json').read_text())
    require(identity['principal_id'] == OWNER and custodian['principal_id'] == CUSTODIAN, 'proposed_identity_drift')
    require(custodian['state'] == 'proposed_unverified' and not any(custodian[k] for k in ['identity_binding_verified', 'independent_credential_control_verified', 'credentials_prepared', 'credentials_installed', 'authority_granted', 'database_login_created']), 'custodian_must_remain_unverified')
    epoch = json.loads((ROOT / 'postgres-data/development-epoch.json').read_text())
    require(epoch['id'] == EPOCH and epoch['state'] == 'prepared' and epoch['activation_approved'] is False, 'prepared_unapproved_epoch_required')
    policy = json.loads((ROOT / 'postgres-data/epoch-backup-policy.json').read_text())
    require(policy['destination'] is None and policy['upload_enabled'] is False and policy['transfer_authorized'] is False, 'independent_backup_must_stay_unset')
    require(not Path('/etc/supa-agent/migration-authority.json').exists(), 'unexpected_installed_authority_stop_review')
    validate_plan(json.loads((ROOT / 'config/first-agent-plan.json').read_text()))
    return strict, hashes


def acl_expression(column, kind, owner):
    # pg_dump can replace an explicit default ACL with NULL. Compare effective
    # privileges, including grantor/grant option, rather than array representation.
    return f"(SELECT coalesce(jsonb_agg(jsonb_build_array(CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,pg_get_userbyid(a.grantor),a.privilege_type,a.is_grantable) ORDER BY CASE WHEN a.grantee=0 THEN 'PUBLIC' ELSE pg_get_userbyid(a.grantee) END,pg_get_userbyid(a.grantor),a.privilege_type,a.is_grantable),'[]') FROM aclexplode(coalesce({column},acldefault({kind},{owner}))) a)"


def catalog(pg, database):
    relation_acl = acl_expression('c.relacl', '(CASE WHEN c.relkind=\'S\' THEN \'s\' ELSE \'r\' END)::"char"', 'c.relowner')
    schema_acl = acl_expression('nspacl', '\'n\'::"char"', 'nspowner')
    function_acl = acl_expression('p.proacl', '\'f\'::"char"', 'p.proowner')
    queries = ["BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
        f"SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY schema,name),'[]') FROM (SELECT n.nspname AS schema,c.relname AS name,c.relkind,pg_get_userbyid(c.relowner) AS owner,{relation_acl} AS privileges FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname<>'information_schema') x",
        f"SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY name),'[]') FROM (SELECT nspname AS name,pg_get_userbyid(nspowner) AS owner,{schema_acl} AS privileges FROM pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname<>'information_schema') x",
        f"SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY schema,signature),'[]') FROM (SELECT n.nspname AS schema,p.oid::regprocedure::text AS signature,pg_get_functiondef(p.oid) AS definition,{function_acl} AS privileges,pg_get_userbyid(p.proowner) AS owner FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname<>'information_schema' AND p.prokind IN ('f','p')) x",
        "SELECT coalesce(jsonb_agg(pg_get_triggerdef(t.oid) ORDER BY n.nspname,c.relname,t.tgname),'[]') FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname<>'information_schema' AND NOT t.tgisinternal", 'COMMIT']
    return hashlib.sha256(pg.sql(';'.join(queries), database).encode()).hexdigest()


def check_sequence_progress(before, after):
    require(set(before) == set(after), 'sequence_inventory_changed')
    require(all(value is None or (after[name] is not None and after[name] >= value) for name, value in before.items()), 'sequence_rewound_after_crash')


def sequence_snapshot(pg, database):
    rows = json.loads(pg.sql("SELECT coalesce(json_agg(json_build_object('schema',schemaname,'name',sequencename,'value',last_value,'increment',increment_by) ORDER BY schemaname,sequencename),'[]') FROM pg_sequences WHERE schemaname NOT LIKE 'pg_%' AND schemaname<>'information_schema'", database))
    require(all(row['increment'] > 0 for row in rows), 'positive_application_sequences_required')
    values = {row['schema'] + '.' + row['name']: row['value'] for row in rows}
    owned = json.loads(pg.sql("SELECT coalesce(json_agg(json_build_array(ns.nspname,s.relname,nt.nspname,t.relname,a.attname)),'[]') FROM pg_class s JOIN pg_namespace ns ON ns.oid=s.relnamespace JOIN pg_depend d ON d.classid='pg_class'::regclass AND d.objid=s.oid AND d.deptype IN ('a','i') JOIN pg_class t ON t.oid=d.refobjid JOIN pg_namespace nt ON nt.oid=t.relnamespace JOIN pg_attribute a ON a.attrelid=t.oid AND a.attnum=d.refobjsubid WHERE s.relkind='S' AND ns.nspname NOT LIKE 'pg_%' AND ns.nspname<>'information_schema'", database))
    quote = lambda name: '"' + name.replace('"', '""') + '"'
    for schema, sequence, table_schema, table, column in owned:
        maximum = pg.sql('SELECT coalesce(max(' + quote(column) + '),0) FROM ' + quote(table_schema) + '.' + quote(table), database)
        value = values[schema + '.' + sequence]
        require(int(maximum) == 0 or (value is not None and value >= int(maximum)), 'sequence_below_committed_identity')
    return values


def exercise(rea_port=None):
    strict, before = baseline()
    run_id = uuid.uuid4().hex[:12]
    output = ROOT / 'artifacts/milestone-8' / run_id
    output.mkdir(mode=0o700)
    (output / 'baseline-protected.json').write_text(json.dumps(before, indent=2) + '\n')
    clean = {k: os.environ[k] for k in ['PATH', 'HOME']}
    # Build only local code; no dotenv, providers or network installation.
    with (output / 'build.log').open('wb') as log:
        subprocess.run(['cargo', 'build', '--workspace', '--examples', '--locked', '--offline'], cwd=ROOT, env=clean, check=True, stdout=log, stderr=subprocess.STDOUT)
    strict.OUT = output / 'strict-migration-tests'
    strict.BASE = ROOT / 'postgres-data/stage-two' / ('m8_' + run_id)
    strict.run()  # Existing signed 9–10/interruption tests and complete Rust/database suite.
    pg = module('onboarding_pg', 'persistent-postgres.py')
    base = ROOT / 'postgres-data/onboarding-validation' / run_id
    pg.private_directory(base)
    pg.private_directory(base / 'socket')
    pg.exclusive_json(base / 'scope.json', {'purpose': 'disposable_milestone_8', 'primary_mutations_authorized': False})
    def env(database='postgres'):
        return {**clean, 'LANG': 'C.UTF-8', 'PGHOST': str(base / 'socket'), 'PGPORT': '55452', 'PGUSER': 'pikoo', 'PGDATABASE': database, 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null', 'PGCONNECT_TIMEOUT': '5', 'PGOPTIONS': '-c timezone=UTC'}
    pg.environment = env
    clone, work = 'supa_cutover_test_m8_clone', 'supa_cutover_test_onboarding_test'
    def url(database):
        return 'postgresql://pikoo@localhost/' + database + '?' + urlencode({'host': str(base / 'socket'), 'port': 55452})
    started = False
    try:
        pg.command('initdb', '-D', base / 'data', '--auth-local=peer', '--auth-host=reject', '--encoding=UTF8', '--locale=C.UTF-8', '--data-checksums')
        with (base / 'data/postgresql.conf').open('a') as file:
            file.write(f"\nlisten_addresses=''\nunix_socket_directories='{base / 'socket'}'\nunix_socket_permissions=0700\nport=55452\ntimezone='UTC'\nmax_connections=100\nfsync=on\nsynchronous_commit=on\nfull_page_writes=on\n")
        pg.command('pg_ctl', '-D', base / 'data', '-l', base / 'server.log', '-w', 'start')
        started = True
        require(pg.sql('SHOW data_directory', 'postgres') == str(base / 'data') and pg.sql('SHOW listen_addresses', 'postgres') == '', 'disposable_cluster_identity_mismatch')
        pg.command('createdb', '--template=template0', clone)
        pg.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', clone, BACKUP)
        require(pg.sql('SELECT max(version) FROM _sqlx_migrations', clone) == '8', 'restore_must_start_at_8')
        require(pg.sql('SELECT id::text||\':\'||state FROM application_epoch', clone) == EPOCH + ':prepared', 'restored_epoch_mismatch')
        tables = json.loads(pg.sql("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='public' AND tablename<>'_sqlx_migrations'", clone))
        columns = {table: json.loads(pg.sql("SELECT json_agg(column_name ORDER BY ordinal_position) FROM information_schema.columns WHERE table_schema='public' AND table_name='" + table + "'", clone)) for table in tables}
        def original_rows():
            return {table: pg.sql('SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),\'[]\') FROM (SELECT ' + ','.join('"' + c + '"' for c in columns[table]) + ' FROM "' + table + '") t', clone) for table in tables}
        original = original_rows()
        with (output / 'clone-migration.log').open('wb') as log:
            subprocess.run([str(ROOT / 'target/debug/examples/review_cutover'), '--exercise-disposable', 'migrate'], cwd=ROOT, env={**clean, 'DATABASE_URL': url(clone)}, stdout=log, stderr=subprocess.STDOUT, check=True)
        require(original_rows() == original, 'original_restored_rows_changed')
        rows = json.loads(pg.sql("SELECT json_agg(json_build_object('version',version,'success',success,'checksum',encode(checksum,'hex')) ORDER BY version) FROM _sqlx_migrations", clone))
        require([r['version'] for r in rows] == list(range(1, 12)), 'eleven_migrations_required')
        for row in rows:
            file, = (ROOT / 'migrations').glob(f"{row['version']:04d}_*.sql")
            require(row['success'] and row['checksum'] == hashlib.sha384(file.read_bytes()).hexdigest(), 'sqlx_checksum_mismatch')
        require(pg.sql("SELECT state||':'||revision FROM application_epoch", clone) == 'prepared:0', 'clone_migration_activated_epoch')
        empty = ['agents', 'agent_access', 'live_budgets', 'live_sessions', 'schedule_authorizations', 'simulated_ledger', 'economic_entries', 'economic_jobs', 'epoch_guard.approvals', 'epoch_guard.authorities', 'epoch_guard.executors', 'epoch_guard.consumptions']
        for table in empty:
            require(pg.sql('SELECT count(*) FROM ' + table, clone) == '0', 'migration_created_operational_state')
        pg.sql('CREATE ROLE m8_readonly NOLOGIN; GRANT USAGE ON SCHEMA public,epoch_guard TO m8_readonly; GRANT SELECT ON ALL TABLES IN SCHEMA public TO m8_readonly', clone)
        require(pg.sql("SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='epoch_guard' AND p.proname<>'verify_active' AND has_function_privilege('m8_readonly',p.oid,'EXECUTE')", clone) == '0', 'readonly_role_can_execute_approval_functions')
        for statement in ["SET ROLE m8_readonly; UPDATE application_epoch SET label='forged'", "SET ROLE m8_readonly; INSERT INTO agents(id) VALUES(gen_random_uuid())", "SET ROLE m8_readonly; CREATE TABLE public.forbidden(id int)"]:
            try:
                pg.sql(statement, clone)
            except ValueError:
                pass
            else:
                raise ValueError('readonly_permission_bypass')
        pg.sql('ALTER DATABASE ' + clone + ' SET default_transaction_read_only=on', 'postgres')
        require(pg.sql('SHOW default_transaction_read_only', clone) == 'on', 'restored_readonly_profile_missing')
        pg.command('createdb', '--template=template0', work)
        with (output / 'operational-schema.log').open('wb') as log:
            subprocess.run([str(ROOT / 'target/debug/examples/review_cutover'), '--exercise-disposable', 'migrate'], cwd=ROOT, env={**clean, 'DATABASE_URL': url(work)}, stdout=log, stderr=subprocess.STDOUT, check=True)
        node_env = {**clean, 'TEST_DATABASE_URL': url(work), 'SUPA_ONBOARDING_EVIDENCE_DIR': str(output)}
        if rea_port:
            node_env['SUPA_ONBOARDING_REA_PORT'] = str(rea_port)
        with (output / 'onboarding.log').open('wb') as log:
            subprocess.run(['node', 'scripts/onboarding-smoke.mjs'], cwd=ROOT, env=node_env, stdout=log, stderr=subprocess.STDOUT, check=True)
        report = json.loads((output / 'onboarding.json').read_text())
        fingerprints = {}
        for database, restored in [(clone, 'supa_m8_clone_restored_test'), (work, 'supa_m8_operational_restored_test')]:
            fingerprint, acl = pg.fingerprint(database), catalog(pg, database)
            restart_fingerprint = pg.fingerprint(database, include_sequence_values=False)
            sequences = sequence_snapshot(pg, database)
            archive = output / (database + '.dump')
            pg.command('pg_dump', '--format=custom', '--file', archive, database=database)
            archive.chmod(0o600)
            require(pg.fingerprint(database) == fingerprint and catalog(pg, database) == acl, 'backup_source_changed')
            pg.command('createdb', '--template=template0', restored)
            pg.command('pg_restore', '--exit-on-error', '--single-transaction', '--dbname', restored, archive)
            if database == clone:
                pg.sql('ALTER DATABASE ' + restored + ' SET default_transaction_read_only=on', 'postgres')
            require(pg.fingerprint(restored) == fingerprint and catalog(pg, restored) == acl, 'restore_integrity_or_acl_mismatch')
            if database == clone:
                for grant, revoke in [
                    ('GRANT UPDATE ON application_epoch TO m8_readonly', 'REVOKE UPDATE ON application_epoch FROM m8_readonly'),
                    ('GRANT EXECUTE ON FUNCTION epoch_guard.approve(uuid,text,timestamptz) TO PUBLIC', 'REVOKE EXECUTE ON FUNCTION epoch_guard.approve(uuid,text,timestamptz) FROM PUBLIC')]:
                    pg.sql('BEGIN READ WRITE;' + grant + ';COMMIT', restored)
                    require(catalog(pg, restored) != acl, 'effective_privilege_escalation_not_detected')
                    pg.sql('BEGIN READ WRITE;' + revoke + ';COMMIT', restored)
                    require(catalog(pg, restored) == acl, 'restored_privileges_not_returned_to_baseline')
            fingerprints[database] = {'state_sha256': fingerprint, 'restart_state_sha256': restart_fingerprint, 'sequence_values_before_restart': sequences, 'catalog_sha256': acl, 'restored_database': restored, 'archive_sha256': hashlib.sha256(archive.read_bytes()).hexdigest()}
        pg.command('pg_dumpall', '--globals-only', '--no-role-passwords', '--file', output / 'disposable-roles.sql')
        (output / 'disposable-roles.sql').chmod(0o600)
        pg.command('pg_ctl', '-D', base / 'data', '-m', 'immediate', '-w', 'stop')
        started = False
        pg.command('pg_ctl', '-D', base / 'data', '-l', base / 'server.log', '-w', 'start')
        started = True
        for database, evidence in fingerprints.items():
            for db in [database, evidence['restored_database']]:
                require(pg.fingerprint(db, include_sequence_values=False) == evidence['restart_state_sha256'] and catalog(pg, db) == evidence['catalog_sha256'], 'crash_restart_changed_persisted_records')
                check_sequence_progress(evidence['sequence_values_before_restart'], sequence_snapshot(pg, db))
        require(pg.sql("SELECT state||':'||revision FROM application_epoch", clone) == 'prepared:0', 'restart_activated_clone')
        require(pg.sql("SELECT data->>'lifecycle' FROM agents WHERE id='" + report['synthetic_agent_id'] + "'", work) == 'suspended', 'restarted_fixture_reactivated')
        require(strict.protected() == before, 'protected_primary_or_accounting_evidence_changed')
        result = {'passed': True, 'source_backup_sha256': BACKUP_SHA, 'primary_started': False, 'primary_connected': False, 'primary_last_verified_schema': 8, 'primary_state': 'prepared', 'migrations_tested': list(range(1, 12)), 'original_source_tables_preserved': len(tables), 'prepared_clone_state': 'prepared:0', 'clone_operational_rows_created': 0, 'checksums_verified': True, 'readonly_permissions_verified': True, 'effective_acl_round_trip_verified': True, 'privilege_escalation_detection_verified': True, 'backups': fingerprints, 'disposable_crash_restart_verified': True, 'sequence_forward_progress_verified': True, 'protected_files_unchanged': len(before), 'historical_database': 'unrecovered', 'archived_budget_uuid': pg.ARCHIVED, 'archived_ceiling_microusd': 100000, 'unresolved_hold_microusd': 4048, 'independent_backup_destination': None, 'real_owner_or_custodian_grants': 0, 'real_credentials_installed': False, 'primary_frontend_repointed': False, 'recipient': None, 'onboarding': report, 'operating_epoch_write_path_ready': False, 'primary_migration_11_entry_point_ready': True}
    finally:
        if started:
            pg.command('pg_ctl', '-D', base / 'data', '-m', 'fast', '-w', 'stop')
    result['all_disposable_clusters_stopped'] = True
    (output / 'validation.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'passed': True, 'evidence': str(output.relative_to(ROOT)), 'primary_unchanged': True}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['inspect', 'exercise-disposable'])
    parser.add_argument('--rea-cdp-port', type=int)
    args = parser.parse_args()
    if args.rea_cdp_port is not None and not 1024 <= args.rea_cdp_port <= 65535:
        parser.error('loopback CDP port required')
    os.umask(0o077)
    if args.command == 'inspect':
        _, hashes = baseline()
        print(json.dumps({'primary_stopped': True, 'inspection': 'offline_only', 'last_verified_schema': 8, 'epoch': EPOCH, 'state': 'prepared', 'owner': OWNER, 'owner_granted': False, 'custodian': CUSTODIAN, 'custodian_verified': False, 'protected_files': len(hashes), 'approved_backup_verified': True, 'archived_hold_microusd': 4048, 'recipient': None, 'write_epoch_activation_supported': False}))
    else:
        exercise(args.rea_cdp_port)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, subprocess.SubprocessError, OSError) as error:
        code = str(error) if isinstance(error, ValueError) and re.fullmatch(r'[a-z_0-9]+', str(error)) else 'inspect_private_evidence'
        print('local_onboarding_validation_failed_' + code, file=sys.stderr)
        raise SystemExit(1)
