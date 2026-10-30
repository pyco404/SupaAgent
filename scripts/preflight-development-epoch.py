#!/usr/bin/env python3
"""Explicit Stage 1 only: inspect primary, back up, restore separately, stop. Never migrate."""
import argparse
import datetime
import fcntl
import getpass
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import uuid

ROOT = Path(__file__).resolve().parent.parent
EPOCH = 'fd2f52e0-33fc-4c0d-893a-7b08b26ab748'
OWNER = '6fd863e1-745d-4a06-b68e-eea69e854a69'
NAMESPACE = '641457b2-5fd8-4b31-b85f-c58839626831'
DB = 'supa_epoch_dev_' + uuid.UUID(EPOCH).hex
REFERENCE = ROOT / 'postgres-data/epoch-backups/93ae651e-8bd7-4d68-92dc-a9ee29038479'
spec = importlib.util.spec_from_file_location('preflight_pg', ROOT / 'scripts/persistent-postgres.py')
pg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pg)


def require(test, code):
    if not test:
        raise ValueError(code)


def scalar(statement, database=DB):
    return pg.sql(statement, database)


def catalog(database):
    # Additional protection beyond row/schema fingerprints: ACLs, functions, triggers and database settings.
    statements = ["BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
        "SELECT jsonb_agg(to_jsonb(x) ORDER BY name) FROM (SELECT n.nspname||'.'||c.relname AS name,c.relkind,pg_get_userbyid(c.relowner) AS owner,c.relacl FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public') x",
        "SELECT jsonb_agg(to_jsonb(x) ORDER BY name) FROM (SELECT n.nspname AS name,pg_get_userbyid(n.nspowner) AS owner,n.nspacl FROM pg_namespace n WHERE n.nspname NOT LIKE 'pg_%' AND n.nspname<>'information_schema') x",
        "SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY signature),'[]') FROM (SELECT p.oid::regprocedure::text AS signature,pg_get_functiondef(p.oid) AS definition,p.proacl,pg_get_userbyid(p.proowner) AS owner FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.prokind IN ('f','p')) x",
        "SELECT coalesce(jsonb_agg(pg_get_triggerdef(t.oid) ORDER BY t.tgname),'[]') FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND NOT t.tgisinternal",
        'COMMIT']
    return hashlib.sha256(scalar(';'.join(statements), database).encode()).hexdigest()


def schema_digest(database=None, archive=None):
    text = pg.command('pg_restore','--schema-only','--file=-',archive) if archive else pg.command('pg_dump','--schema-only',database=database)
    # Dump-tool metadata and randomized psql restriction markers are not schema changes.
    lines = [line for line in text.splitlines() if not line.startswith(('-- Dumped from database version','-- Dumped by pg_dump version',chr(92)+'restrict ',chr(92)+'unrestrict '))]
    return hashlib.sha256(('\n'.join(lines)+'\n').encode()).hexdigest()


def protected():
    paths = [pg.BASE / 'identity.json', pg.BASE / 'data/postgresql.conf', pg.BASE / 'data/pg_hba.conf',
             ROOT / 'postgres-data/development-epoch.json', ROOT / 'postgres-data/epoch-backup-policy.json',
             ROOT / 'postgres-data/owner-provisioning/identity.json', ROOT / 'postgres-data/owner-provisioning/credentials.pending.json',
             ROOT / 'web/.env.local', REFERENCE / 'manifest.json', REFERENCE / 'database.dump']
    paths += list((ROOT / 'artifacts/live-eval').glob('*'))
    paths += [ROOT / 'artifacts/persistent-postgres/verification.json', ROOT / 'docs/persistent-postgres-accounting.md']
    require(all(p.is_file() and not p.is_symlink() for p in paths), 'protected_file_missing_or_symlink')
    return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}


