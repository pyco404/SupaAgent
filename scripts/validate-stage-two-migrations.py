#!/usr/bin/env python3
"""Stage 2 validation: isolated clones only. Never starts/connects to the primary cluster."""
import getpass
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import uuid
from urllib.parse import urlencode

ROOT=Path(__file__).resolve().parent.parent
SOURCE=ROOT/'postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833'
BASE=ROOT/'postgres-data/stage-two'/uuid.uuid4().hex[:12]
OUT=ROOT/'artifacts/stage-two-migrations'
USER=getpass.getuser()

def module(name,file):
    spec=importlib.util.spec_from_file_location(name,ROOT/'scripts'/file)
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
pg=module('stage_two_pg','persistent-postgres.py')
preflight=module('stage_two_protected','preflight-development-epoch.py')

def require(ok,code):
    if not ok:raise ValueError(code)

def environment(database='postgres'):
    return {'PATH':'/usr/bin:/bin','HOME':str(Path.home()),'LANG':'C.UTF-8','PGHOST':str(BASE/'s'),'PGPORT':'55444','PGUSER':USER,'PGDATABASE':database,'PGCONNECT_TIMEOUT':'5','PGPASSFILE':'/dev/null','PGSERVICEFILE':'/dev/null','PGOPTIONS':'-c timezone=UTC'}
pg.environment=environment

def sql(text,database='supa_stage2_root_test'):
    return pg.sql(text,database)

def protected():
    require(not (preflight.pg.BASE/'data/postmaster.pid').exists(),'primary_must_remain_stopped')
    before=preflight.protected()
    for p in [preflight.pg.BASE/'data/global/pg_control',SOURCE/'database.dump',SOURCE/'manifest.json',SOURCE/'verification.json',ROOT/'postgres-data/custodian-provisioning/identity.json']:
        before[str(p.relative_to(ROOT))]=hashlib.sha256(p.read_bytes()).hexdigest()
    return before

def checksums(database):
    rows=json.loads(sql("SELECT json_agg(json_build_object('version',version,'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) FROM _sqlx_migrations",database))
    require(len(rows)==10,'expected_ten_migrations')
    for r in rows:
        file,=(ROOT/'migrations').glob(f"{r['version']:04d}_*.sql")
        require(r['success'] and hashlib.sha384(file.read_bytes()).hexdigest()==r['checksum'],'checksum_mismatch')

