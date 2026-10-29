#!/usr/bin/env python3
"""Disposable cutover drill. Never applies migration/transition/config to the real candidate."""
import importlib.util
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import uuid
from urllib.parse import urlencode

spec=importlib.util.spec_from_file_location('review',Path(__file__).with_name('cutover-review.py'))
r=importlib.util.module_from_spec(spec);spec.loader.exec_module(r)
e=r.epoch
pg=e.pg


def save(path,value):pg.exclusive_json(path,value)


def env_text(values):
    return ''.join(k+'='+json.dumps(v)+'\n' for k,v in values.items()).encode()


def invoke(config,operation,approval,revision):
    if not config['database'].startswith('supa_cutover_test_'):raise ValueError('disposable_database_required')
    url=e.url(config['database'])+'&'+urlencode({'options':'-cdefault_transaction_read_only=off'})
    env={'PATH':os.environ['PATH'],'HOME':str(Path.home()),'DATABASE_URL':url,
         'SUPA_REVIEW_EPOCH_ID':config['epoch_id'],'SUPA_REVIEW_ACTOR_ID':config['synthetic_actor_id'],
         'SUPA_REVIEW_APPROVAL_ID':approval,'SUPA_REVIEW_REVISION':str(revision),
         'SUPA_REVIEW_PLAN_SHA256':config['plan_sha256']}
    result=subprocess.run([str(r.ROOT/'target/debug/examples/review_cutover'),'--exercise-disposable',operation],cwd=r.ROOT,env=env,capture_output=True,timeout=30)
    if result.returncode:raise ValueError('disposable_transition_rejected')
    return result.stdout.decode().strip()


def prepare():
    real=e.load()
    if real['id']!=r.EXPECTED:raise ValueError('unexpected_candidate')
    source=pg.fingerprint(real['database'])
    b=e.backup();folder=r.ROOT/'postgres-data/epoch-backups'/b['backup_id']
    manifest=json.loads(pg.private_file(folder/'manifest.json').read_text())
    if pg.digest(folder/'database.dump')!=manifest['database_sha256']:raise ValueError('source_backup_changed')
    id=str(uuid.uuid4());root=r.fixture_root(r.ROOT/'postgres-data/cutover-config-test'/id)
    db='supa_cutover_test_'+uuid.uuid4().hex
    pg.command('createdb','--template=template0',db)
    pg.command('pg_restore','--exit-on-error','--single-transaction','--dbname',db,folder/'database.dump')
    config={'root':str(root),'database':db,'epoch_id':real['id'],'synthetic_actor_id':str(uuid.uuid4()),
            'activation_approval':str(uuid.uuid4()),'rollback_approval':str(uuid.uuid4()),
            'plan_sha256':r.digest((r.ROOT/'artifacts/cutover-review/change-set.json').read_bytes()),
            'source_fingerprint':source,'source_backup_id':b['backup_id']}
    invoke(config,'migrate',config['activation_approval'],0)
    pg.sql("UPDATE application_epoch SET label='Disposable cutover review fixture'",db)
    pg.sql("CREATE TABLE cutover_probe(id integer PRIMARY KEY,value text NOT NULL); INSERT INTO cutover_probe VALUES(1,'Non-agent fixture data must survive rollback')",db)
    pg.sql(f'ALTER DATABASE "{db}" SET default_transaction_read_only=on','postgres')
    save(root/'fixture.json',config)
    return {'root':str(root),'database':db,'actual_candidate_modified':False,'synthetic_actor_only':True}


def load(root):
    p=r.fixture_root(root);c=json.loads(pg.private_file(p/'fixture.json').read_text())
    if c['root']!=str(p) or not c['database'].startswith('supa_cutover_test_'):raise ValueError('fixture_identity_required')
    pg.identity();return c


