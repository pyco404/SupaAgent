#!/usr/bin/env python3
"""Offline tests of inert private-runner templates; never calls GitHub or Railway."""
import copy
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import yaml

ROOT = Path(__file__).resolve().parents[1]
RUNNER = ROOT / 'deployment/railway-smoke-runner.yml'
MONITOR = ROOT / 'deployment/railway-smoke-failure-monitor.yml'


def load(path):
    return yaml.load(path.read_text(), Loader=yaml.BaseLoader)


def event():
    return {'repository': {'full_name': 'pyco404/supa-agent-infra', 'private': True},
            'workflow_run': {'head_repository': {'full_name': 'pyco404/supa-agent-infra'},
                'head_branch': 'main', 'event': 'workflow_dispatch',
                'name': 'Railway smoke shutdown', 'path': '.github/workflows/railway-smoke-runner.yml',
                'actor': {'login': 'pyco404'}, 'conclusion': 'failure',
                'html_url': 'https://github.com/pyco404/supa-agent-infra/actions/runs/123'}}


class WorkflowTests(unittest.TestCase):
    def run_monitor(self, data):
        code = load(MONITOR)['jobs']['notify']['steps'][0]['run']
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory, 'event.json')
            path.write_text(json.dumps(data))
            with patch.dict(os.environ, {'GITHUB_EVENT_PATH': str(path), 'RUNNER_TEMP': directory}):
                exec(compile(code, str(MONITOR), 'exec'), {})
            body = Path(directory, 'supa-alert.txt').read_text()
            self.assertIn('@pyco404', body)
            self.assertNotIn('scope', body)
            self.assertEqual(Path(directory, 'supa-alert.txt').stat().st_mode & 0o777, 0o600)

    def test_failure_event_prepares_bounded_non_sensitive_alert(self):
        original = os.umask(0o077)
        try:
            for result in ('failure', 'cancelled', 'timed_out', 'startup_failure'):
                data = event()
                data['workflow_run']['conclusion'] = result
                self.run_monitor(data)
        finally:
            os.umask(original)

    def test_notification_rejects_foreign_public_success_and_untrusted_events(self):
        changes = [('repository', 'private', False), ('repository', 'full_name', 'other/repo'),
                   ('workflow_run', 'head_branch', 'untrusted'),
                   ('workflow_run', 'conclusion', 'success'),
                   ('workflow_run', 'event', 'pull_request'),
                   ('workflow_run', 'html_url', 'https://attacker.invalid/'),
                   ('workflow_run', 'path', '.github/workflows/other.yml')]
        for section, field, value in changes:
            with self.subTest(field=field):
                data = copy.deepcopy(event())
                data[section][field] = value
                with self.assertRaises(AssertionError):
                    self.run_monitor(data)

    def test_templates_are_inert_manual_and_least_privilege(self):
        runner, monitor = load(RUNNER), load(MONITOR)
        self.assertEqual(set(runner['on']), {'workflow_dispatch'})
        self.assertEqual(set(monitor['on']), {'workflow_run'})
        self.assertEqual(runner['permissions'], {'contents': 'read'})
        self.assertEqual(monitor['permissions'], {'contents': 'read', 'issues': 'write'})
        self.assertEqual(runner['concurrency']['cancel-in-progress'], 'false')
        self.assertEqual(runner['jobs']['shutdown']['timeout-minutes'], '20')
        self.assertNotIn('secrets.', MONITOR.read_text())
        self.assertEqual(RUNNER.read_text().count('secrets.'), 1)
        for path in (RUNNER, MONITOR):
            self.assertFalse((ROOT / '.github/workflows' / path.name).exists())
            self.assertNotIn('railway up', path.read_text())
            for job in load(path)['jobs'].values():
                self.assertIn("repository.private == true", job['if'])
                for step in job['steps']:
                    if step['shell'] == 'python':
                        compile(step['run'], str(path), 'exec')
                    else:
                        subprocess.run(['bash', '-n'], input=step['run'], text=True, check=True)

    def test_controller_hashes_match_exact_approved_bytes(self):
        import hashlib
        text = RUNNER.read_text()
        for name in ('railway-shutdown.py', 'test-railway-shutdown.py'):
            digest = hashlib.sha256((ROOT / 'scripts' / name).read_bytes()).hexdigest()
            self.assertIn(digest, text)
        self.assertIn("re.fullmatch('[0-9a-f]{40}', revision)", text)
        self.assertIn("time.time() < scope['started_at_unix']", text)
        self.assertIn("scope['version'] == 2", text)


if __name__ == '__main__':
    unittest.main(verbosity=2)