def verify_source():
    pg.identity()
    config = json.loads(pg.private_file(ROOT / 'postgres-data/development-epoch.json').read_text())
    require(config['id']==EPOCH and config['database']==DB and config['state']=='prepared', 'prepared_configuration_mismatch')
    epoch = json.loads(scalar("SELECT to_jsonb(e)-'singleton'-'created_at' FROM application_epoch e"))
    require(epoch==config['epoch'] and epoch['id']==EPOCH and epoch['accounting_namespace']==NAMESPACE and epoch['state']=='prepared' and epoch['historical_database']=='unrecovered', 'epoch_mismatch')
    require(scalar('SHOW default_transaction_read_only')=='on', 'database_not_frozen')
    require(scalar('SHOW port')=='55440' and scalar('SHOW unix_socket_directories')==str(pg.BASE / 'socket') and scalar('SHOW unix_socket_permissions')=='0700', 'socket_profile_mismatch')
    require(all(scalar('SHOW '+name)=='' for name in ['shared_preload_libraries','session_preload_libraries','local_preload_libraries']), 'unexpected_preload')
    require(scalar("SELECT count(*) FROM pg_hba_file_rules WHERE error IS NOT NULL OR (type='local' AND auth_method<>'peer') OR (type LIKE 'host%' AND auth_method<>'reject')", 'postgres')=='0', 'authentication_rules_mismatch')
    roles = json.loads(scalar("SELECT json_agg(rolname ORDER BY rolname) FROM pg_roles WHERE rolname NOT LIKE 'pg_%'", 'postgres'))
    require(roles==[getpass.getuser()], 'unexpected_login_or_custom_role')
    require(scalar("SELECT count(*) FROM pg_auth_members m JOIN pg_roles r ON r.oid=m.roleid JOIN pg_roles u ON u.oid=m.member WHERE r.rolname NOT LIKE 'pg_%' OR u.rolname NOT LIKE 'pg_%'", 'postgres')=='0', 'unexpected_role_membership')
    require(scalar("SELECT count(*) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public' AND (c.relacl IS NOT NULL OR pg_get_userbyid(c.relowner)<>current_user)")=='0', 'unexpected_object_owner_or_grant')
    require(scalar("SELECT count(*) FROM pg_database WHERE datname=current_database() AND (datacl IS NOT NULL OR pg_get_userbyid(datdba)<>current_user)")=='0', 'unexpected_database_grant')
    migrations = json.loads(scalar("SELECT json_agg(json_build_object('version',version,'sha384',encode(checksum,'hex'),'success',success) ORDER BY version) FROM _sqlx_migrations"))
    require([r['version'] for r in migrations]==list(range(1,9)) and all(r['success'] for r in migrations), 'migration_inventory_changed')
    for r in migrations:
        file, = (ROOT / 'migrations').glob(f"{r['version']:04d}_*.sql")
        require(hashlib.sha384(file.read_bytes()).hexdigest()==r['sha384'], 'migration_checksum_mismatch')
    tables = ['agents','agent_access','schedule_authorizations','live_budgets','live_sessions','inference_calls','worker_health','simulated_ledger']
    counts = {name:int(scalar(f'SELECT count(*) FROM "{name}"')) for name in tables}
    require(not any(counts.values()), 'unexpected_agent_authorization_or_accounting')
    require(scalar("SELECT count(*) FROM pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname NOT IN ('public','information_schema')")=='0', 'unexpected_security_schema')
    return {'epoch':epoch,'migrations':migrations,'counts':counts,'roles':roles,'catalog_sha256':catalog(DB),'state_sha256':pg.fingerprint(DB)}


def prepare_custodian():
    base = ROOT / 'postgres-data/custodian-provisioning'
    pg.private_directory(base)
    file = base / 'identity.json'
    if file.exists() or file.is_symlink():
        record=json.loads(pg.private_file(file).read_text())
        require(record.get('epoch_id')==EPOCH and record.get('display_name')=='KI Daniel' and record.get('state')=='proposed_unverified' and all(record.get(key) is False for key in ['authority_granted','identity_binding_verified','independent_credential_control_verified','credentials_prepared','credentials_installed','database_login_created']), 'custodian_proposal_mismatch')
    else:
        record={'display_name':'KI Daniel','principal_id':str(uuid.uuid4()),'epoch_id':EPOCH,'duty':'independent_activation_custodian','state':'proposed_unverified','identity_binding_verified':False,'independent_credential_control_verified':False,'credentials_prepared':False,'credentials_installed':False,'authority_granted':False,'database_login_created':False}
        pg.exclusive_json(file,record)
    require(uuid.UUID(record['principal_id']).int!=0 and str(uuid.UUID(record['principal_id']))==record['principal_id'] and record['principal_id'] not in [OWNER,EPOCH,NAMESPACE,pg.ARCHIVED], 'custodian_must_be_distinct')
    return record


