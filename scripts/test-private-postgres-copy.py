#!/usr/bin/env python3
"""Synthetic registry-copy regressions: no network, registry or build access."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


copy = load('private_copy', 'copy-private-postgres.py')
fixtures = load('publication_fixtures', 'test-postgres-publication.py')


class PrivateCopyTests(unittest.TestCase):
    def setUp(self):
        self.fixture = fixtures.PublicationTests()
        self.fixture.setUp()
        self.addCleanup(self.fixture.doCleanups)
        self.root = self.fixture.root
        self.env = self.fixture.env
        self.identity, self.manifest = copy.publication.canonical_layout(
            self.fixture.archive, self.root / 'validated', self.fixture.record)
        self.raw = (self.root / 'validated/blobs/sha256' / self.identity['digest'][7:]).read_bytes()
        self.calls = []
        self.uploads = 0
        self.work = None

    def exercise(self, *, source_status=200, destination_exists=False, post_status=200,
                 post_visibility='private', anonymous_status=403, fail_copy=False,
                 raw=None, tamper_manifest=False, tamper_blob=False, tamper_source=False,
                 precheck_status=404):
        output = io.StringIO()
        token = 'synthetic-never-log-or-store-in-git'

        def http(url, authorization=None):
            self.calls.append(('http', url, authorization is not None))
            if url == copy.SOURCE_API:
                return source_status, {'name': 'supa-agent-postgres', 'visibility': 'public'}
            if url == copy.DESTINATION_API:
                if self.uploads:
                    return post_status, {'name': 'supa-agent-postgres-private', 'visibility': post_visibility, 'id': 42}
                return (200, {'visibility': 'private'}) if destination_exists else (precheck_status, None)
            if '/token?' in url:
                self.assertIsNone(authorization, 'anonymous grant must not use the job token')
                return 200, {'token': 'anonymous-fixture-grant'}
            if '/manifests/' in url:
                self.assertEqual(authorization, 'Bearer anonymous-fixture-grant')
                return anonymous_status, None
            return 404, None

        def tool(args, **kwargs):
            self.calls.append(('tool', args))
            if args[1] == 'login':
                self.work = Path(args[args.index('--authfile') + 1]).parent
                self.assertEqual((self.work / 'registry-auth.json').stat().st_mode & 0o777, 0o600)
                self.assertEqual(kwargs['input'], (token + '\n').encode())
            elif args[1] == 'inspect':
                if self.uploads and tamper_manifest:
                    return self.raw + b' '
                return self.raw if raw is None else raw
            elif args[1] == 'copy':
                if args[-1].startswith('docker://'):
                    self.uploads += 1
                    if fail_copy:
                        raise RuntimeError('registry_tool_failed_exit_1')
                else:
                    destination = Path(args[-1].removeprefix('oci:').rsplit(':', 1)[0])
                    shutil.copytree(self.root / 'validated', destination)
                    if (self.uploads and tamper_blob) or (not self.uploads and tamper_source):
                        layer = destination / 'blobs/sha256' / self.manifest['layers'][0]['digest'][7:]
                        layer.write_bytes(b'changed layer')
            return b''

        with patch.object(copy, 'APPROVED_DIGEST', self.identity['digest']), \
                patch.object(copy, 'APPROVED_CONFIG', self.manifest['config']['digest']), \
                patch.object(copy.publication, 'http_json', side_effect=http), \
                patch.object(copy.publication, 'run', side_effect=tool), contextlib.redirect_stdout(output):
            error = None
            try:
                report = copy.copy_image(token, self.env)
            except (ValueError, RuntimeError) as failure:
                error = str(failure)
                report = None
        self.assertNotIn(token, output.getvalue())
        if self.work:
            self.assertFalse((self.work / 'registry-auth.json').exists())
        return error, report, output.getvalue()

    def test_success_copies_once_and_preserves_all_manifest_config_and_layer_hashes(self):
        error, report, output = self.exercise()
        self.assertIsNone(error)
        self.assertEqual(self.uploads, 1)
        self.assertEqual(report['manifest'], self.identity)
        self.assertEqual(report['config'], self.manifest['config'])
        self.assertEqual(report['layers'], self.manifest['layers'])
        self.assertTrue(report['anonymous_pull_denied'])
        self.assertEqual(report['visibility'], 'private')
        self.assertIn('private_copy_verified', output)
        tools = [row[1] for row in self.calls if row[0] == 'tool']
        self.assertTrue(all(args[0] == 'skopeo' for args in tools))
        self.assertTrue(all('--preserve-digests' in args and args[args.index('--retry-times') + 1] == '0'
                            for args in tools if args[1] == 'copy'))
        saved = json.loads((self.work / 'verification.json').read_text())
        self.assertEqual(saved, report)
        self.assertEqual((self.work / 'verification.json').stat().st_mode & 0o777, 0o600)

    def test_missing_metadata_privilege_stops_before_any_registry_write(self):
        for status in (401, 403, 404):
            with self.subTest(status=status):
                error, _, _ = self.exercise(source_status=status)
                self.assertEqual(error, 'source_package_metadata_read_required')
                self.assertEqual(self.uploads, 0)

    def test_existing_destination_or_ambiguous_absence_is_rejected(self):
        for options in ({'destination_exists': True}, {'precheck_status': 403}, {'precheck_status': 500}):
            with self.subTest(options=options):
                error, _, _ = self.exercise(**options)
                self.assertEqual(error, 'new_destination_absence_required')
                self.assertEqual(self.uploads, 0)

    def test_destination_created_during_source_verification_stops_before_upload(self):
        original = copy.metadata_preflight
        checks = 0

        def raced(token):
            nonlocal checks
            checks += 1
            if checks == 2:
                raise ValueError('new_destination_absence_required')
            return original(token)

        with patch.object(copy, 'metadata_preflight', side_effect=raced):
            error, _, _ = self.exercise()
        self.assertEqual(checks, 2)
        self.assertEqual(error, 'new_destination_absence_required')
        self.assertEqual(self.uploads, 0)

    def test_wrong_approved_config_cannot_be_accepted_even_with_matching_manifest(self):
        with patch.object(copy, 'APPROVED_DIGEST', self.identity['digest']), \
                patch.object(copy, 'APPROVED_CONFIG', 'sha256:' + '0' * 64):
            with self.assertRaisesRegex(ValueError, 'approved_source_configuration_required'):
                copy.manifest_identity(self.raw)

    def test_changed_source_manifest_or_corrupt_source_blob_never_uploads(self):
        for options in ({'raw': self.raw + b' '}, {'tamper_source': True}):
            with self.subTest(options=options):
                error, _, _ = self.exercise(**options)
                self.assertIsNotNone(error)
                self.assertEqual(self.uploads, 0)

    def test_public_or_unreadable_new_package_fails_immediately_after_upload(self):
        for options in ({'post_visibility': 'public'}, {'post_status': 403}, {'post_status': 404}):
            self.setUp_case()
            with self.subTest(options=options):
                error, _, output = self.exercise(**options)
                self.assertEqual(error, 'destination_explicit_private_required')
                self.assertEqual(self.uploads, 1)
                self.assertNotIn('private_copy_verified', output)
                self.assertEqual(self.calls[-1][:2], ('http', copy.DESTINATION_API))

    def setUp_case(self):
        self.calls = []
        self.uploads = 0
        self.work = None

    def test_anonymous_manifest_200_or_404_never_counts_as_private(self):
        for status in (200, 404, 500):
            self.setUp_case()
            with self.subTest(status=status):
                error, _, _ = self.exercise(anonymous_status=status)
                self.assertEqual(error, 'anonymous_access_denial_required')
                self.assertEqual(self.uploads, 1)

    def test_manifest_or_layer_tampering_stops_without_retry(self):
        for options in ({'tamper_manifest': True}, {'tamper_blob': True}, {'fail_copy': True}):
            self.setUp_case()
            with self.subTest(options=options):
                error, _, output = self.exercise(**options)
                self.assertIsNotNone(error)
                self.assertEqual(self.uploads, 1)
                self.assertIn('"automatic_retry_permitted": false', output)
                self.assertNotIn('private_copy_verified', output)

    def test_rerun_fork_changed_revision_and_non_owner_cannot_access_registry(self):
        for key, value in [('GITHUB_RUN_ATTEMPT', '2'), ('GITHUB_ACTOR', 'other'),
                           ('GITHUB_REPOSITORY', 'other/SupaAgent'), ('GITHUB_REF', 'refs/heads/other'),
                           ('EXPECTED_SOURCE_SHA', 'f' * 40)]:
            with self.subTest(key=key), patch.object(copy.publication, 'http_json') as http:
                with self.assertRaises(ValueError):
                    copy.copy_image('fixture-token', {**self.env, key: value})
                http.assert_not_called()

    def test_workflow_is_manual_scoped_and_cannot_build_or_change_visibility(self):
        workflow = (ROOT / '.github/workflows/postgres-private-copy.yml').read_text()
        self.assertIn('workflow_dispatch:', workflow)
        self.assertIn('github.run_attempt == 1', workflow)
        self.assertIn('persist-credentials: false', workflow)
        self.assertEqual(workflow.count('packages: write'), 1)
        self.assertEqual(workflow.count('--authorize-provisional-private-copy'), 1)
        self.assertIn('${{ github.token }}', workflow)
        for forbidden in ['  push:', 'pull_request:', 'docker build', 'ci-staging-images.py',
                          'railway ', 'secrets.', 'continue-on-error', 'upload-artifact']:
            self.assertNotIn(forbidden, workflow)
        self.assertEqual(copy.APPROVED_DIGEST, 'sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8')
        self.assertEqual(copy.DESTINATION, 'ghcr.io/pyco404/supa-agent-postgres-private')


if __name__ == '__main__':
    unittest.main(verbosity=2)
