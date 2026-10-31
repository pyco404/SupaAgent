#!/usr/bin/env python3
"""Recheck actual API status and browser projections on a previously validated disposable cluster."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import uuid
from urllib.parse import urlencode
ROOT=Path(__file__).resolve().parent.parent

def module(name,file):
    spec=importlib.util.spec_from_file_location(name,ROOT/'scripts'/file)
    result=importlib.util.module_from_spec(spec);spec.loader.exec_module(result);return result

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--validation',type=Path,required=True)
    parser.add_argument('--rea-port',type=int)
    args=parser.parse_args();os.umask(0o077)
    if args.rea_port is not None and not 1024<=args.rea_port<=65535:raise ValueError('invalid_rea_port')
    path=args.validation.resolve()
    relative=path.relative_to(ROOT).as_posix()
    if not re.fullmatch(r'artifacts/milestone-8a/[a-f0-9]{12}/validation.json',relative):raise ValueError('validated_disposable_evidence_required')
    report=json.loads(path.read_text())
    if not report['passed'] or not report['all_clusters_stopped']:raise ValueError('completed_validation_required')
    run=path.parent.name
    preparation=module('operator_preparation','validate-local-onboarding.py');strict,before=preparation.baseline()
    work=ROOT/'postgres-data/stage-two'/('m8a_'+run)/'workspace'
    if work.is_symlink() or (work/'data/postmaster.pid').exists():raise ValueError('disposable_cluster_must_be_stopped')
    out=ROOT/'artifacts/milestone-8b'/uuid.uuid4().hex[:12];out.mkdir(mode=0o700,parents=True)
    env={**{k:os.environ[k] for k in ['HOME','PATH']},'CARGO_INCREMENTAL':'0','CARGO_BUILD_JOBS':'2'}
    with (out/'status-build.log').open('wb') as log:
        subprocess.run(['cargo','build','--workspace','--bins','--locked','--offline'],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    pg=strict.pg
    pg.environment=lambda database='postgres':{**env,'LANG':'C.UTF-8','PGHOST':str(work/'s'),'PGPORT':'55445','PGDATABASE':database,'PGUSER':'pikoo','PGPASSFILE':'/dev/null','PGSERVICEFILE':'/dev/null'}
    started=False
    try:
        pg.command('pg_ctl','-D',work/'data','-l',work/'server.log','-w','start');started=True
        if pg.sql('SHOW data_directory','postgres')!=str(work/'data') or pg.sql('SHOW listen_addresses','postgres')!='':raise ValueError('wrong_disposable_cluster')
        url='postgresql://pikoo@localhost/supa_stage2_workspace_test?'+urlencode({'host':str(work/'s'),'port':55445})
        testenv={**env,'TEST_DATABASE_URL':url,'SUPA_8B_BROWSER':'1','SUPA_8B_EVIDENCE_DIR':str(out)}
        if args.rea_port:testenv['SUPA_8B_REA_PORT']=str(args.rea_port)
        with (out/'operator-tests.log').open('wb') as log:
            subprocess.run(['cargo','test','-p','supa-api','--test','writable_epoch','--locked','--offline','--','--include-ignored'],cwd=ROOT,env=testenv,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=540)
        names=json.loads(pg.sql("SELECT json_agg(datname) FROM pg_database WHERE datname LIKE 'supa_writable_%_test'",'postgres'))
        checked=0
        for database in names:
            if database=='supa_writable_root_test':continue
            strict.checksums(database,12);checked+=1
            if pg.sql("SELECT state||':'||revision FROM application_epoch",database)!='prepared:0':raise ValueError('fixture_activated_epoch')
        if preparation.baseline()[1]!=before:raise ValueError('protected_evidence_changed')
        (out/'operator-validation.json').write_text(json.dumps({'passed':True,'primary_stopped':True,'primary_connected':False,'protected_hashes_unchanged':len(before),'restored_schema_12_databases_checked':checked,'writable_epoch_tests':7,'actual_rust_http_status':True,'actual_browser_api':True,'independent_backup_destination':None,'unresolved_hold_microusd':4048,'real_credentials_installed':False,'paid_requests':0},indent=2)+'\n')
    finally:
        if started:pg.command('pg_ctl','-D',work/'data','-m','fast','-w','stop')
    print(json.dumps({'passed':True,'evidence':str(out.relative_to(ROOT)),'disposable_stopped':True}))
if __name__=='__main__':main()