def run():
    os.umask(0o077)
    before_files=protected()
    baseline=json.loads(pg.private_file(REFERENCE / 'manifest.json').read_text())
    require(pg.digest(REFERENCE / 'database.dump')==baseline['database_sha256'], 'reviewed_backup_corrupt')
    require(not (pg.BASE / 'data/postmaster.pid').exists(), 'cluster_already_running_stop_review')
    pg.private_directory(ROOT / 'postgres-data/preflight')
    run_id=str(uuid.uuid4()); folder=ROOT / 'postgres-data/preflight' / run_id
    pg.private_directory(folder)
    started=False
    try:
        pg.start(); started=True
        before=verify_source()
        reviewed_schema=schema_digest(archive=REFERENCE / 'database.dump')
        require(schema_digest(database=DB)==reviewed_schema,'schema_permissions_or_security_differs_from_reviewed_backup')
        require(before['state_sha256']==baseline['state_sha256'], 'source_differs_from_reviewed_backup')
        dump=folder / 'database.dump'
        pg.command('pg_dump','--format=custom','--file',dump,database=DB)
        pg.private_file(dump)
        require(pg.fingerprint(DB)==before['state_sha256'] and catalog(DB)==before['catalog_sha256'], 'source_changed_during_backup')
        manifest={'stage':1,'epoch_id':EPOCH,'source_database':DB,'database_sha256':pg.digest(dump),'state_sha256':before['state_sha256'],'catalog_sha256':before['catalog_sha256'],'schema_sha256':reviewed_schema,'migrations':before['migrations'],'same_disk_only_accepted':True,'independent_destination':None,'upload_authorized':False,'historical_recovered':False}
        pg.exclusive_json(folder / 'manifest.json',manifest)
        restored='supa_epoch_preflight_test_'+uuid.uuid4().hex
        require(scalar(f"SELECT count(*) FROM pg_database WHERE datname='{restored}'",'postgres')=='0','restore_destination_exists')
        pg.command('createdb','--template=template0',restored)
        pg.command('pg_restore','--exit-on-error','--single-transaction','--dbname',restored,dump)
        require(pg.fingerprint(restored)==before['state_sha256'] and catalog(restored)==before['catalog_sha256'] and schema_digest(database=restored)==reviewed_schema, 'restoration_integrity_mismatch')
        scalar(f'ALTER DATABASE "{restored}" SET default_transaction_read_only=on','postgres')
        require(scalar('SHOW default_transaction_read_only',restored)=='on', 'restore_not_read_only')
        require(verify_source()==before and schema_digest(database=DB)==reviewed_schema and protected()==before_files, 'source_or_protected_files_changed')
        report={'stage':1,'completed_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'database':DB,'epoch_id':EPOCH,'accounting_namespace':NAMESPACE,'state':'prepared','successful_migrations':8,'migration_checksums_verified':True,'source_matches_reviewed_backup':True,'source_unchanged':True,'source_state_sha256':before['state_sha256'],'source_catalog_sha256':before['catalog_sha256'],'source_schema_sha256':reviewed_schema,'backup_directory':str(folder.relative_to(ROOT)),'backup_sha256':manifest['database_sha256'],'backup_mode':'0600','backup_verified':True,'restored_database':restored,'restored_state_and_catalog_match':True,'restore_read_only':True,'unexpected_grants':False,'inventory':before['counts'],'socket_only':True,'durability_settings_verified':True,'database_read_only':True,'same_disk_only_accepted':True,'independent_backup_destination':None,'historical_database':'unrecovered','archived_budget_uuid':pg.ARCHIVED,'historical_ceiling_microusd':100000,'historical_unresolved_hold_microusd':4048,'historical_evidence_unchanged':True,'migrations_applied':0,'owner_grants':0,'custodian_grants':0,'credentials_installed':False,'epoch_activated':False,'frontend_repointed':False,'workers_started':0,'provider_requests':0,'uploads':0}
    except Exception as error:
        pg.exclusive_json(folder / 'failure.json',{'stage':1,'stopped':True,'code':str(error) if isinstance(error,ValueError) else type(error).__name__})
        raise
    finally:
        if started or (pg.BASE / 'data/postmaster.pid').exists():
            pg.command('pg_ctl','-D',pg.BASE / 'data','-m','fast','-w','stop')
    report['cluster_stopped_after_preflight']=True
    report['custodian_proposal']=prepare_custodian()
    pg.exclusive_json(folder / 'verification.json',report)
    print(json.dumps(report,indent=2))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--approved-stage-one',action='store_true',required=True)
    parser.parse_args()
    # Serialize local preparation; never inherit application credentials or invoke API/worker binaries.
    lockpath=ROOT / 'postgres-data/preflight-stage-one.lock'
    fd=os.open(lockpath,os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600)
    with os.fdopen(fd,'w') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        run()
