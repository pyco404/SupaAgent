#!/usr/bin/env python3
"""Offline production-path tests: real CLI children and real local HTTP responses."""
import copy
import http.server
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.request
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent
G = runpy.run_path(str(ROOT / 'railway-activation-state.py'))
M = runpy.run_path(str(ROOT / 'railway-observation.py'))
IMAGE = 'ghcr.io/example/frontend@sha256:' + 'a' * 64
EXPECTED = {'id': 'new', 'projectId': 'project', 'environmentId': 'staging', 'serviceId': 'web', 'image': IMAGE}
BASELINE = {'supa-web': {'source': {'image': IMAGE, 'repo': None}, 'numReplicas': 1,
                         'environmentId': 'staging', 'activeDeployments': [{'id': 'old', 'status': 'SUCCESS'}]},
            'supa-api': {'activeDeployments': [{'id': 'api', 'status': 'SUCCESS'}], 'environmentId': 'staging'},
            'supa-postgres': {'activeDeployments': [{'id': 'pg', 'status': 'SUCCESS'}], 'environmentId': 'staging'}}

for _value in BASELINE.values():
    _value.setdefault('source', {'image': IMAGE, 'repo': None})
    _value.setdefault('numReplicas', 1)
    _value.setdefault('hasEverDeployed', True)


def ready():
    services = copy.deepcopy(BASELINE)
    services['supa-web']['activeDeployments'] = [{'id': 'new', 'status': 'SUCCESS'}]
    return {'deployment': {**EXPECTED, 'status': 'SUCCESS', 'deploymentStopped': False,
                           'meta': {'image': IMAGE, 'imageDigest': IMAGE.split('@')[1]}},
            'project': {'id': 'project', 'name': 'Supa Agent', 'workspaceId': 'workspace',
                        'environments': {'edges': [{'node': {'id': 'staging', 'name': 'staging'}}]},
                        'services': {'edges': [{'node': {'id': name, 'name': name,
                           'serviceInstances': {'edges': [{'node': value}]}}} for name, value in services.items()]}}}


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def do_GET(self):
        self.server.requests.append(self.path)
        private = self.path in ('/operator', '/api/agents', '/api/operator-status', '/agents/new', '/wallets') or self.headers.get('X-Test-Host') == 'supa-web-staging.up.railway.app'
        status = (200 if self.server.unsafe else 401) if private else 200
        if self.server.unavailable:
            self.server.unavailable -= 1
            status = 503
        self.send_response(status)
        self.end_headers()
        self.wfile.write(b'<a class="enter" href="/home">Enter</a>')


class TransportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name); self.config = self.root / 'fixture.json'
        self.config.write_text(json.dumps({'responses': [ready()]}))
        self.counter = self.root / 'count'; self.cli = self.root / 'railway'
        self.cli.write_text('#!' + sys.executable + '\n' + '''import json,pathlib,sys,time
root=pathlib.Path(__file__).parent
v=json.loads((root/'fixture.json').read_text());counter=root/'count'
n=int(counter.read_text()) if counter.exists() else 0
counter.write_text(str(n+1));query=sys.argv[2]
with (root/'calls.jsonl').open('a') as f:f.write(json.dumps({'query':query,'variables':json.load(sys.stdin)})+'\\n')
if n<len(v.get('failures',[])) and v['failures'][n]:
 kind=v['failures'][n]
 if kind=='sleep':time.sleep(1)
 if kind=='invalid':print('not json');sys.exit(0)
 print(kind+' synthetic-sensitive-value',file=sys.stderr);sys.exit(2)
frame=v['responses'][min(n,len(v['responses'])-1)]
key='deployment' if 'deployment(id:' in query else 'project'
print(json.dumps({'data':{key:frame[key]}}))
'''); self.cli.chmod(0o700)
        self.audit = []; self.now = 0
        self.observer = M['Observer'](G, str(self.cli), 'project', 'staging', 'workspace', 'new', record=self.audit.append)
        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.server.requests = []; self.server.unsafe = False; self.server.unavailable = 0
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True); self.thread.start()
        self.addCleanup(self.server.server_close); self.addCleanup(self.server.shutdown)
        real = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        port = self.server.server_address[1]
        class LocalOnly:
            def open(inner, request, timeout):
                from urllib.parse import urlsplit
                u = urlsplit(request.full_url)
                assert u.hostname in ('supaagency.xyz', 'supa-web-staging.up.railway.app')
                mapped = urllib.request.Request('http://127.0.0.1:' + str(port) + u.path,
                                               headers={'X-Test-Host': u.hostname})
                return real.open(mapped, timeout=timeout)
        self.opener = LocalOnly()

    def configure(self, **changes):
        v = json.loads(self.config.read_text()); v.update(changes); self.config.write_text(json.dumps(v))

    def monitor(self, **kwargs):
        # Only the outer HTTP transport is redirected to our real local server.
        # The production parser, subprocess calls, callbacks and readiness guard run unchanged.
        with patch.object(M['urllib'].request, 'build_opener', return_value=self.opener):
            return M['monitor'](G, self.observer, EXPECTED, BASELINE,
                                timeout=kwargs.pop('timeout', 3), interval=.01,
                                record=self.audit.append, **kwargs)

    def test_actual_cli_failure_then_recovery_and_two_complete_security_passes(self):
        self.configure(failures=['request failed'])
        result = self.monitor()
        self.assertTrue(result['ready']); self.assertEqual(result['security_passes'], 2)
        self.assertTrue(any(x.get('read_unavailable') for x in self.audit))
        calls = [json.loads(x)['query'] for x in (self.root / 'calls.jsonl').read_text().splitlines()]
        self.assertTrue(all(x.startswith('query') for x in calls))
        self.assertGreaterEqual(len(self.server.requests), 20)
        self.assertNotIn('synthetic-sensitive-value', json.dumps(self.audit))

    def test_invalid_and_missing_read_evidence_cannot_be_success(self):
        self.configure(failures=['invalid', '503'])
        self.assertTrue(self.monitor()['ready'])
        self.assertIn('invalid_response', [x.get('reason') for x in self.audit])

    def test_repeated_errors_expire_instead_of_early_rollback(self):
        self.configure(failures=['429'] * 100)
        start = time.monotonic()
        with self.assertRaisesRegex(G['ActivationFailure'], 'deadline_exceeded'):
            self.monitor(timeout=.35)
        self.assertGreaterEqual(time.monotonic() - start, .3)
        self.assertFalse(self.server.requests)
        self.assertNotIn('controller_observation_error', json.dumps(self.audit))

    def test_child_timeout_consumes_existing_deadline(self):
        self.configure(failures=['sleep'])
        start = time.monotonic()
        with self.assertRaisesRegex(G['ActivationFailure'], 'deadline_exceeded'):
            self.monitor(timeout=.08)
        self.assertLess(time.monotonic() - start, .5)
        self.assertIn('request_timeout', [x.get('reason') for x in self.audit])

    def test_inventory_read_failure_after_deployment_success_still_retries(self):
        self.configure(failures=['', '502'])
        self.assertTrue(self.monitor()['ready'])
        self.assertTrue(any(x.get('stage') == 'inventory' and not x['observation_available'] for x in self.audit if 'observation_available' in x))

    def test_unknown_identity_and_wrong_digest_are_critical(self):
        for key, value, code in [('serviceId', 'other', 'identity_mismatch'),
                                 ('meta', {'image': IMAGE, 'imageDigest': 'wrong'}, 'digest_mismatch'),
                                 ('status', 'CRASHED', 'target_deployment_failed')]:
            with self.subTest(key=key):
                self.counter.write_text('0'); frame = ready(); frame['deployment'][key] = value
                self.configure(responses=[frame], failures=[])
                with self.assertRaisesRegex(G['ActivationFailure'], code):self.monitor()
        self.assertFalse(self.server.requests)

    def test_actual_unsafe_http_response_is_immediate_failure(self):
        self.server.unsafe = True
        with self.assertRaisesRegex(G['ActivationFailure'], 'public_private_security_failure'):self.monitor()
        self.assertEqual(self.server.requests, ['/operator'])

    def test_missing_success_metadata_waits_for_verified_evidence(self):
        frame = ready(); frame['deployment']['meta'] = None
        self.configure(responses=[frame])
        with self.assertRaisesRegex(G['ActivationFailure'], 'deadline_exceeded'):self.monitor(timeout=.2)
        self.assertFalse(self.server.requests)

    def test_read_transport_rejects_mutation_before_any_child_process(self):
        with self.assertRaisesRegex(G['ActivationFailure'], 'read_only'):
            self.observer.read('deployment', 'mutation{deploymentStop(id:"new")}', {}, time.monotonic()+1)
        self.assertFalse(self.counter.exists())

    def test_bounded_read_retries_used_by_non_polling_controller_reads(self):
        self.configure(failures=['503'])
        result = self.observer.read_until('controller_read', M['DEPLOYMENT_QUERY'], {'d': 'new'},
                                         time.monotonic()+1, sleep=lambda _: None)
        self.assertEqual(result['deployment']['id'], 'new')
        self.assertEqual(self.counter.read_text(), '2')

    def test_http_503_retries_but_never_counts_as_a_security_pass(self):
        self.server.unavailable = 1
        self.assertTrue(self.monitor()['ready'])
        self.assertTrue(any(x.get('reason') == 'upstream_unavailable' for x in self.audit))

    def test_confirmed_tls_verification_failure_is_critical(self):
        import ssl
        class InvalidCertificate:
            def open(self, *args, **kwargs):
                raise urllib.error.URLError(ssl.SSLCertVerificationError('synthetic-private-details'))
        with patch.object(M['urllib'].request, 'build_opener', return_value=InvalidCertificate()):
            with self.assertRaisesRegex(G['ActivationFailure'], 'https_certificate_verification_failed'):
                self.observer.probe(1)
        self.assertNotIn('synthetic-private-details', json.dumps(self.audit))

    def test_missing_inventory_fields_never_trigger_early_backend_change(self):
        frame = ready(); del frame['project']['services']['edges'][1]['node']['serviceInstances']['edges'][0]['node']['source']
        self.configure(responses=[frame])
        with self.assertRaisesRegex(G['ActivationFailure'], 'deadline_exceeded'):self.monitor(timeout=.2)
        self.assertFalse(self.server.requests)

    def test_final_https_acceptance_retries_transport_but_keeps_auth_status(self):
        self.server.unavailable = 1
        request = urllib.request.Request('https://supaagency.xyz/operator')
        with self.observer.https_until(self.opener, request, time.monotonic()+1, sleep=lambda _: None) as response:
            self.assertEqual(response.status, 401)
        self.assertEqual(self.server.requests, ['/operator', '/operator'])

    def test_final_acceptance_cannot_retry_writes_or_unapproved_hosts(self):
        for request in (urllib.request.Request('https://supaagency.xyz/api/agents', method='POST'),
                        urllib.request.Request('https://untrusted.invalid/')):
            with self.assertRaisesRegex(G['ActivationFailure'], 'approved_read_only_https'):
                self.observer.https_until(self.opener, request, time.monotonic()+1)
        self.assertFalse(self.server.requests)

    def test_https_transport_errors_are_unavailable_not_success(self):
        class Broken:
            def open(self, *args, **kwargs):raise urllib.error.URLError('synthetic-sensitive-value')
        with patch.object(M['urllib'].request, 'build_opener', return_value=Broken()):
            with self.assertRaises(G['ObservationUnavailable']):self.observer.probe(1)
        self.assertNotIn('synthetic-sensitive-value', json.dumps(self.audit))


if __name__ == '__main__':
    unittest.main(verbosity=2)