def activate(c):
    root=Path(c['root'])
    invoke(c,'activate_read_only',c['activation_approval'],0)
    duplicate=False
    try:invoke(c,'activate_read_only',c['activation_approval'],0)
    except ValueError:duplicate=True
    if not duplicate:raise ValueError('duplicate_transition_accepted')
    api=(root/'api.env').read_bytes();site=(root/'site.env').read_bytes()
    # The fixture launcher supplied the concrete temporary ports/tokens. Change only reviewed mode.
    from shlex import split
    def parse(data):return {line.split('=',1)[0]:split(line.split('=',1)[1])[0] for line in data.decode().splitlines()}
    values=parse(api);values.update(SUPA_EPOCH_MODE='active_read_only',SUPA_EPOCH_READONLY_PREVIEW='false',SUPA_EPOCH_APPROVAL_ID=c['activation_approval'])
    records=[r.install_fixture(root,'api.env',env_text(values),r.digest(api)),r.install_fixture(root,'site.env',site+b'# active read-only fixture profile\n',r.digest(site))]
    save(root/'rollback.json',records)
    return {'active_read_only':True,'duplicate_rejected':True,'fixture_only':True}


def rollback(c):
    root=Path(c['root']);invoke(c,'rollback',c['rollback_approval'],1)
    records=json.loads(pg.private_file(root/'rollback.json').read_text())
    if not all(r.rollback_fixture(root,record) for record in records):raise ValueError('configuration_rollback_mismatch')
    if pg.sql("SELECT value FROM cutover_probe WHERE id=1",c['database'])!='Non-agent fixture data must survive rollback':raise ValueError('fixture_data_lost')
    if int(pg.sql('SELECT count(*) FROM epoch_transitions',c['database']))!=2:raise ValueError('audit_history_lost')
    for table in ['agents','agent_access','schedule_authorizations','live_budgets','live_sessions','worker_health','simulated_ledger']:
        if int(pg.sql('SELECT count(*) FROM '+table,c['database']))!=0:raise ValueError('unexpected_business_record')
    if pg.fingerprint(e.load()['database'])!=c['source_fingerprint']:raise ValueError('real_candidate_changed')
    return {'prepared_again':True,'configuration_bytes_restored':True,'probe_preserved':True,'audit_entries':2,'actual_candidate_unchanged':True}


def backup_verify(c):
    root=Path(c['root']);before=pg.fingerprint(c['database'])
    pg.command('pg_dump','--format=custom','--file',root/'active.dump',database=c['database'])
    restore='supa_cutover_test_restore_'+uuid.uuid4().hex
    pg.command('createdb','--template=template0',restore)
    pg.command('pg_restore','--exit-on-error','--single-transaction','--dbname',restore,root/'active.dump')
    if pg.fingerprint(restore)!=before:raise ValueError('active_restore_mismatch')
    pg.sql(f'ALTER DATABASE "{restore}" SET default_transaction_read_only=on','postgres')
    # Reconnect verifies durable identity and read-only defaults, without restarting unrelated services.
    if pg.sql('SHOW default_transaction_read_only',restore)!='on':raise ValueError('restore_not_frozen')
    save(root/'active-backup.json',{'sha256':pg.digest(root/'active.dump'),'state_sha256':before,'restore':restore,'read_only':True})
    return {'backup_restore_verified':True,'restored_database':restore,'read_only':True}


if __name__=='__main__':
    try:
        os.umask(0o077)
        if sys.argv[1:]==['prepare']:result=prepare()
        elif len(sys.argv)==3 and sys.argv[1] in ['activate','rollback','backup-verify']:
            c=load(sys.argv[2]);result={'activate':activate,'rollback':rollback,'backup-verify':backup_verify}[sys.argv[1]](c)
        else:raise ValueError('disposable_fixture_command_required')
        print(json.dumps(result))
    except (ValueError,OSError,KeyError,subprocess.SubprocessError):
        print(json.dumps({'error':'disposable_cutover_drill_failed','real_candidate_activated':False}),file=sys.stderr);sys.exit(2)
