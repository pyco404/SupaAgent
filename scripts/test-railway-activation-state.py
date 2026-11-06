#!/usr/bin/env python3
"""Offline Railway activation regressions. No network, credentials or live services."""
import copy
import json
from pathlib import Path
import runpy
import unittest

M = runpy.run_path(str(Path(__file__).with_name('railway-activation-state.py')))
Failure = M['ActivationFailure']
Unavailable = M['ObservationUnavailable']
Probe = M['ProbeResult']
IMAGE = 'ghcr.io/example/frontend@sha256:' + 'a' * 64
EXPECTED = {'id': 'new', 'projectId': 'project', 'environmentId': 'staging',
            'serviceId': 'web', 'image': IMAGE}
BASELINE = {
    'supa-web': {'source': {'image': IMAGE, 'repo': None}, 'numReplicas': 1,
                 'activeDeployments': [{'id': 'old', 'status': 'SUCCESS'}]},
    'supa-api': {'activeDeployments': [{'id': 'api', 'status': 'SUCCESS'}]},
    'supa-postgres': {'activeDeployments': [{'id': 'pg', 'status': 'SUCCESS'}]},
}


def observation(status='SUCCESS', stopped=False, rows=None):
    services = copy.deepcopy(BASELINE)
    services['supa-web']['activeDeployments'] = rows if rows is not None else [{'id': 'new', 'status': 'SUCCESS'}]
    return {'deployment': {**EXPECTED, 'status': status, 'deploymentStopped': stopped,
                           'meta': {'image': IMAGE, 'imageDigest': IMAGE.split('@')[1]}},
            'services': services}


class Harness:
    def __init__(self, frames=None, probes=None):
        self.frames = frames or [observation()]
        self.probes = probes or ['pass']
        self.now = 0
        self.reads = self.probe_calls = 0
        self.audit = []
        self.budgets = []
        self.mutations = []

    def fetch(self, remaining):
        self.budgets.append(remaining)
        value = self.frames[min(self.reads, len(self.frames) - 1)]
        self.reads += 1
        if isinstance(value, Exception):
            raise value
        return copy.deepcopy(value)

    def probe(self, remaining):
        self.budgets.append(remaining)
        value = self.probes[min(self.probe_calls, len(self.probes) - 1)]
        self.probe_calls += 1
        if isinstance(value, Exception):
            raise value
        return Probe(value)

    def sleep(self, seconds):
        self.now += seconds

    def run(self, **kwargs):
        return M['await_ready'](self.fetch, self.probe, EXPECTED, BASELINE,
                                timeout=kwargs.pop('timeout', 20), interval=2,
                                clock=lambda: self.now, sleep=self.sleep,
                                record=self.audit.append, **kwargs)


