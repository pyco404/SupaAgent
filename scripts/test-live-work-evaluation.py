#!/usr/bin/env python3
"""Network-free consent/replay tests; synthetic scope and mock HTTP only."""
import copy
from datetime import datetime,timezone
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
import urllib.error
from unittest.mock import patch
import uuid
spec=importlib.util.spec_from_file_location('work_eval',Path(__file__).with_name('live-work-evaluation.py'));m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class Readiness(unittest.TestCase):
    def plan(self):
        p=json.loads((m.ROOT/'config/live-work-evaluation-plan.json').read_text());p.update(api_origin='http://127.0.0.1:44444',agent_id=str(uuid.uuid4()),job_id=str(uuid.uuid4()),budget_id=str(uuid.uuid4()),project_id='proj_fixture',input_sha256='a'*64);return p
    def test_unassigned_readiness_never_contacts_a_provider(self):
        p=json.loads((m.ROOT/'config/live-work-evaluation-plan.json').read_text())
        with patch.object(m.urllib.request,'build_opener',side_effect=AssertionError('network forbidden')):
            report,_=m.inspect(p);self.assertFalse(report['ready']);self.assertEqual(report['paid_requests'],0);self.assertEqual(len(report['blockers']),6)
    def test_scope_model_budget_and_archived_budget_are_pinned(self):
        for key,value in [('model','other'),('max_requests',2),('max_cost_microusd',100000),('use_memory',True),('api_origin','https://example.com'),('budget_id','ad91e475-f1da-495a-8247-139643d2d8b1')]:
            p=self.plan();p[key]=value;self.assertFalse(m.inspect(p)[0]['ready'])
    def test_wrong_digest_stops_before_authentication_or_network(self):
        p=self.plan()
        with patch.object(m.urllib.request,'build_opener',side_effect=AssertionError('network forbidden')):
            with self.assertRaises(ValueError):m.run(p,'0'*64,p['model'],4000)
    def test_one_post_is_journaled_and_cannot_be_replayed(self):
        p=self.plan();case=m.inspect(p)[1];source=json.dumps({k:case[k] for k in ['objective','material','requirements']});p['input_sha256']=hashlib.sha256(source.encode()).hexdigest();approval=m.inspect(p)[0]['scope_sha256'];calls=[]
        class Opener:
            def open(self,req,timeout):
                calls.append((req.full_url,req.data))
                if req.full_url.endswith('/health'):result={'epoch':None}
                elif req.full_url.endswith('/jobs/'+p['job_id']):result={'job':{'state':'created','agent_id':p['agent_id'],'received_atoms':0,'earned_atoms':0,'input':source}}
                elif req.data:
                    body=json.loads(req.data);result={'id':body['consent']['request_id'],'status':'awaiting_approval','usage':{'input_tokens':100,'output_tokens':50},'accounted_microusd':35}
                else:result={'enabled':True,'project_id':p['project_id'],'policy':{'model':p['model'],'budget_id':p['budget_id'],'budget_limit_microusd':5000,'input_token_bound':24192,'max_output_tokens':1024},'jobs':[{'id':p['job_id'],'revision':0,'input_sha256':p['input_sha256'],'approved_tools':['inspect_input','extract_evidence','check_requirements']}]}
                return io.BytesIO(json.dumps(result).encode())
        with tempfile.TemporaryDirectory(prefix='supa-m10-runner-') as tmp,patch.object(m,'ROOT',Path(tmp)),patch.dict(m.os.environ,{'SUPA_WORK_OPERATOR_TOKEN':'fixture-owner-token-'*3}),patch.object(m.urllib.request,'build_opener',return_value=Opener()),patch('sys.stdout',io.StringIO()):
            m.run(p,approval,p['model'],4000)
            with self.assertRaises(FileExistsError):m.run(p,approval,p['model'],4000)
            records=list(Path(tmp).rglob('*.jsonl'));self.assertEqual(len(records),1);text=records[0].read_text();self.assertNotIn('fixture-owner-token',text)
        self.assertEqual(sum(body is not None for _,body in calls),1)
    def test_disposable_identity_gate_rejects_before_http_or_authentication(self):
        p=self.plan();p['preflight']={'request_id':str(uuid.uuid4())};approval=m.inspect(p)[0]['scope_sha256']
        with patch.object(m.subprocess,'run',return_value=m.subprocess.CompletedProcess([],1,'','')),patch.object(m.urllib.request,'build_opener',side_effect=AssertionError('HTTP forbidden')):
            with self.assertRaisesRegex(ValueError,'frozen_disposable_target_not_execution_ready'):m.run(p,approval,p['model'],4000)
    def test_job_read_failure_never_posts_or_creates_a_request_journal(self):
        p=self.plan();approval=m.inspect(p)[0]['scope_sha256'];calls=[]
        class Opener:
            def open(self,req,timeout):
                self_test.assertIsNone(req.data, 'No generation POST is permitted after a failed job read')
                calls.append(req.full_url)
                if req.full_url.endswith('/health'):result={'epoch':None}
                elif req.full_url.endswith('/jobs/'+p['job_id']):raise urllib.error.HTTPError(req.full_url,503,'fixture job read failed',{},None)
                else:result={'enabled':True,'project_id':p['project_id'],'policy':{'model':p['model'],'budget_id':p['budget_id'],'budget_limit_microusd':5000,'input_token_bound':24192,'max_output_tokens':1024}}
                return io.BytesIO(json.dumps(result).encode())
        self_test=self
        with tempfile.TemporaryDirectory(prefix='supa-job-read-failure-') as tmp,patch.object(m,'ROOT',Path(tmp)),patch.dict(m.os.environ,{'SUPA_WORK_OPERATOR_TOKEN':'fixture-owner-token-'*3}),patch.object(m.urllib.request,'build_opener',return_value=Opener()):
            with self.assertRaises(urllib.error.HTTPError):m.run(p,approval,p['model'],4000)
            self.assertEqual(list(Path(tmp).rglob('*.jsonl')),[])
        self.assertEqual(len(calls),3)
    def test_frozen_nonce_survives_local_journal_loss(self):
        p=self.plan();case=m.inspect(p)[1];source=json.dumps({k:case[k] for k in ['objective','material','requirements']});p['input_sha256']=hashlib.sha256(source.encode()).hexdigest();nonce=str(uuid.uuid4());p['preflight']={'request_id':nonce};approval=m.inspect(p)[0]['scope_sha256'];posted=[]
        class Opener:
            def open(self,req,timeout):
                if req.full_url.endswith('/health'):result={'epoch':None}
                elif req.full_url.endswith('/jobs/'+p['job_id']):result={'job':{'state':'created','agent_id':p['agent_id'],'received_atoms':0,'earned_atoms':0,'input':source}}
                elif req.data:
                    body=json.loads(req.data);posted.append(body['consent']['request_id']);result={'id':nonce,'status':'awaiting_approval'}
                else:result={'enabled':True,'project_id':p['project_id'],'policy':{'model':p['model'],'budget_id':p['budget_id'],'budget_limit_microusd':5000,'input_token_bound':24192,'max_output_tokens':1024},'jobs':[{'id':p['job_id'],'revision':0,'input_sha256':p['input_sha256'],'approved_tools':['inspect_input','extract_evidence','check_requirements']}]}
                return io.BytesIO(json.dumps(result).encode())
        checked=m.subprocess.CompletedProcess([],0,json.dumps({'scope_sha256':approval,'database':{'budget_state':'installed_zero_spent'}}),'')
        for _ in range(2):
            with tempfile.TemporaryDirectory(prefix='supa-m10a-nonce-') as tmp,patch.object(m,'ROOT',Path(tmp)),patch.dict(m.os.environ,{'SUPA_WORK_OPERATOR_TOKEN':'fixture-owner-token-'*3}),patch.object(m.subprocess,'run',return_value=checked),patch.object(m.urllib.request,'build_opener',return_value=Opener()),patch('sys.stdout',io.StringIO()):m.run(p,approval,p['model'],4000)
        self.assertEqual(posted,[nonce,nonce])  # The Rust reservation rejects the second use; a lost file cannot mint a new nonce.
    def test_claude_is_independent_and_workspace_is_required(self):
        plan=json.loads((m.ROOT/'config/claude-work-evaluation-plan.json').read_text())
        plan['project_id']=None  # Exercise missing scope independently of the enrolled public workspace.
        result,_=m.inspect(plan)
        self.assertFalse(result['ready']);self.assertIn('project_id_unassigned',result['blockers'])
        plan['project_id']='wrkspc_fixture'
        report,_=m.inspect(plan);self.assertTrue(report['ready'])
        self.assertNotEqual(report['scope_sha256'],result['scope_sha256'])
        for field,value in [('provider','unverified'),('model','claude-invented'),('max_requests',2),('project_id','proj_openai'),('budget_limit_microusd',50000)]:
            bad=copy.deepcopy(plan);bad[field]=value;self.assertFalse(m.inspect(bad)[0]['ready'])
    def test_claude_wrong_or_unassigned_scope_never_reaches_http(self):
        plan=json.loads((m.ROOT/'config/claude-work-evaluation-plan.json').read_text())
        plan['project_id']=None  # Exercise missing scope independently of the enrolled public workspace.
        with patch.object(m.urllib.request,'build_opener',side_effect=AssertionError('HTTP forbidden')):
            with self.assertRaises(ValueError):m.run(plan,m.inspect(plan)[0]['scope_sha256'],plan['model'],4000)
    def test_claude_database_preflight_rejects_borrowed_budget_and_wrong_target(self):
        spec=importlib.util.spec_from_file_location('claude',m.ROOT/'scripts/claude-work-preflight.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        plan=json.loads((m.ROOT/'config/claude-work-evaluation-plan.json').read_text())
        for field,value in [('budget_id','e6076529-e302-403e-b846-1ff902c1f641'),('input_sha256','0'*64)]:
            bad=copy.deepcopy(plan);bad[field]=value
            with self.assertRaises(ValueError):module.inspect(bad)
        bad=copy.deepcopy(plan);bad['preflight']['database']['data_directory']='postgres-data/infrastructure-v1/data'
        with self.assertRaises(ValueError):module.inspect(bad)
if __name__=='__main__':unittest.main()
