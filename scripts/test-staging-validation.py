#!/usr/bin/env python3
"""Offline security regression checks; these never build or execute containers."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


images = load('images', 'ci-staging-images.py')
postgres = load('postgres18', 'validate-postgres18.py')


class StagingSecurity(unittest.TestCase):
    def test_local_fork_pr_and_wrong_branch_fail_before_docker_or_network(self):
        approved = {'GITHUB_ACTIONS': 'true', 'GITHUB_REPOSITORY': 'pyco404/SupaAgent',
                    'GITHUB_REF': 'refs/heads/main', 'RUNNER_ENVIRONMENT': 'github-hosted',
                    'GITHUB_EVENT_NAME': 'workflow_dispatch'}
        images.require_hosted_ci(approved)
        for key in approved:
            rejected = {**approved, key: 'untrusted'}
            with self.subTest(key=key), self.assertRaises(ValueError):
                images.require_hosted_ci(rejected)
        with patch.dict(os.environ, {}, clear=True), patch.object(images.subprocess, 'run') as run:
            with self.assertRaisesRegex(ValueError, 'manual_official_main'):
                images.validate('worker', Path('/bin/true'))
            run.assert_not_called()

    def test_advisories_and_any_secret_block_even_without_fix(self):
        clean = {'SchemaVersion': 2, 'Results': [{'Vulnerabilities': None}]}
        images.vulnerability_gate(clean)
        for severity in ['HIGH', 'CRITICAL']:
            with self.subTest(severity=severity), self.assertRaises(ValueError):
                images.vulnerability_gate({'SchemaVersion': 2, 'Results': [{'Vulnerabilities': [
                    {'Severity': severity, 'FixedVersion': ''}]}]})
        with self.assertRaises(ValueError):
            images.vulnerability_gate({'SchemaVersion': 2, 'Results': [{'Secrets': [{'Severity': 'LOW'}]}]})

    def test_missing_or_changed_scanner_schema_never_passes(self):
        for report in [None, {}, {'SchemaVersion': 1, 'Results': []}, {'SchemaVersion': 2},
                       {'SchemaVersion': 2, 'Results': None}]:
            with self.subTest(report=report), self.assertRaises(ValueError):
                images.vulnerability_gate(report)

    def test_explicit_context_excludes_private_local_files_and_git_history(self):
        with tempfile.TemporaryDirectory() as temp:
            context = Path(temp)
            images.copy_context(ROOT, context)
            self.assertFalse((context / '.git').exists())
            self.assertFalse((context / '.git-recovery').exists())
            self.assertFalse((context / 'artifacts').exists())
            self.assertFalse((context / 'postgres-data').exists())
            self.assertFalse((context / '.env.eval').exists())
            self.assertFalse((context / 'web/node_modules').exists())
            self.assertEqual((context / 'web/.env.local').read_bytes(), images.CANARY)
            self.assertTrue((context / '.dockerignore').is_file())

    def test_layer_stream_canary_detection_handles_chunk_boundaries(self):
        with tempfile.TemporaryDirectory() as temp:
            layer = Path(temp) / 'image.tar'
            layer.write_bytes(b'x' * (1024 * 1024 - 5) + images.CANARY + b'x')
            self.assertTrue(images.contains_canary(layer))
            layer.write_bytes(b'ordinary image bytes')
            self.assertFalse(images.contains_canary(layer))

    def test_postgres_major_mismatch_is_rejected(self):
        postgres.version18('postgres (PostgreSQL) 18.3 (Ubuntu)')
        for version in ['postgres (PostgreSQL) 16.11', 'postgres (PostgreSQL) 19.0', '']:
            with self.subTest(version=version), self.assertRaises(ValueError):
                postgres.version18(version)

    def test_workflow_has_no_automatic_triggers_secret_access_or_publish_steps(self):
        text = (ROOT / '.github/workflows/staging-validation.yml').read_text()
        # Ignore descriptive comments when checking executable workflow policy.
        source = '\n'.join(line.split(' # ', 1)[0] for line in text.splitlines() if not line.lstrip().startswith('#'))
        self.assertIn('  workflow_dispatch:', source)
        self.assertNotIn('  push:', source)
        self.assertNotIn('  pull_request', source)
        self.assertNotIn('secrets.', source)
        self.assertNotRegex(source, r'(?m)^\s+[\w-]+:\s*write(?:\s|$)')
        self.assertNotIn('self-hosted', source)
        self.assertNotIn('railway ', source)
        self.assertNotIn('upload-artifact', source)
        self.assertEqual(source.count('persist-credentials: false'), 2)
        import re
        uses = re.findall(r'uses: ([^\s]+)', source)
        self.assertTrue(uses)
        self.assertTrue(all(re.fullmatch(r'actions/checkout@[0-9a-f]{40}', action) for action in uses))


if __name__ == '__main__':
    unittest.main(verbosity=2)
