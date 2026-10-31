#!/usr/bin/env python3
"""Validate restored migration/recovery evidence on the explicitly isolated receipt cluster."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
from urllib.parse import urlencode

ROOT=Path(__file__).resolve().parent.parent
SOURCE=ROOT/'postgres-data/preflight/1d0e9efe-e66e-4e4f-afc4-478eee4f0833/database.dump'
BACKUP_SHA='600d2bb19b52b4f17d4278459ba722ea5f1135df703b8efc1b0b3db0c8360671'

def validate(config_path,output):
    assert not (ROOT/'postgres-data/infrastructure-v1/data/postmaster.pid').exists()
    assert hashlib.sha256(SOURCE.read_bytes()).hexdigest()==BACKUP_SHA
    cfg=json.loads(config_path.read_text())['receipt'];base=Path(cfg['base']).resolve();assert base.parent==ROOT/'postgres-data/devnet-receipt-readiness'
    assert Path(cfg['socket']).resolve()==base/'socket' and cfg['port']==55450
    env={key:os.environ[key]for key in ['PATH','HOME']};env.update(PGHOST=cfg['socket'],PGPORT='55450',PGUSER='pikoo',PGDATABASE='supa_cutover_test_m7aepoch',PGPASSFILE='/dev/null',PGSERVICEFILE='/dev/null',PGOPTIONS='-c timezone=UTC',LANG='C.UTF-8')
    if cfg.get('library'):env['LD_LIBRARY_PATH']=cfg['library']
    def pg(name,*args):return subprocess.check_output([cfg['bin']+'/'+name,*map(str,args)],env=env,stderr=subprocess.PIPE,text=True)
    def sql(q):return pg('psql','-XqAt','-v','ON_ERROR_STOP=1','-c',q).strip()
    assert sql('SHOW data_directory')==str(base/'data') and sql('SHOW listen_addresses')==''
    assert sql('SELECT max(version) FROM _sqlx_migrations')=='8'
    tables=json.loads(sql("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='public' AND tablename<>'_sqlx_migrations'"))
    columns={t:json.loads(sql("SELECT json_agg(column_name ORDER BY ordinal_position) FROM information_schema.columns WHERE table_schema='public' AND table_name='"+t+"'"))for t in tables}
    def originals():
        return {t:sql('SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),\'[]\') FROM (SELECT '+','.join('"'+c+'"'for c in columns[t])+' FROM "'+t+'") t')for t in tables}
    before=originals();assert sql("SELECT state FROM application_epoch")=='prepared'
    url='postgresql://pikoo@localhost/supa_cutover_test_m7aepoch?'+urlencode({'host':cfg['socket'],'port':55450})
    process_env={k:env[k]for k in ['PATH','HOME','LANG']};process_env['DATABASE_URL']=url
    subprocess.run([str(ROOT/'target/debug/examples/review_cutover'),'--exercise-disposable','migrate'],env=process_env,check=True,stdout=subprocess.DEVNULL)
    assert originals()==before,'original_columns_or_records_changed'
    rows=json.loads(sql("SELECT json_agg(json_build_object('version',version,'success',success,'checksum',encode(checksum,'hex'))ORDER BY version) FROM _sqlx_migrations"))
    assert [r['version']for r in rows]==list(range(1,12))
    for r in rows:assert r['success'] and hashlib.sha384(next((ROOT/'migrations').glob('%04d_*.sql'%r['version'])).read_bytes()).hexdigest()==r['checksum']
    assert sql("SELECT state||':'||revision FROM application_epoch")=='prepared:0'
    assert sql("SELECT (SELECT count(*) FROM agents)+(SELECT count(*) FROM agent_access)+(SELECT count(*) FROM live_budgets)+(SELECT count(*) FROM schedule_authorizations)+(SELECT count(*) FROM epoch_guard.approvals)+(SELECT count(*) FROM epoch_guard.authorities)+(SELECT count(*) FROM epoch_guard.executors)")=='0'
    assert sql('SELECT count(*) FROM economic_entries')=='0'
    spec=importlib.util.spec_from_file_location('receipt_pg',ROOT/'scripts/persistent-postgres.py');pgmod=importlib.util.module_from_spec(spec);spec.loader.exec_module(pgmod);pgmod.BIN=Path(cfg['bin']);pgmod.environment=lambda db='postgres':{**env,'PGDATABASE':db}
    fingerprint=pgmod.fingerprint('supa_cutover_test_m7aepoch')
    output.mkdir(parents=True,exist_ok=False,mode=0o700);dump=output/'restored-migrated.dump';pg('pg_dump','--format=custom','--file',dump);dump.chmod(0o600)
    restored='supa_m7a_forward_restore_test';pg('createdb','--template=template0',restored);pg('pg_restore','--exit-on-error','--single-transaction','--dbname',restored,dump)
    assert pgmod.fingerprint(restored)==fingerprint
    report={'passed':True,'stage_one_backup_verified':True,'primary_started':False,'original_tables_preserved':len(tables),'migration_versions':[r['version']for r in rows],'checksums_verified':True,'cloned_epoch':'prepared:0','owner_or_custodian_grants':0,'approvals_created':0,'economic_entries':0,'migrated_backup_restore_fingerprint':fingerprint,'backup_sha256':hashlib.sha256(dump.read_bytes()).hexdigest(),'rollback_policy':'Preserve committed schema/audit data; restore only into a separate database or use separately reviewed forward repair. No down migration or primary overwrite.'}
    (output/'validation.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
if __name__=='__main__':
    os.umask(0o077);p=argparse.ArgumentParser();p.add_argument('--cluster-config',type=Path,required=True);p.add_argument('--output',type=Path,required=True);args=p.parse_args();validate(args.cluster_config,args.output)
