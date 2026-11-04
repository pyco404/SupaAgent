#!/usr/bin/env python3
"""Synthetic-only watchdog regressions. No Railway connection or real token."""
import contextlib
import copy
import importlib.util
import io
import json
import os
import selectors
import signal
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

spec = importlib.util.spec_from_file_location('shutdown', Path(__file__).with_name('railway-shutdown.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
START = 1800000000


def uid(n):
    return f'00000000-0000-4000-8000-{n:012d}'


def fixture():
    return {'version': 1, 'project_id': m.PROJECT, 'environment_id': m.ENVIRONMENT,
            'application_revision': m.REVISION, 'postgres_image': m.IMAGE,
            'services': dict(zip(('frontend', 'api', 'postgres'), map(uid, (1, 2, 3)))),
            'started_at_unix': START, 'shutdown_at_unix': START + 19800,
            'deadline_unix': START + 21600, 'independent_runner_verified': True,
            'autodeploy_disabled': True, 'synthetic_only': True, 'worker_replicas': 0}


def row(n, service=1, status='SUCCESS', stopped=False):
    return {'id': uid(n), 'projectId': m.PROJECT, 'environmentId': m.ENVIRONMENT,
            'serviceId': uid(service), 'status': status, 'deploymentStopped': stopped}


class Clock:
    def __init__(self, wall):
        self.now, self.ticks = wall, 0

    def sleep(self, seconds):
        self.now += seconds
        self.ticks += seconds


class Log:
    def __init__(self):
        self.events = []

    def write(self, event, **fields):
        self.events.append({'event': event, **fields})


class Provider:
    def __init__(self, rows):
        self.rows, self.calls, self.reads = copy.deepcopy(rows), [], 0

    def snapshot(self, scope):
        self.reads += 1
        return copy.deepcopy(self.rows)

    def mutate(self, operation, deployment_id):
        self.calls.append((operation, deployment_id))
        for item in self.rows:
            if item['id'] == deployment_id:
                item.update(status='REMOVED', deploymentStopped=True)


def drive(provider, now=None, scope=None, log=None, clock=None):
    scope = scope or fixture()
    clock = clock or Clock(now or scope['deadline_unix'] - 15)
    return m.watch(scope, 'a' * 64, provider, log or Log(),
                   lambda: clock.now, lambda: clock.ticks, clock.sleep)


class ScopeTests(unittest.TestCase):
    def test_valid_scope_fingerprint_and_changes(self):
        scope, digest = m.load_scope(json.dumps(fixture()).encode())
        self.assertEqual(scope, fixture())
        changed = fixture(); changed['services']['frontend'] = uid(6)
        self.assertNotEqual(digest, m.load_scope(json.dumps(changed).encode())[1])

    def test_unexpected_scope_fields_and_sensitive_values_rejected(self):
        value = fixture(); value['api_key'] = 'private-canary'
        with self.assertRaisesRegex(m.GuardError, '^scope_fields$'):
            m.load_scope(json.dumps(value).encode())

    def test_scope_target_revision_duration_permissions(self):
        changes = [{'project_id': uid(9)}, {'environment_id': uid(9)},
                   {'application_revision': 'f' * 40}, {'postgres_image': 'postgres:18'},
                   {'independent_runner_verified': False}, {'autodeploy_disabled': False},
                   {'synthetic_only': False}, {'worker_replicas': 1}, {'version': True},
                   {'deadline_unix': START + 21601}, {'shutdown_at_unix': START + 21000}]
        for change in changes:
            with self.subTest(change=change), self.assertRaises(m.GuardError):
                m.load_scope(json.dumps({**fixture(), **change}).encode())

    def test_allowlist_not_blanket_project_authority(self):
        for services in ({}, {'frontend': uid(1)},
                         {'frontend': uid(1), 'api': uid(1), 'postgres': uid(3)},
                         {**fixture()['services'], 'unrelated': uid(9)},
                         {**fixture()['services'], 'api': 'unassigned'}):
            with self.subTest(services=services), self.assertRaises(m.GuardError):
                m.load_scope(json.dumps({**fixture(), 'services': services}).encode())

    def test_malformed_and_oversized_scope(self):
        for raw in (b'not-json', b'[]', b'x' * 16385):
            with self.subTest(raw=raw[:10]), self.assertRaises(m.GuardError):
                m.load_scope(raw)
        with self.assertRaisesRegex(m.GuardError, 'duplicate_scope_field'):
            m.load_scope(b'{"version":1,"version":1}')

    def test_validate_never_constructs_network_client(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'scope.json'; path.write_text(json.dumps(fixture()))
            with patch.object(m, 'Railway', side_effect=AssertionError('network forbidden')):
                with contextlib.redirect_stdout(io.StringIO()) as output:
                    self.assertEqual(m.main(['validate', '--scope', str(path)]), 0)
                self.assertEqual(json.loads(output.getvalue())['network_requests'], 0)
                for extra in ([], ['--authorize-shutdown', '--scope-sha256', 'b' * 64]):
                    with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                        self.assertEqual(m.main(['run', '--scope', str(path), *extra]), 1)


class ControllerTests(unittest.TestCase):
    def test_readiness_is_announced_only_after_inventory_and_durable_audit(self):
        scope=fixture(); clock=Clock(scope['deadline_unix']-5); log=Log(); events=[]
        def ready(event):
            self.assertEqual(log.events[-1]['event'],'armed')
            events.append(event)
        m.watch(scope,'a'*64,Provider([]),log,lambda:clock.now,
                lambda:clock.ticks,clock.sleep,on_armed=ready)
        self.assertEqual(len(events),1)
        self.assertEqual(events[0]['deadline_unix'],scope['deadline_unix'])
        failed=Provider([])
        failed.snapshot=lambda scope: (_ for _ in ()).throw(m.GuardError('provider_connection_failure'))
        with self.assertRaises(m.GuardError):
            m.watch(scope,'a'*64,failed,Log(),lambda:clock.now,
                    lambda:clock.ticks,clock.sleep,on_armed=ready)
        self.assertEqual(len(events),1)

    def test_cancels_pending_and_stops_apps_before_database(self):
        provider = Provider([row(11), row(12, 2), row(13, 3), row(14, 2, 'QUEUED')])
        log = Log(); result = drive(provider, log=log)
        self.assertTrue(result['compute_stopped_verified'])
        self.assertFalse(result['storage_cleanup_authorized'])
        self.assertEqual(provider.calls[-1], ('deploymentStop', uid(13)))
        self.assertIn(('deploymentCancel', uid(14)), provider.calls)
        self.assertGreaterEqual(provider.reads, 3)
        self.assertEqual(log.events[1]['event'], 'shutdown_intent')

    def test_predeadline_restart_never_waits_another_six_hours(self):
        provider = Provider([row(11)])
        clock = Clock(fixture()['shutdown_at_unix'] - 10)
        drive(provider, clock=clock)
        self.assertLessEqual(clock.ticks, 1830)
        self.assertEqual(len(provider.calls), 1)

    def test_forward_clock_jump_cannot_extend_window(self):
        clock = Clock(fixture()['shutdown_at_unix'] - 1)
        def jump(seconds):
            clock.now += 2000 if clock.ticks == 0 else seconds
            clock.ticks += seconds
        provider = Provider([row(11)])
        m.watch(fixture(), 'a' * 64, provider, Log(), lambda: clock.now,
                lambda: clock.ticks, jump)
        self.assertEqual(len(provider.calls), 1)

    def test_backward_wall_clock_uses_monotonic_deadline(self):
        clock = Clock(fixture()['deadline_unix'] - 5)
        def backwards(seconds):
            clock.ticks += seconds; clock.now -= seconds
        result = m.watch(fixture(), 'a' * 64, Provider([]), Log(), lambda: clock.now,
                         lambda: clock.ticks, backwards)
        self.assertTrue(result['compute_stopped_verified'])
        self.assertLessEqual(clock.ticks, 20)

    def test_expired_cleanup_and_far_future_launch_rejected(self):
        for now in (fixture()['deadline_unix'] + 1801, START - 301):
            with self.subTest(now=now), self.assertRaises(m.GuardError):
                drive(Provider([]), now=now)

    def test_new_replacement_deployment_is_stopped(self):
        class Replacement(Provider):
            def snapshot(self, scope):
                if self.reads == 1:
                    self.rows.append(row(12, 2, 'BUILDING'))
                return super().snapshot(scope)
        provider = Replacement([row(11)])
        drive(provider)
        self.assertIn(('deploymentCancel', uid(12)), provider.calls)

    def test_mutation_timeout_never_retried_or_reported_as_stopped(self):
        provider = Provider([row(11)]); log = Log()
        def ambiguous(operation, deployment_id):
            provider.calls.append((operation, deployment_id))
            raise m.GuardError('provider_connection_failure')
        provider.mutate = ambiguous
        with self.assertRaisesRegex(m.GuardError, 'provider_connection_failure'):
            drive(provider, log=log)
        self.assertEqual(len(provider.calls), 1)
        self.assertNotIn('compute_stopped_verified', [e['event'] for e in log.events])

    def test_acknowledgment_is_not_proof_of_stopped_instances(self):
        provider = Provider([row(11, status='REMOVED', stopped=False)])
        provider.mutate = lambda op, ident: provider.calls.append((op, ident))
        with self.assertRaisesRegex(m.GuardError, 'cleanup_deadline'):
            drive(provider)
        self.assertEqual(len(provider.calls), 1)

    def test_read_failure_and_unknown_states_cannot_report_success(self):
        provider = Provider([])
        provider.snapshot = lambda scope: (_ for _ in ()).throw(m.GuardError('provider_schema_error'))
        with self.assertRaises(m.GuardError):
            drive(provider)
        for change in ({'status': 'UNKNOWN'}, {'projectId': uid(9)},
                       {'environmentId': uid(9)}, {'serviceId': uid(9)},
                       {'deploymentStopped': 'true'}):
            with self.subTest(change=change), self.assertRaises(m.GuardError):
                drive(Provider([{**row(11), **change}]))

    def test_duplicate_inventory_and_zero_worker_enforced(self):
        with self.assertRaisesRegex(m.GuardError, 'duplicate_deployment'):
            drive(Provider([row(11), row(11)]))
        scope = fixture(); scope['services']['worker'] = uid(4)
        with self.assertRaisesRegex(m.GuardError, 'worker_not_at_zero'):
            drive(Provider([row(11, 4)]), scope=scope)

    def test_audit_failure_prevents_mutation(self):
        provider = Provider([row(11)])
        class Broken(Log):
            def write(self, event, **fields):
                if event == 'shutdown_intent':
                    raise OSError('synthetic disk full')
        with self.assertRaises(OSError):
            drive(provider, log=Broken())
        self.assertEqual(provider.calls, [])


class AdapterTests(unittest.TestCase):
    def provider(self, replies):
        provider = m.Railway('synthetic-project-token-for-tests')
        provider.query = lambda *args: replies.pop(0)
        return provider

    def env(self):
        return {'environment': {'id': m.ENVIRONMENT, 'projectId': m.PROJECT,
                'serviceInstances': {'edges': [{'node': {'serviceId': value}}
                 for value in fixture()['services'].values()], 'pageInfo': {'hasNextPage': False}}}}

    def test_snapshot_checks_token_service_inventory_and_pages(self):
        replies = [{'projectToken': {'projectId': m.PROJECT, 'environmentId': m.ENVIRONMENT}},
                   self.env(), {'deployments': {'edges': [{'node': row(11)}],
                    'pageInfo': {'hasNextPage': True, 'endCursor': 'next'}}},
                   {'deployments': {'edges': [{'node': row(12, 2)}],
                    'pageInfo': {'hasNextPage': False}}}]
        self.assertEqual(len(self.provider(replies).snapshot(fixture())), 2)
        self.assertEqual(replies, [])

    def test_wrong_token_unexpected_service_and_truncated_inventory_rejected(self):
        with self.assertRaisesRegex(m.GuardError, 'token_target_mismatch'):
            self.provider([{'projectToken': {'projectId': uid(9), 'environmentId': m.ENVIRONMENT}}]).snapshot(fixture())
        for key in ('extra', 'truncated'):
            env = self.env()
            if key == 'extra':
                env['environment']['serviceInstances']['edges'].append({'node': {'serviceId': uid(9)}})
            else:
                env['environment']['serviceInstances']['pageInfo']['hasNextPage'] = True
            with self.assertRaisesRegex(m.GuardError, 'service_inventory_mismatch'):
                self.provider([{'projectToken': {'projectId': m.PROJECT,
                               'environmentId': m.ENVIRONMENT}}, env]).snapshot(fixture())

    def test_false_mutation_ack_and_non_shutdown_operations_rejected(self):
        with self.assertRaisesRegex(m.GuardError, 'not_acknowledged'):
            self.provider([{'deploymentStop': False}]).mutate('deploymentStop', uid(11))
        for operation in ('volumeDelete', 'deploymentCreate', 'serviceDelete'):
            with self.assertRaisesRegex(m.GuardError, 'operation_forbidden'):
                self.provider([]).mutate(operation, uid(11))

    def test_network_failure_diagnostics_do_not_expose_token_or_body(self):
        provider = m.Railway('synthetic-private-canary-token')
        with patch.object(provider.client, 'open', side_effect=urllib.error.URLError('sensitive-canary')):
            with self.assertRaisesRegex(m.GuardError, '^provider_connection_failure$'):
                provider.query('query { projectToken { projectId } }')
        with self.assertRaisesRegex(m.GuardError, 'redirect_rejected'):
            m.NoRedirect().redirect_request(None, None, 302, '', {}, 'https://untrusted.invalid')

    def test_private_audit_permissions_symlinks_and_durable_record(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'audit.jsonl'
            audit = m.Audit(path); audit.write('armed', scope_sha256='a' * 64); audit.close()
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertEqual(json.loads(path.read_text())['event'], 'armed')
            first = m.Audit(path)
            try:
                with self.assertRaisesRegex(m.GuardError, 'already_locked'):
                    m.Audit(path)
            finally:
                first.close()
            path.chmod(0o644)
            with self.assertRaisesRegex(m.GuardError, 'unsafe_audit_file'):
                m.Audit(path)
            link = Path(directory) / 'link'; link.symlink_to(path)
            with self.assertRaises(OSError):
                m.Audit(link)


class SmokeAndAlertTests(unittest.TestCase):
    def smoke(self):
        return {**fixture(), 'version': 2, 'profile': 'smoke-10m',
                'shutdown_at_unix': START + 300, 'deadline_unix': START + 600}

    def test_smoke_requires_separately_reviewed_hash_and_exact_deadlines(self):
        _, old_hash = m.load_scope(json.dumps(fixture()).encode())
        _, new_hash = m.load_scope(json.dumps(self.smoke()).encode())
        self.assertNotEqual(old_hash, new_hash)
        for change in ({'profile': 'arbitrary'}, {'deadline_unix': START + 601},
                       {'shutdown_at_unix': START + 301}, {'version': 1}):
            with self.subTest(change=change), self.assertRaises(m.GuardError):
                m.load_scope(json.dumps({**self.smoke(), **change}).encode())

    def test_smoke_shutdown_and_restart_keep_original_ten_minute_deadline(self):
        clock = Clock(START + 299); provider = Provider([row(11), row(12, 2), row(13, 3)])
        result = drive(provider, scope=self.smoke(), clock=clock)
        self.assertTrue(result['compute_stopped_verified'])
        self.assertLessEqual(clock.ticks, 320)
        self.assertEqual(provider.calls[-1], ('deploymentStop', uid(13)))
        with self.assertRaisesRegex(m.GuardError, 'cleanup_authorization_expired'):
            drive(Provider([]), scope=self.smoke(), now=START + 901)

    def invoke(self, error=None, break_audit=False, missing_alert=False):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory); scope_path = directory / 'scope.json'
            raw = json.dumps(self.smoke()).encode(); scope_path.write_bytes(raw)
            _, digest = m.load_scope(raw)
            audit_path, alert_path = directory/'audit.jsonl', directory/'alerts.jsonl'
            args = ['run', '--scope', str(scope_path), '--scope-sha256', digest,
                    '--authorize-shutdown', '--audit-file', str(audit_path)]
            if not missing_alert:
                args += ['--failure-alert-file', str(alert_path)]
            original = m.Audit
            def audit(path):
                if break_audit and str(path) == str(audit_path):
                    raise OSError('synthetic-sensitive-canary')
                return original(path)
            with patch.object(m, 'Railway', return_value=Provider([])) as client, \
                 patch.object(m, 'watch', side_effect=error,
                              return_value={'compute_stopped_verified': True}), \
                 patch.object(m, 'Audit', side_effect=audit), \
                 patch.dict(os.environ, {'SUPA_SHUTDOWN_PROJECT_TOKEN': 'synthetic-sensitive-canary',
                                         'GITHUB_ACTIONS': 'true'}), \
                 contextlib.redirect_stdout(io.StringIO()) as out, \
                 contextlib.redirect_stderr(io.StringIO()) as err:
                result = m.main(args)
            alerts = [json.loads(line) for line in alert_path.read_text().splitlines()] if alert_path.exists() else []
            text = out.getvalue()+err.getvalue()+json.dumps(alerts)
            self.assertNotIn('synthetic-sensitive-canary', text)
            return result, err.getvalue(), alerts, client.call_count

    def test_connection_auth_and_protocol_failures_emit_private_alert_and_nonzero_exit(self):
        for code in ('provider_connection_failure', 'project_token_target_mismatch',
                     'provider_schema_error', 'shutdown_not_acknowledged'):
            with self.subTest(code=code):
                status, stderr, alerts, _ = self.invoke(m.GuardError(code))
                self.assertEqual(status, 1)
                self.assertEqual(alerts[0]['code'], code)
                self.assertTrue(alerts[0]['compute_may_still_be_running'])
                self.assertTrue(alerts[0]['operator_action_required'])
                self.assertIn('::error title=Supa shutdown', stderr)

    def test_audit_failure_still_produces_durable_failure_alert(self):
        status, stderr, alerts, clients = self.invoke(break_audit=True)
        self.assertEqual(status, 1); self.assertEqual(clients, 0)
        self.assertEqual(alerts[0]['event'], 'operator_alert')
        self.assertIn('operator_alert', stderr)

    def test_signal_interruption_is_not_a_silent_exit(self):
        for error in (KeyboardInterrupt(), m.GuardError('watchdog_terminated')):
            status, _, alerts, _ = self.invoke(error)
            self.assertEqual(status, 1)
            self.assertIn(alerts[0]['code'], ('watchdog_interrupted', 'watchdog_terminated'))
        with self.assertRaisesRegex(m.GuardError, 'watchdog_terminated'):
            m.terminated(None, None)

    def test_missing_alert_sink_blocks_before_any_network_client(self):
        status, _, _, clients = self.invoke(missing_alert=True)
        self.assertEqual(status, 1); self.assertEqual(clients, 0)

    def test_alert_storage_failure_falls_back_to_stderr(self):
        class Broken:
            def write(self, *args, **kwargs):
                raise OSError('synthetic-sensitive-canary')
        with contextlib.redirect_stderr(io.StringIO()) as out:
            self.assertFalse(m.failure_alert('provider_connection_failure', 'a'*64, Broken()))
        self.assertIn('"durable_alert_written": false', out.getvalue())
        self.assertNotIn('synthetic-sensitive-canary', out.getvalue())

    def test_verified_completion_does_not_create_a_false_failure_alert(self):
        status, stderr, alerts, _ = self.invoke()
        self.assertEqual(status, 0); self.assertEqual(stderr, ''); self.assertEqual(alerts, [])

    def test_real_local_sigterm_emits_alert_without_network(self):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            source = Path(__file__).with_name('railway-shutdown.py').resolve()
            raw = json.dumps(self.smoke()).encode(); _, digest = m.load_scope(raw)
            scope = directory/'scope.json'; scope.write_bytes(raw)
            audit, alert = directory/'audit.jsonl', directory/'alert.jsonl'
            code = '''import importlib.util,sys,time
spec=importlib.util.spec_from_file_location('shutdown',sys.argv[1])
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.Railway=lambda token: object()
def wait(*args,**kwargs):
 print('synthetic-watch-ready',flush=True)
 time.sleep(30)
m.watch=wait
raise SystemExit(m.main(sys.argv[2:]))
'''
            child = subprocess.Popen([sys.executable, '-c', code, str(source), 'run',
                '--scope', str(scope), '--scope-sha256', digest, '--authorize-shutdown',
                '--audit-file', str(audit), '--failure-alert-file', str(alert)],
                env={**os.environ, 'GITHUB_ACTIONS': 'true',
                     'SUPA_SHUTDOWN_PROJECT_TOKEN': 'synthetic-sensitive-canary'},
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                with selectors.DefaultSelector() as ready:
                    ready.register(child.stdout, selectors.EVENT_READ)
                    self.assertTrue(ready.select(timeout=5), 'local child startup timed out')
                self.assertEqual(child.stdout.readline().strip(), 'synthetic-watch-ready')
                child.send_signal(signal.SIGTERM)
                stdout, stderr = child.communicate(timeout=5)
                self.assertEqual(child.returncode, 1)
                self.assertIn('watchdog_terminated', stderr)
                self.assertIn('::error title=Supa shutdown', stderr)
                self.assertEqual(json.loads(alert.read_text())['code'], 'watchdog_terminated')
                self.assertNotIn('synthetic-sensitive-canary', stdout+stderr+alert.read_text())
            finally:
                if child.poll() is None:
                    child.kill(); child.communicate(timeout=5)


if __name__ == '__main__':
    unittest.main(verbosity=2)
