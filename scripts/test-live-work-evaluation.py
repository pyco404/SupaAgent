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
if __name__=='__main__':unittest.main()
