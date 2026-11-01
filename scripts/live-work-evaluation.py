#!/usr/bin/env python3
"""Network-free readiness by default. One authenticated evaluation POST only after fresh approval.

Never creates budgets, agents, credentials, payments or database state. Does not read dotenv.
"""
import argparse
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import subprocess
import urllib.request
from urllib.parse import urlparse
import uuid

ROOT=Path(__file__).resolve().parent.parent
FIXTURES=ROOT/'config/work-evaluation-fixtures.json'

def digest(value):
    return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()

def inspect(plan):
    cases=json.loads(FIXTURES.read_text())['cases']
    case=next((c for c in cases if c['id']==plan.get('fixture_id')),None)
    required=['api_origin','agent_id','job_id','input_sha256','project_id','budget_id']
    blockers=[f'{key}_unassigned' for key in required if not plan.get(key)]
    if (plan.get('version'),plan.get('model'),plan.get('max_requests'),plan.get('max_input_tokens'),plan.get('max_output_tokens'),plan.get('max_cost_microusd'),plan.get('budget_limit_microusd'),plan.get('use_memory'))!=(1,'gpt-6-luna',1,24192,1024,4000,5000,False):blockers.append('unreviewed_model_or_limits')
    if not case or case.get('mode')!=0 or case.get('use_memory') or case['id']!='supplied-synthesis':blockers.append('unreviewed_fixture_scope')
    if case is None:case={}
    if plan.get('api_origin'):
        parsed=urlparse(plan['api_origin'])
        if parsed.scheme!='http' or parsed.hostname!='127.0.0.1' or not parsed.port or parsed.path not in ('','/') or parsed.username or parsed.password or parsed.query or parsed.fragment:blockers.append('nonlocal_api_target')
    for key in ['agent_id','job_id','budget_id']:
        if plan.get(key):
            try:
                value=uuid.UUID(plan[key]);assert value.int and value!=uuid.UUID('ad91e475-f1da-495a-8247-139643d2d8b1')
            except (ValueError,AssertionError,TypeError):blockers.append(f'invalid_{key}')
    if plan.get('input_sha256') and not re.fullmatch('[a-f0-9]{64}',plan['input_sha256']):blockers.append('invalid_input_binding')
    if plan.get('project_id') and not re.fullmatch(r'proj_[A-Za-z0-9_-]{1,114}',plan['project_id']):blockers.append('invalid_project')
    if 'preflight' in plan:
        preflight=plan['preflight']
        try:
            nonce=uuid.UUID(preflight['request_id']);assert nonce.version==4 and nonce.int
        except (KeyError,ValueError,TypeError,AssertionError):blockers.append('invalid_fixed_evaluation_nonce')
    return {'mode':'readiness_only','readiness_basis':'scope_structure_only','paid_execution_authorized':False,'network_requests':0,'paid_requests':0,'ready':not blockers,'blockers':blockers,'scope_sha256':digest({'plan':plan,'fixture':case}),'fixture_sha256':digest(case),'proposed_reservation_microusd':3536,'maximum_provider_requests':1},case

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self,*args,**kwargs):return None