def run():
    os.umask(0o077)
    require(not (OUT/'validation.json').exists(),'preserve_existing_evidence')
    before=protected();manifest=json.loads((SOURCE/'manifest.json').read_text())
    require(hashlib.sha256((SOURCE/'database.dump').read_bytes()).hexdigest()==manifest['database_sha256'],'stage_one_backup_mismatch')
    pg.private_directory(BASE);pg.private_directory(BASE/'s')
    OUT.mkdir(parents=True,exist_ok=True)
    started=False;workspace_started=False
    try:
        pg.command('initdb','-D',BASE/'data','--auth-local=peer','--auth-host=reject','--encoding=UTF8','--locale=C.UTF-8','--data-checksums')
        with (BASE/'data/postgresql.conf').open('a') as f:f.write(f"\nlisten_addresses=''\nunix_socket_directories='{BASE/'s'}'\nunix_socket_permissions=0700\nport=55444\ntimezone='UTC'\nmax_connections=150\n")
        (BASE/'data/pg_hba.conf').write_text(f'local all {USER} peer\nlocal all all peer\nhost all all 127.0.0.1/32 reject\nhost all all ::1/128 reject\n')
        pg.command('pg_ctl','-D',BASE/'data','-l',BASE/'server.log','-w','start');started=True
        pg.command('createdb','--template=template0','supa_stage2_root_test')
        require(sql('SHOW data_directory')==str(BASE/'data') and sql('SHOW listen_addresses')=='','wrong_validation_cluster')
        env={'PATH':os.environ['PATH'],'HOME':str(Path.home()),'TEST_DATABASE_URL':'postgresql://'+USER+'@localhost/supa_stage2_root_test?'+urlencode({'host':str(BASE/'s'),'port':55444})}
        # Only cfg(test) can supply a disposable Target. The shipped CLI has no bypass switch.
        with (BASE/'migration-tests.log').open('wb') as log:
            r=subprocess.run(['cargo','test','-p','supa-migration-admin','--locked','--offline','--','--include-ignored'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=300)
        require(r.returncode==0,'migration_tests_failed_see_private_log')
        dbs=json.loads(sql("SELECT json_agg(datname ORDER BY datname) FROM pg_database WHERE datname LIKE 'supa_stage2_test_%'"))
        valid=[]
        for db in dbs:
            if sql('SELECT max(version) FROM _sqlx_migrations',db)=='10':valid.append(db)
        require(len(valid)>=3,'migration_success_fixtures_missing')
        source=valid[0];checksums(source)
        state=pg.fingerprint(source)
        pg.command('pg_dump','--format=custom','--file',BASE/'migrated.dump',database=source)
        pg.private_file(BASE/'migrated.dump')
        restored='supa_stage2_restored_test'
        pg.command('createdb','--template=template0',restored)
        pg.command('pg_restore','--exit-on-error','--single-transaction','--dbname',restored,BASE/'migrated.dump')
        require(pg.fingerprint(restored)==state,'migrated_restore_mismatch');checksums(restored)
        sql(f'ALTER DATABASE {restored} SET default_transaction_read_only=on','postgres')
        # Crash and restart this disposable cluster only; verify committed receipts and schema persist.
        pg.command('pg_ctl','-D',BASE/'data','-m','immediate','-w','stop');started=False
        pg.command('pg_ctl','-D',BASE/'data','-l',BASE/'server.log','-w','start');started=True
        require(pg.fingerprint(source)==state and pg.fingerprint(restored)==state,'restart_integrity_mismatch')
        require(sql("SELECT state||':'||revision FROM application_epoch",source)=='prepared:0','migration_activated_epoch')
        require(sql('SELECT count(*) FROM epoch_guard.migration_receipts',source)=='1','receipt_missing_after_restart')
        # Production command must reject absent approval/trust before connecting to any primary service.
        r=subprocess.run([str(ROOT/'target/debug/supa-migration-admin'),'--approval-file',str(BASE/'absent-approval'),'--signature-file',str(BASE/'absent-signature'),'--connection-file',str(BASE/'absent-connection')],cwd=ROOT,env={'PATH':os.environ['PATH'],'HOME':str(Path.home())},capture_output=True,timeout=10)
        require(r.returncode==2,'unsigned_production_request_not_rejected')
        # Keep strict migration tests peer-only and free of runtime-suite fixture roles.
        work=BASE/'workspace';pg.private_directory(work);pg.private_directory(work/'s')
        pg.command('initdb','-D',work/'data','--auth-local=peer','--auth-host=reject','--encoding=UTF8','--locale=C.UTF-8','--data-checksums')
        with (work/'data/postgresql.conf').open('a') as f:f.write(f"\nlisten_addresses=''\nunix_socket_directories='{work/'s'}'\nport=55445\ntimezone='UTC'\nmax_connections=150\n")
        (work/'data/pg_hba.conf').write_text(f'local all {USER} peer\nlocal all all scram-sha-256\nhost all all 127.0.0.1/32 reject\nhost all all ::1/128 reject\n')
        pg.command('pg_ctl','-D',work/'data','-l',work/'server.log','-w','start');workspace_started=True
        pg.command('createdb','--host',work/'s','--port','55445','--template=template0','supa_stage2_workspace_test')
        env['TEST_MIGRATION_DATABASE_URL']=env['TEST_DATABASE_URL']
        env['TEST_DATABASE_URL']='postgresql://'+USER+'@localhost/supa_stage2_workspace_test?'+urlencode({'host':str(work/'s'),'port':55445})
        with (BASE/'workspace-tests.log').open('wb') as log:
            r=subprocess.run(['cargo','test','--workspace','--all-targets','--locked','--offline','--','--include-ignored'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=600)
        require(r.returncode==0,'workspace_tests_failed_see_private_log')
        tests=re.findall(r'^test ([^\n]+) \.\.\. ok$',(BASE/'workspace-tests.log').read_text(),re.M)
        require(protected()==before,'primary_or_historical_evidence_changed')
        report={'isolated_cluster':str(BASE.relative_to(ROOT)),'primary_started':False,'primary_connected':False,'primary_control_and_protected_files_unchanged':True,'primary_last_verified_schema':8,'primary_last_verified_state':'prepared','source_backup_sha256':manifest['database_sha256'],'stage_one_backup_verified':True,'migration_checksums_verified':list(range(1,11)),'migrations_9_and_10_atomic':True,'interrupted_transaction_recovered_at_8':True,'partial_failure_rolled_back_at_8':True,'preexisting_partial_9_rejected':True,'duplicate_and_concurrent_execution_fenced':True,'altered_checksum_and_permissions_rejected':True,'signed_scope_target_identity_expiry_and_artifacts_enforced':True,'migrated_backup_restoration_verified':True,'disposable_crash_restart_preserves_schema_and_receipt':True,'unsigned_production_request_rejected':True,'workspace_tests_passed':len(tests),'application_state':'prepared:0','activation_approvals_created':0,'real_approvals_created':0,'real_credentials_installed':False,'real_owner_or_custodian_grants':0,'runtime_services_started':0,'provider_requests':0,'primary_frontend_repointed':False,'historical_database':'unrecovered','archived_budget_uuid':pg.ARCHIVED,'archived_ceiling_microusd':100000,'unresolved_hold_microusd':4048,'independent_backup_destination':None,'same_disk_backup_accepted':True,'uploads':0}
    finally:
        if workspace_started:pg.command('pg_ctl','-D',BASE/'workspace/data','-m','fast','-w','stop')
        if started:pg.command('pg_ctl','-D',BASE/'data','-m','fast','-w','stop')
    report['primary_and_disposable_clusters_stopped']=True
    (OUT/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
    (OUT/'tests.json').write_text(json.dumps({'command':'cargo test --workspace --all-targets --locked --offline -- --include-ignored','passed':len(tests),'failed':0,'ignored':0,'tests':tests},indent=2)+'\n')
    print(json.dumps({'complete':True,'workspace_tests_passed':len(tests),'primary_unchanged':True}))

if __name__=='__main__':run()
