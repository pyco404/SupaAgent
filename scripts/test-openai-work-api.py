#!/usr/bin/env python3
"""Network-free launcher regression tests; no real keys, database or provider calls."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('api', Path(__file__).with_name('openai-work-api.py'))
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)


class ScopedPreparation(unittest.TestCase):
    def test_paid_execution_requires_both_exact_scope_and_fresh_flag(self):
        for scope, approved in [(api.SCOPE, False), ('0' * 64, True), (None, True)]:
            with patch.object(api, 'frozen', side_effect=AssertionError('protected setup must not run')):
                with self.assertRaisesRegex(ValueError, 'fresh_exact_paid_authorization_required'):
                    api.execute(scope, approved)

    def test_changed_plan_rejected_before_database_startup(self):
        plan = json.loads(api.PLAN.read_text())
        with tempfile.TemporaryDirectory(prefix='supa-scope-unit-') as tmp:
            path = Path(tmp) / 'plan.json'
            for mutate in [lambda p: p.update(max_requests=2), lambda p: p.update(budget_id='ad91e475-f1da-495a-8247-139643d2d8b1'), lambda p: p['preflight'].update(task_input='changed')]:
                changed = copy.deepcopy(plan)
                mutate(changed)
                path.write_text(json.dumps(changed))
                with patch.object(api, 'PLAN', path), patch.object(api, 'pg', side_effect=AssertionError('no database mutation')):
                    with self.assertRaisesRegex(ValueError, 'frozen_scope_changed'):
                        api.frozen()

    def test_private_credentials_never_serialize_into_reports(self):
        with tempfile.TemporaryDirectory(prefix='supa-key-unit-') as tmp:
            root = Path(tmp)
            file = root / '.env.eval'
            key = 'sk-fake-local-test'
            file.write_text('OPENAI_API_KEY=' + key + '\nOPENAI_MODEL=gpt-6-luna\nOPENAI_PROJECT_ID=proj_npsyg93Ofk7ivKheGa8IDw5q\n')
            file.chmod(0o600)
            with patch.object(api, 'ROOT', root):
                self.assertEqual(api.credentials()['OPENAI_API_KEY'], key)
                file.chmod(0o644)
                with self.assertRaisesRegex(ValueError, 'unsafe_private_environment'):
                    api.credentials()

    def test_sandbox_has_no_repository_or_primary_mount(self):
        args = api.sandbox()
        self.assertNotIn('--bind', args)
        self.assertNotIn(str(api.ROOT), args)
        self.assertNotIn(str(api.DATA), args)
        self.assertIn(str(api.SOCKET), args)
        self.assertIn('--unshare-pid', args)
        self.assertIn('--as-pid-1', args)
        self.assertEqual(args[-1], '/app/api')

    def test_existing_attempt_never_replays_even_with_paid_flags(self):
        with tempfile.TemporaryDirectory(prefix='supa-journal-unit-') as tmp:
            root = Path(tmp)
            directory = root / 'artifacts/milestone-10/live-requests'
            directory.mkdir(parents=True)
            (directory / (api.SCOPE + '.jsonl')).write_text('ambiguous prior attempt\n')
            with patch.object(api, 'ROOT', root), patch.object(api, 'frozen'), patch.object(api, 'stop_api', side_effect=AssertionError('no process change')):
                with self.assertRaisesRegex(ValueError, 'existing_attempt_requires_review_no_retry'):
                    api.execute(api.SCOPE, True)

    def test_pid_reuse_fails_identity_check(self):
        runtime = {'child_pid': 12345, 'child_start_ticks': 'original'}
        with patch.object(api, 'child_identity', return_value='replacement'):
            self.assertFalse(api.process_matches(runtime))

    def test_shutdown_targets_verified_api_child_instead_of_supervisor(self):
        runtime = {'pid': 12344, 'child_pid': 12345, 'child_start_ticks': 'original'}
        with patch.object(api.Path, 'exists', return_value=True), patch.object(api, 'private_json', return_value=runtime), patch.object(api, 'process_matches', side_effect=[True, False, False]), patch.object(api.os, 'kill') as kill:
            api.stop_api()
            kill.assert_called_once_with(12345, api.signal.SIGTERM)

    def test_preparation_action_cannot_accept_paid_flags(self):
        with patch('sys.argv', ['openai-work-api.py', 'start', '--authorize-paid']), patch.object(api, 'start', side_effect=AssertionError('no setup')):
            with self.assertRaisesRegex(ValueError, 'paid_flags_forbidden_for_preparation'):
                api.main()


if __name__ == '__main__':
    unittest.main()
