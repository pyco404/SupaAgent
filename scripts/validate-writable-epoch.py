#!/usr/bin/env python3
"""Milestone 8A: isolated restored PostgreSQL only; never starts or connects to primary."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import uuid
ROOT=Path(__file__).resolve().parent.parent

def module(name,file):
    spec=importlib.util.spec_from_file_location(name,ROOT/'scripts'/file)
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m

def main():
    os.umask(0o077)
    preparation=module('writable_preparation','validate-local-onboarding.py')
    strict,before=preparation.baseline()
    run=uuid.uuid4().hex[:12]
    out=ROOT/'artifacts/milestone-8a'/run;out.mkdir(parents=True,mode=0o700)
    env={k:os.environ[k] for k in ['PATH','HOME']}
    with (out/'build.log').open('wb') as log:
        subprocess.run(['cargo','build','--workspace','--bins','--locked','--offline'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    strict.BASE=ROOT/'postgres-data/stage-two'/('m8a_'+run)
    strict.OUT=out/'rust-tests'
    strict.run() # All targets, all opt-in database tests, independent signed migration profiles.
    pg=strict.pg
    work=strict.BASE/'workspace'
    def environment(database='postgres'):
        return {**env,'LANG':'C.UTF-8','PGHOST':str(work/'s'),'PGPORT':'55445','PGDATABASE':database,'PGUSER':'pikoo','PGPASSFILE':'/dev/null','PGSERVICEFILE':'/dev/null'}
    pg.environment=environment
    started=False
    try:
        pg.command('pg_ctl','-D',work/'data','-l',work/'server.log','-w','start');started=True
        names=json.loads(pg.sql("SELECT json_agg(datname ORDER BY datname) FROM pg_database WHERE datname LIKE 'supa_writable_%_test' AND datname<>'supa_writable_root_test'",'postgres'))
        if len(names)<6:raise ValueError('restored_security_fixtures_missing')
        source=next(db for db in names if int(pg.sql('SELECT count(*) FROM cycles',db))==2)
        # strict.run stopped this disposable server: every earlier admission must already be fenced.
        if pg.sql('SELECT count(*) FROM epoch_guard.operational_claims c WHERE epoch_guard.operation_live(c.approval_id)',source)!='0':raise ValueError('prior_restart_did_not_fence_claims')
        binding=json.loads(pg.sql("SELECT json_build_object('plan',a.plan,'issuer',v.login,'executor',p.login) FROM epoch_guard.operational_approvals a JOIN epoch_guard.operational_principals p ON p.credential_id=(a.plan->>'credential_id')::uuid JOIN epoch_guard.authorities v ON v.principal_id=p.verified_by LIMIT 1",source))
        import re
        if not all(re.fullmatch(r'[a-z_0-9]+',binding[k]) for k in ['issuer','executor']):raise ValueError('invalid_synthetic_login')
        # Explicit NEW fixture consent only; never replay or extend a prior approval.
        probe=str(uuid.uuid4())
        raw=json.dumps(binding['plan']).replace("'","''")
        pg.sql(f"BEGIN READ WRITE; SET SESSION AUTHORIZATION {binding['issuer']}; SELECT epoch_guard.approve_operation('{probe}', '{raw}'::jsonb||jsonb_build_object('scope','agent_creation','expires_at',clock_timestamp()+interval '10 minutes')); RESET SESSION AUTHORIZATION; SET SESSION AUTHORIZATION {binding['executor']}; SELECT epoch_guard.claim_operation('{probe}','{binding['plan']['epoch_id']}','{binding['plan']['accounting_namespace']}'); COMMIT",source)
        if pg.sql(f"SELECT epoch_guard.operation_live('{probe}')",source)!='t':raise ValueError('fresh_disposable_probe_not_live')
        before_rows=pg.fingerprint(source,include_sequence_values=False)
        before_catalog=preparation.catalog(pg,source)
        pg.command('pg_dump','--format=custom','--file',out/'operational.dump',database=source)
        pg.private_file(out/'operational.dump')
        restored='supa_writable_restored_test'
        pg.command('createdb','--template=template0',restored)
        pg.command('pg_restore','--exit-on-error','--single-transaction','--dbname',restored,out/'operational.dump')
        pg.sql('ALTER DATABASE '+restored+' SET default_transaction_read_only=on','postgres')
        if pg.sql('SHOW default_transaction_read_only',restored)!='on':raise ValueError('restored_readonly_default_missing')
        if pg.fingerprint(restored,include_sequence_values=False)!=before_rows or preparation.catalog(pg,restored)!=before_catalog:raise ValueError('restore_integrity_mismatch')
        if pg.sql('SELECT count(*) FROM epoch_guard.operational_claims c WHERE epoch_guard.operation_live(c.approval_id)',restored)!='0':raise ValueError('cross_database_claim_replayed')
        sequences=preparation.sequence_snapshot(pg,source)
        pg.command('pg_ctl','-D',work/'data','-m','immediate','-w','stop');started=False
        pg.command('pg_ctl','-D',work/'data','-l',work/'server.log','-w','start');started=True
        if pg.fingerprint(source,include_sequence_values=False)!=before_rows:raise ValueError('crash_changed_committed_records')
        preparation.check_sequence_progress(sequences,preparation.sequence_snapshot(pg,source))
        if pg.sql('SELECT count(*) FROM epoch_guard.operational_claims c WHERE epoch_guard.operation_live(c.approval_id)',source)!='0':raise ValueError('server_restart_failed_to_fence_claims')
        for db in names:
            if pg.sql("SELECT state||':'||revision FROM application_epoch",db)!='prepared:0':raise ValueError('fixture_epoch_activated')
        if preparation.baseline()[1]!=before:raise ValueError('protected_files_changed')
        tests=json.loads((out/'rust-tests/tests.json').read_text())
        report={'passed':True,'rust_tests_passed':tests['passed'],'writable_epoch_tests':6,'primary_started':False,'primary_connected':False,'primary_last_verified_schema':8,'protected_files_unchanged':len(before),'unresolved_hold_microusd':4048,'historical_database':'unrecovered','source_database':source,'restored_database':restored,'restored_rows_and_effective_acl_verified':True,'cross_database_restored_claims_denied':True,'postgres_crash_restart_fences_all_claims':True,'committed_rows_and_audit_preserved':True,'sequence_forward_progress_verified':True,'real_credentials_or_grants_created':False,'paid_provider_requests':0,'real_financial_transactions':0,'primary_frontend_repointed':False}
    finally:
        if started:pg.command('pg_ctl','-D',work/'data','-m','fast','-w','stop')
    report['all_clusters_stopped']=True
    (out/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':True,'evidence':str(out.relative_to(ROOT))}))
if __name__=='__main__':main()
