#!/usr/bin/env python3
"""Synthetic-only watchdog regressions. No Railway connection or real token."""
import contextlib
import copy
import importlib.util
import io
import json
import os
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
                    with contextlib.redirect_stdout(io.StringIO()):
                        self.assertEqual(m.main(['run', '--scope', str(path), *extra]), 1)


class ControllerTests(unittest.TestCase):
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


if __name__ == '__main__':
    unittest.main(verbosity=2)