def run(plan,approval,model,ceiling):
    report,case=inspect(plan)
    if not report['ready'] or approval!=report['scope_sha256'] or model!=plan['model'] or ceiling!=plan['max_cost_microusd']:raise ValueError('exact_scope_model_budget_approval_required')
    if 'preflight' in plan:
        # Read-only identity/accounting verification before any HTTP request or paid dispatch.
        checked=subprocess.run(['node',str(ROOT/'scripts/live-work-preflight.mjs'),'--plan-stdin','--check-database','--for-execution'],input=json.dumps(plan),text=True,capture_output=True,timeout=12)
        if checked.returncode:raise ValueError('frozen_disposable_target_not_execution_ready')
        target=json.loads(checked.stdout)
        if target.get('scope_sha256')!=approval or target.get('database',{}).get('budget_state')!='installed_zero_spent':raise ValueError('frozen_target_scope_mismatch')
    token=os.environ.get('SUPA_WORK_OPERATOR_TOKEN','')
    if not 32<=len(token)<=256 or not token.isascii() or any(ord(c)<33 or ord(c)>126 for c in token):raise ValueError('scoped_owner_authentication_required')
    origin=plan['api_origin'].rstrip('/');opener=urllib.request.build_opener(urllib.request.ProxyHandler({}),NoRedirect())
    def request(path,body=None):
        req=urllib.request.Request(origin+path,data=None if body is None else json.dumps(body).encode(),headers={'Authorization':'Bearer '+token,'Content-Type':'application/json'})
        with opener.open(req,timeout=65) as response:
            payload=response.read(256001)
            if len(payload)>256000:raise ValueError('response_limit')
            return json.loads(payload)
    health=request('/health')
    if 'epoch' not in health or health['epoch'] is not None:raise ValueError('application_epochs_forbidden_for_this_disposable_evaluation')
    metadata=request('/agents/'+plan['agent_id']+'/live-work')
    policy=metadata.get('policy') or {}
    for field in ['model','budget_id','budget_limit_microusd','max_output_tokens']:
        if policy.get(field)!=plan[field]:raise ValueError('provider_configuration_mismatch')
    if metadata.get('project_id')!=plan['project_id'] or not metadata.get('enabled') or policy.get('input_token_bound')!=plan['max_input_tokens']:raise ValueError('project_or_token_configuration_mismatch')
    saved=request('/jobs/'+plan['job_id'])['job']
    approved={'objective':case['objective'],'material':case['material'],'requirements':case['requirements']}
    if saved.get('state')!='created' or saved.get('agent_id')!=plan['agent_id'] or saved.get('received_atoms')!=0 or saved.get('earned_atoms')!=0 or json.loads(saved['input'])!=approved or hashlib.sha256(saved['input'].encode()).hexdigest()!=plan['input_sha256']:raise ValueError('unpaid_approved_fixture_mismatch')
    job=next((j for j in metadata['jobs'] if j['id']==plan['job_id']),None)
    if job is None or job['input_sha256']!=plan['input_sha256']:raise ValueError('missing_bound_job')
    now=datetime.now(timezone.utc);stamp=lambda t:t.isoformat().replace('+00:00','Z')
    consent={'job_id':job['id'],'expected_revision':job['revision'],'input_sha256':job['input_sha256'],'approved_tools':job['approved_tools'],'use_memory':False,'consent':{'request_id':plan.get('preflight',{}).get('request_id') or str(uuid.uuid4()),'goal':case['objective'],'model':plan['model'],'authorize_paid':True,'consented_at':stamp(now),'expires_at':stamp(now+timedelta(minutes=2)),'max_requests':1,'max_input_tokens':plan['max_input_tokens'],'max_output_tokens':plan['max_output_tokens'],'max_cost_microusd':ceiling}}
    directory=ROOT/'artifacts/milestone-10/live-requests';directory.mkdir(parents=True,exist_ok=True,mode=0o700)
    if directory.is_symlink() or directory.stat().st_uid!=os.geteuid() or directory.stat().st_mode&0o077:raise ValueError('private_request_journal_directory_required')
    # One scope gets one journal. Ambiguous or repeated invocations never dispatch again.
    journal=directory/(approval+'.jsonl')
    descriptor=os.open(journal,os.O_CREAT|os.O_EXCL|os.O_WRONLY,0o600)
    with os.fdopen(descriptor,'w') as stream:
        def record(value):stream.write(json.dumps(value)+'\n');stream.flush();os.fsync(stream.fileno())
        record({'state':'prepared_no_automatic_replay','scope_sha256':approval,'request_id':consent['consent']['request_id']})
        directory_fd=os.open(directory,os.O_RDONLY)
        try:os.fsync(directory_fd)
        finally:os.close(directory_fd)
        try:
            result=request('/agents/'+plan['agent_id']+'/live-work',consent)
            if result.get('id')!=consent['consent']['request_id']:raise ValueError('session_binding_mismatch')
            record({'state':'received','session':result})
        except Exception:
            record({'state':'outcome_unknown_review_saved_session_no_retry'})
            raise ValueError('outcome_unknown_review_saved_session_no_retry') from None
    print(json.dumps({'session_id':result['id'],'status':result['status'],'usage':result.get('usage'),'accounted_microusd':result.get('accounted_microusd'),'report_path':str(journal),'commercial_revenue':0}))

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--plan',type=Path,default=ROOT/'config/live-work-evaluation-plan.json');parser.add_argument('--run',action='store_true');parser.add_argument('--authorize-paid',action='store_true');parser.add_argument('--scope-sha256');parser.add_argument('--model');parser.add_argument('--max-cost-microusd',type=int);args=parser.parse_args()
    plan=json.loads(args.plan.read_text())
    if not args.run and not args.authorize_paid:print(json.dumps(inspect(plan)[0],indent=2));return
    if not(args.run and args.authorize_paid):raise ValueError('fresh_paid_authorization_flags_required')
    run(plan,args.scope_sha256,args.model,args.max_cost_microusd)
if __name__=='__main__':
    try:main()
    except Exception as error:
        safe=str(error) if isinstance(error,ValueError) else 'evaluation_failed_without_automatic_retry'
        print(json.dumps({'error':safe}),file=sys.stderr);sys.exit(1)
