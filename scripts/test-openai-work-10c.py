#!/usr/bin/env python3
"""Non-paid regression tests for the exact new disposable scope and reused launcher."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent

def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

current = load('new_scope_launcher', ROOT / 'scripts/openai-work-10c-api.py')
legacy_tests = load('existing_launcher_regressions', ROOT / 'scripts/test-openai-work-api.py')
legacy_tests.api = current.api


class FreshScope(unittest.TestCase):
    def test_low_capacity_stops_before_database_or_dispatch(self):
        with patch.object(current.os, 'statvfs', return_value=SimpleNamespace(f_bavail=0, f_frsize=4096)), patch.object(current.api, 'pg', side_effect=AssertionError('database forbidden')):
            with self.assertRaisesRegex(ValueError, 'minimum_one_gib_free_required'):
                current.frozen()

    def test_database_default_binds_the_current_pinned_target(self):
        api = current.api
        with patch.object(api.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, 'fixture', '')) as run:
            self.assertEqual(api.sql('SELECT current_database()'), 'fixture')
            self.assertEqual(run.call_args.kwargs['env']['PGDATABASE'], api.DATABASE)
            self.assertNotEqual(api.DATABASE, 'supa_m10a_a13db38ce0a5_test')

    def test_fresh_identifiers_never_reuse_consumed_scope(self):
        api = current.api
        plan = current.frozen()
        old = json.loads((ROOT / 'config/openai-work-evaluation-current.json').read_text())
        for field in ['agent_id', 'job_id', 'budget_id']:
            self.assertNotEqual(plan[field], old[field])
        self.assertNotEqual(plan['preflight']['request_id'], old['preflight']['request_id'])
        self.assertNotEqual(api.SCOPE, '0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0')
        self.assertEqual((plan['max_requests'], plan['max_cost_microusd'], plan['budget_limit_microusd']), (1, 4000, 5000))

    def test_unreviewed_binary_fails_before_database_or_dispatch(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'unreviewed'
            path.write_text('synthetic incorrect binary')
            with patch.object(current.api, 'BINARY', path), patch.object(current.api, 'pg', side_effect=AssertionError('database forbidden')):
                with self.assertRaisesRegex(ValueError, 'reviewed_binary_changed'):
                    current.frozen()

    def test_changed_schema_fails_before_database_or_dispatch(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root / 'crates/providers/src/work.rs'
            path.parent.mkdir(parents=True)
            path.write_text('synthetic schema change')
            with patch.object(current, 'ROOT', root), patch.object(current.api, 'pg', side_effect=AssertionError('database forbidden')):
                with self.assertRaisesRegex(ValueError, 'reviewed_implementation_changed'):
                    current.frozen()


if __name__ == '__main__':
    suite = unittest.TestSuite([unittest.defaultTestLoader.loadTestsFromModule(legacy_tests), unittest.defaultTestLoader.loadTestsFromTestCase(FreshScope)])
    result = unittest.TextTestRunner().run(suite)
    raise SystemExit(not result.wasSuccessful())
