#!/usr/bin/env python3
"""Read-only real-candidate plan plus fixture-only configuration rollback primitives."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import uuid

spec=importlib.util.spec_from_file_location('epoch',Path(__file__).with_name('development-epoch.py'))
epoch=importlib.util.module_from_spec(spec)
spec.loader.exec_module(epoch)
ROOT=epoch.ROOT
EXPECTED='fd2f52e0-33fc-4c0d-893a-7b08b26ab748'


def digest(data):
    return hashlib.sha256(data).hexdigest()


def plan():
    c=epoch.load()
    if c['id']!=EXPECTED:
        raise ValueError('unexpected_real_candidate')
    config=ROOT/'web/.env.local'
    info=config.lstat()
    if config.is_symlink() or info.st_uid!=os.getuid() or info.st_mode&0o077:
        raise ValueError('private_current_configuration_required')
    pg=epoch.pg
    inventory={name:int(pg.sql(f'SELECT count(*) FROM "{name}"',c['database'])) for name in ['agents','agent_access','schedule_authorizations','live_budgets','live_sessions','inference_calls','worker_health','simulated_ledger']}
    if any(inventory.values()):
        raise ValueError('candidate_not_empty_stop_review')
    migrations=int(pg.sql('SELECT count(*) FROM _sqlx_migrations WHERE success',c['database']))
    if migrations!=8:
        raise ValueError('real_candidate_schema_changed_stop_review')
    roster_path=ROOT/'postgres-data/cutover-review/operators.json'
    roster=json.loads(epoch.pg.private_file(roster_path).read_text()) if roster_path.exists() else []
    if not isinstance(roster,list) or any(not isinstance(entry,dict) or not entry.get('display_name') or entry.get('intended_role') not in ['viewer','operator','owner'] or entry.get('permissions_granted') is not False for entry in roster):
        raise ValueError('unapproved_operator_roster')
    public_roster=[{'display_name_ref':f'postgres-data/cutover-review/operators.json#/{i}/display_name','principal_id':entry.get('principal_id'),'intended_role':entry['intended_role'],'identity_verified':entry.get('identity_verified') is True,'permissions_granted':False} for i,entry in enumerate(roster)]
    return {
        'review_only':True,'ready_for_activation':False,'epoch_id':c['id'],'database':c['database'],
        'accounting_namespace':c['epoch']['accounting_namespace'],'current_state':'prepared','proposed_state':'active_read_only',
        'current_migrations':8,'proposed_migration':'0009_epoch_cutover_review.sql',
        'actual_candidate_modified':False,'primary_frontend_repointed':False,
        'named_operators':public_roster,'identity_verification':'principal UUID and identity confirmation pending','permissions_granted':False,
        'approval_actor':None,'approval_id':None,'service_database_role':'<approved_read_only_service_role>',
        'api_environment':{
            'DATABASE_URL':f"postgresql://<approved_read_only_service_role>:your-password@/{c['database']}?host={pg.BASE/'socket'}&port=55440",
            'SUPA_BIND':'127.0.0.1:8080','SUPA_EXPECTED_EPOCH_ID':c['id'],'SUPA_EPOCH_MODE':'active_read_only',
            'SUPA_EPOCH_READONLY_PREVIEW':'false','SUPA_EPOCH_APPROVAL_ID':'<explicit_approved_transition_uuid>',
            'SUPA_OPERATOR_CREDENTIALS':'<approved_named_roster_with_server_only_tokens>',
            'SUPA_API_TOKEN':'<new_server_only_service_token>',
            'SUPA_PROVIDER':'offline','SUPA_ALLOW_OPENAI':'false','SUPA_CONTROLLED_INFERENCE':'false',
            'SUPA_API_SCHEDULER_ENABLED':'false','SUPA_WALLET_ENABLED':'false','SUPA_PRIVATE_STAGING':'false'},
        'frontend_environment':{'PORT':'3200','SUPA_SITE_ENV_FILE':'<private_cutover_directory>/site.env'},
        'frontend_private_file':{'SUPA_BACKEND_URL':'http://127.0.0.1:8080','SUPA_API_TOKEN':'<matching_service_token>',
                                 'SUPA_WEB_OPERATORS':'<named_accounts_mapped_to_distinct_backend_tokens>'},
        'unset_inherited_variables':['OPENAI_API_KEY','OPENAI_PROJECT_ID','SUPA_LIVE_BUDGET_ID','SUPA_OPERATOR_ID','SUPA_WEB_USERNAME','SUPA_WEB_PASSWORD','SUPA_WALLET_MASTER_FILE','SUPA_WALLET_POLL_SECONDS'],
        'worker':'stopped; not part of cutover','database_read_only_default':'on; never disabled',
        'role_sql_review_only':[
            'CREATE ROLE <approved_read_only_service_role> LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION;',
            'GRANT CONNECT ON DATABASE <exact_epoch_database> TO <approved_read_only_service_role>;',
            'GRANT USAGE ON SCHEMA public TO <approved_read_only_service_role>;',
            'GRANT SELECT ON ALL TABLES IN SCHEMA public TO <approved_read_only_service_role>;'],
        'authentication_review_only':'Add one database-and-role-specific local SCRAM rule before the existing peer rule; set password through a private secret workflow. Do not change unrelated rules.',
        'do_not_grant':['INSERT','UPDATE','DELETE','TRUNCATE','CREATE','sequence USAGE','membership in migrator/admin roles'],
        'rollback_order':['stop reviewed API/frontend processes','append fenced rollback transition through separately approved admin path','verify database default remains read-only','restore exact pre-cutover private config bytes only if hashes match','retain data and both transition audit entries','restart only the previously approved configuration; do not claim historical recovery'],
        'backup_destination':None,'upload_authorized':False,
        'historical_budget':{'id':pg.ARCHIVED,'ceiling_microusd':100000,'unresolved_hold_microusd':4048,'quarantine_changed':False},
        'blockers':['verified named operator and role roster','dedicated service role and private credentials approval','reviewed real-target transition implementation (current primitive is fixture-only)','approval to apply migration 9 to the real candidate','fresh exact cutover approval','independent backup decision or explicit acceptance of same-disk limitation'],
        'inventory':inventory}


def fixture_root(path):
    base=ROOT/'postgres-data/cutover-config-test'
    p=Path(path)
    if p.parent!=base or str(uuid.UUID(p.name))!=p.name:
        raise ValueError('disposable_configuration_root_required')
    epoch.pg.private_directory(p)
    return p


def private_write(path,data):
    with open(os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600),'wb') as f:
        f.write(data);f.flush();os.fsync(f.fileno())


def install_fixture(root,name,data,expected):
    p=fixture_root(root)
    if name not in ['api.env','site.env']:
        raise ValueError('fixture_configuration_name_required')
    target=epoch.pg.private_file(p/name)
    original=target.read_bytes()
    if digest(original)!=expected:
        raise ValueError('configuration_changed_stop')
    id=uuid.uuid4().hex
    before=p/('before-'+id+'.env')
    private_write(before,original)
    pending=p/('pending-'+id+'.env')
    private_write(pending,data)
    os.replace(pending,target)
    return {'file':name,'before':before.name,'before_sha256':digest(original),'after_sha256':digest(data)}


def rollback_fixture(root,record):
    p=fixture_root(root)
    if record['file'] not in ['api.env','site.env'] or not record['before'].startswith('before-') or Path(record['before']).name!=record['before']:
        raise ValueError('fixture_rollback_record_required')
    current=epoch.pg.private_file(p/record['file'])
    before=epoch.pg.private_file(p/record['before']).read_bytes()
    if digest(current.read_bytes())!=record['after_sha256'] or digest(before)!=record['before_sha256']:
        raise ValueError('configuration_changed_stop')
    pending=p/('rollback-'+uuid.uuid4().hex+'.env')
    private_write(pending,before);os.replace(pending,current)
    return current.read_bytes()==before


if __name__=='__main__':
    try:
        if sys.argv[1:]!=['plan']:
            raise ValueError('review_plan_only_no_activation_command')
        print(json.dumps(plan(),indent=2))
    except (ValueError,OSError,KeyError):
        print(json.dumps({'error':'cutover_review_failed_closed','activated':False}),file=sys.stderr)
        sys.exit(2)
