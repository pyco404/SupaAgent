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
    def test_scan_evidence_never_serializes_matched_secret_material(self):
        report = {'SchemaVersion': 2, 'Results': [{'Target': 'public-package-metadata',
            'Vulnerabilities': [{'VulnerabilityID': 'CVE-fixture', 'PkgName': 'example',
                                 'Severity': 'HIGH', 'InstalledVersion': '1', 'FixedVersion': '2'}],
            'Secrets': [{'Match': 'private-fixture-value-do-not-log', 'Code': 'sensitive-source'}]}]}
        evidence = images.scan_evidence(report)
        self.assertEqual(evidence['secret_findings'], 1)
        self.assertEqual(evidence['findings'][0]['FixedVersion'], '2')
        self.assertNotIn('private-fixture', json.dumps(evidence))
        self.assertNotIn('sensitive-source', json.dumps(evidence))

    def test_only_frontend_can_exit_with_conventional_sigterm_status(self):
        for service in images.SERVICES:
            self.assertTrue(images.clean_shutdown(service, 0))
            self.assertEqual(images.clean_shutdown(service, 143), service == 'frontend')
            for failure in (1, 127, 137, 139):
                self.assertFalse(images.clean_shutdown(service, failure))

    def test_loopback_auth_probes_never_use_environment_proxy(self):
        from unittest.mock import Mock
        response = Mock()
        response.__enter__ = Mock(return_value=Mock(status=200))
        response.__exit__ = Mock(return_value=False)
        opener = Mock()
        opener.open.return_value = response
        with patch.object(images.urllib.request, 'ProxyHandler') as proxy, \
             patch.object(images.urllib.request, 'build_opener', return_value=opener):
            self.assertEqual(images.request(3000, '/healthz'), 200)
        proxy.assert_called_once_with({})
        self.assertEqual(opener.open.call_args.args[0].full_url, 'http://127.0.0.1:3000/healthz')

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

    def test_exact_migration_inventory_checks_content_and_rejects_gaps_duplicates(self):
        import hashlib
        current = images.migration_inventory(ROOT / 'migrations')
        self.assertEqual(len(current), 13)
        self.assertEqual(current[13], hashlib.sha384((ROOT / 'migrations/0013_work_evaluation_reviews.sql').read_bytes()).hexdigest())
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            for contents in [{}, {'0002_gap.sql': 'SELECT 1;'},
                             {'0001_a.sql': 'SELECT 1;', '0001_b.sql': 'SELECT 2;'}]:
                for source in directory.iterdir():
                    source.unlink()
                for name, text in contents.items():
                    (directory / name).write_text(text)
                with self.assertRaises(ValueError):
                    images.migration_inventory(directory)
            for source in directory.iterdir():
                source.unlink()
            (directory / '0001_test.sql').write_text('SELECT 1;')
            before = images.migration_inventory(directory)
            (directory / '0001_test.sql').write_text('SELECT 2;')
            self.assertNotEqual(before, images.migration_inventory(directory))

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
            for relative in ['reference-web/not-found.html', 'reference-web/launch.mjs']:
                self.assertTrue((context / relative).is_file())
                self.assertIn('!'+relative, (context / '.dockerignore').read_text())
                self.assertIn(relative, (ROOT / 'deployment/frontend.Dockerfile').read_text())

    def test_layer_stream_canary_detection_handles_chunk_boundaries(self):
        with tempfile.TemporaryDirectory() as temp:
            layer = Path(temp) / 'image.tar'
            layer.write_bytes(b'x' * (1024 * 1024 - 5) + images.CANARY + b'x')
            self.assertTrue(images.contains_canary(layer))
            layer.write_bytes(b'ordinary image bytes')
            self.assertFalse(images.contains_canary(layer))

    def test_failure_diagnostics_are_bounded_and_redact_ci_credentials(self):
        private = 'synthetic-private-value-1234567890'
        from urllib.parse import urlunsplit
        fixture_url = urlunsplit(('postgresql', 'user:secret@database', '/example', '', ''))
        text = '\x1b[31merror\x1b[0m ' + private + ' ' + fixture_url + ' '
        text += 'sk-' + 'x' * 32
        safe = images.safe_ci_diagnostic(text, [private])
        self.assertNotIn(private, safe)
        self.assertNotIn('user:secret', safe)
        self.assertNotIn('sk-', safe)
        self.assertNotIn('\x1b', safe)
        self.assertIn('error', safe)
        self.assertLessEqual(len(images.safe_ci_diagnostic('x' * 50000)), 12000)

    def test_postgres_major_mismatch_is_rejected(self):
        postgres.version18('postgres (PostgreSQL) 18.3 (Ubuntu)')
        for version in ['postgres (PostgreSQL) 16.11', 'postgres (PostgreSQL) 19.0', '']:
            with self.subTest(version=version), self.assertRaises(ValueError):
                postgres.version18(version)

    def test_database_readiness_requires_running_tcp_listener_and_scrubs_exit_logs(self):
        import subprocess
        import io
        from contextlib import redirect_stdout
        from unittest.mock import Mock
        ok = subprocess.CompletedProcess([], 0, 'true\n', '')
        failed = subprocess.CompletedProcess([], 1, '', '')
        ready = subprocess.CompletedProcess([], 0, '', '')
        docker = Mock(side_effect=[ok, failed, ok, ready])
        with patch.object(images.time, 'sleep') as sleep:
            images.wait_for_database(docker, 'owned-ci-database', [])
        sleep.assert_called_once_with(1)
        self.assertIn('-h', docker.call_args.args)
        self.assertIn('127.0.0.1', docker.call_args.args)
        private = 'synthetic-secret-for-redaction'
        docker = Mock(side_effect=[subprocess.CompletedProcess([], 0, 'false\n', ''),
                                   subprocess.CompletedProcess([], 0, 'startup failed ' + private, '')])
        output = io.StringIO()
        with redirect_stdout(output), self.assertRaisesRegex(RuntimeError, 'database_startup_failed'):
            images.wait_for_database(docker, 'owned-ci-database', [private])
        self.assertNotIn(private, output.getvalue())
        self.assertIn('startup failed', json.loads(output.getvalue())['diagnostic'])
        self.assertFalse(any(call.args[0] == 'exec' for call in docker.call_args_list))

    def test_postgres_loader_failure_preserves_only_safe_library_names(self):
        import subprocess
        import io
        from contextlib import redirect_stdout
        failed = subprocess.CompletedProcess([], 127, '', '/private/path/postgres: error while loading shared libraries: libldap.so.2: cannot open shared object file; sensitive extra text')
        output = io.StringIO()
        with patch.object(postgres.subprocess, 'run', return_value=failed), redirect_stdout(output):
            with self.assertRaisesRegex(RuntimeError, 'postgres18_binary_startup_failed'):
                postgres.library_environment(Path('/tmp'), None)
        report = json.loads(output.getvalue())
        self.assertEqual(report['missing_shared_libraries'], ['libldap.so.2'])
        self.assertEqual(report['exit_code'], 127)
        self.assertNotIn('private', output.getvalue())
        self.assertNotIn('sensitive', output.getvalue())

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