class StateTests(unittest.TestCase):
    def test_native_access_success_without_local_token_or_receipt(self):
        # The real handover path consumes only Railway deployment observations.
        h = Harness()
        self.assertTrue(h.run()['ready'])
        self.assertEqual(h.probe_calls, 2)
        self.assertEqual(h.mutations, [])

    def test_native_access_denied_is_terminal_without_retrying_deployment(self):
        h = Harness([observation('FAILED')])
        self.fails(h, 'target_deployment_failed')
        self.assertEqual(h.reads, 1)
        self.assertEqual(h.probe_calls, 0)

    def test_native_access_unavailable_never_accepted(self):
        h = Harness([Unavailable()])
        self.fails(h, 'activation_readiness_deadline_exceeded')
        self.assertEqual(h.probe_calls, 0)

    def test_native_access_unknown_state_never_accepted(self):
        self.fails(Harness([observation('UNKNOWN')]),
                   'activation_readiness_deadline_exceeded')

    def test_native_access_requires_immutable_reference(self):
        for image in ('ghcr.io/example/frontend:latest',
                      'ghcr.io/example/frontend@sha256:short',
                      'https://user:password@ghcr.io/example/frontend'):
            with self.subTest(image=image):
                expected = {**EXPECTED, 'image': image}
                with self.assertRaisesRegex(Failure, '^immutable_private_image_required$'):
                    M['native_image_access'](observation()['deployment'], expected)

    def test_native_access_missing_or_conflicting_digest_fails_closed(self):
        for digest in (None, 'sha256:' + 'b' * 64):
            with self.subTest(digest=digest):
                frame = observation()
                frame['deployment']['meta']['imageDigest'] = digest
                self.fails(Harness([frame]), 'target_digest_mismatch')

    def fails(self, h, code):
        with self.assertRaisesRegex(Failure, '^' + code + '$'):
            h.run()
        self.assertEqual(h.mutations, [])

    def test_historical_startup_stopped_flag_is_not_terminal(self):
        h = Harness([observation('DEPLOYING', True), observation('DEPLOYING', True), observation()])
        self.assertTrue(h.run()['ready'])
        self.assertEqual(h.probe_calls, 2)

    def test_historical_overlap_does_not_override_security_success(self):
        h = Harness([observation(rows=[{'id': 'old', 'status': 'SUCCESS'}, {'id': 'new', 'status': 'SUCCESS'}])])
        r = h.run()
        self.assertTrue(r['ready'])
        self.assertFalse(r['inventory_settled'])
        self.assertEqual(r['known_old_entries'], 1)
        self.assertEqual(h.reads, 2)

    def test_delayed_inventory_then_successful_handover(self):
        h = Harness([observation(rows=[{'id': 'old', 'status': 'SUCCESS'}]),
                     observation(rows=[{'id': 'new', 'status': 'SUCCESS'}, {'id': 'old', 'status': 'SUCCESS'}]),
                     observation()])
        self.assertTrue(h.run()['inventory_settled'])
        self.assertEqual(h.probe_calls, 2)

    def test_known_old_terminal_entries_are_teardown_metadata(self):
        for state in M['TERMINAL']:
            with self.subTest(state=state):
                h = Harness([observation(rows=[{'id': 'new', 'status': 'SUCCESS'}, {'id': 'old', 'status': state}])])
                self.assertTrue(h.run()['ready'])

    def test_actual_target_failures_rejected(self):
        for state in M['TERMINAL']:
            with self.subTest(state=state):
                h = Harness([observation(state)])
                self.fails(h, 'target_deployment_failed')
                self.assertEqual(h.probe_calls, 0)

    def test_stopped_success_never_accepted(self):
        self.fails(Harness([observation('SUCCESS', True)]), 'activation_readiness_deadline_exceeded')

    def test_delayed_success_stopped_flag_can_converge(self):
        h = Harness([observation('SUCCESS', True), observation()])
        self.assertTrue(h.run()['ready'])

    def test_target_missing_from_inventory_is_not_success(self):
        h = Harness([observation(rows=[{'id': 'old', 'status': 'SUCCESS'}])])
        self.fails(h, 'activation_readiness_deadline_exceeded')
        self.assertEqual(h.probe_calls, 0)

    def test_success_requires_positive_security_probes(self):
        self.fails(Harness(probes=['pending']), 'activation_readiness_deadline_exceeded')

    def test_actual_privacy_failure_is_immediate(self):
        h = Harness(probes=['unsafe'])
        self.fails(h, 'public_private_security_failure')
        self.assertEqual(h.reads, 1)

    def test_unknown_active_deployment_rejected(self):
        self.fails(Harness([observation(rows=[{'id': 'stranger', 'status': 'SUCCESS'}])]), 'unknown_active_deployment')

    def test_duplicate_conflicting_entries_rejected(self):
        self.fails(Harness([observation(rows=[{'id': 'new', 'status': 'SUCCESS'}, {'id': 'new', 'status': 'DEPLOYING'}])]), 'duplicate_active_deployment')

    def test_target_terminal_in_inventory_rejected(self):
        self.fails(Harness([observation(rows=[{'id': 'new', 'status': 'CRASHED'}])]), 'target_active_deployment_failed')

    def test_wrong_identity_rejected(self):
        for key in ['id', 'projectId', 'environmentId', 'serviceId']:
            with self.subTest(field=key):
                o = observation(); o['deployment'][key] = 'unexpected'
                self.fails(Harness([o]), 'deployment_identity_mismatch')

    def test_wrong_image_and_digest_rejected(self):
        for key, code in [('image', 'target_image_mismatch'), ('imageDigest', 'target_digest_mismatch')]:
            with self.subTest(field=key):
                o = observation(); o['deployment']['meta'][key] = 'wrong'
                self.fails(Harness([o]), code)

    def test_backend_or_service_inventory_change_rejected(self):
        for key in ['supa-api', 'supa-postgres']:
            with self.subTest(service=key):
                o = observation(); o['services'][key]['activeDeployments'] = []
                self.fails(Harness([o]), 'protected_backend_changed')
        o = observation(); o['services']['worker'] = {}
        self.fails(Harness([o]), 'service_inventory_changed')

    def test_source_and_replica_changes_rejected(self):
        o = observation(); o['services']['supa-web']['source']['image'] = 'wrong'
        self.fails(Harness([o]), 'frontend_source_changed')
        o = observation(); o['services']['supa-web']['numReplicas'] = 2
        self.fails(Harness([o]), 'frontend_settings_changed')

    def test_transient_read_failure_is_not_a_deployment_failure(self):
        h = Harness([Unavailable('do-not-retain-private-value'), observation()])
        self.assertTrue(h.run()['ready'])
        self.assertNotIn('do-not-retain-private-value', json.dumps(h.audit))

    def test_transient_probe_failure_is_pending(self):
        h = Harness(probes=[Unavailable('secret'), 'pass'])
        self.assertTrue(h.run()['ready'])
        self.assertNotIn('secret', json.dumps(h.audit))

    def test_consecutive_security_passes_required(self):
        h = Harness(probes=['pass', 'pending', 'pass', 'pass'])
        self.assertTrue(h.run()['ready'])
        self.assertEqual(h.probe_calls, 4)

    def test_permanent_read_failure_expires_without_retrying_mutations(self):
        self.fails(Harness([Unavailable()]), 'activation_readiness_deadline_exceeded')

    def test_unknown_exception_does_not_leak_raw_details(self):
        h = Harness([RuntimeError('sensitive-password')])
        self.fails(h, 'controller_observation_error')
        self.assertNotIn('sensitive-password', json.dumps(h.audit))

    def test_callback_cannot_return_success_after_deadline(self):
        h = Harness()
        def slow_probe(remaining):
            h.now += remaining + 1
            return Probe('pass')
        h.probe = slow_probe
        self.fails(h, 'activation_readiness_deadline_exceeded')

    def test_remaining_budgets_and_sleep_are_bounded(self):
        h = Harness(probes=['pending'])
        self.fails(h, 'activation_readiness_deadline_exceeded')
        self.assertTrue(all(0 < value <= 20 for value in h.budgets))
        self.assertEqual(h.now, 20)

    def test_non_advancing_clock_still_has_finite_poll_count(self):
        h = Harness(probes=['pending']); h.sleep = lambda seconds: None
        self.fails(h, 'activation_readiness_deadline_exceeded')
        self.assertLess(h.reads, 20)

    def test_invalid_polling_bounds_rejected(self):
        for kwargs in [{'timeout': 0}, {'timeout': float('inf')}, {'stable_passes': 1}]:
            with self.subTest(bounds=kwargs):
                with self.assertRaises(Failure):
                    Harness().run(**kwargs)


if __name__ == '__main__':
    unittest.main(verbosity=2)
