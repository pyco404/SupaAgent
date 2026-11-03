#!/usr/bin/env python3
"""Network-free synthetic image/registry tests. No Docker, keys or real records."""
import contextlib
import gzip
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import tarfile
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('publication', ROOT / 'scripts/publish-postgres-image.py')
publication = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publication)
spec = importlib.util.spec_from_file_location('candidate_images', ROOT / 'scripts/ci-staging-images.py')
images = importlib.util.module_from_spec(spec)
spec.loader.exec_module(images)


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.env = {'GITHUB_ACTIONS': 'true', 'GITHUB_REPOSITORY': 'pyco404/SupaAgent',
                    'GITHUB_REF': 'refs/heads/main', 'GITHUB_EVENT_NAME': 'workflow_dispatch',
                    'RUNNER_ENVIRONMENT': 'github-hosted', 'GITHUB_ACTOR': 'pyco404',
                    'GITHUB_RUN_ATTEMPT': '1', 'GITHUB_SHA': '1' * 40,
                    'EXPECTED_SOURCE_SHA': '1' * 40, 'GITHUB_RUN_ID': '123',
                    'RUNNER_TEMP': str(self.root)}
        self.layer = b'synthetic filesystem layer, never a database or credential'
        self.diff_id = 'sha256:' + hashlib.sha256(self.layer).hexdigest()
        self.config = json.dumps({'os': 'linux', 'architecture': 'amd64',
                                  'rootfs': {'type': 'layers', 'diff_ids': [self.diff_id]}}).encode()
        self.archive = self.root / 'validated.tar'
        self.write_archive()
        self.record = {'validated': True, 'source_revision': '1' * 40, 'run_id': '123',
                       'image_id': 'sha256:' + hashlib.sha256(self.config).hexdigest(),
                       'archive': str(self.archive), 'archive_sha256': publication.file_hash(self.archive),
                       'diff_ids': [self.diff_id], 'architecture': 'amd64',
                       'compatibility': {'xml': True, 'tls': True, 'locale': True,
                                         'restricted_api': True, 'restore': True},
                       'scan': {'secret_findings': 0, 'findings': [
                           {'Severity': 'UNKNOWN', 'VulnerabilityID': 'CVE-2026-39824',
                            'PkgName': 'golang.org/x/sys', 'InstalledVersion': 'v0.1.0'}]}}

    def write_archive(self, *, layer=None, config=None, manifest=None, duplicate=False, symlink=False):
        manifest = manifest or [{'Config': 'config.json', 'Layers': ['layer.tar']}]
        with tarfile.open(self.archive, 'w') as archive:
            for name, data in [('manifest.json', json.dumps(manifest).encode()),
                               ('config.json', config or self.config),
                               ('layer.tar', self.layer if layer is None else layer)]:
                member = tarfile.TarInfo(name)
                member.size = len(data)
                if symlink and name == 'layer.tar':
                    member.type = tarfile.SYMTYPE
                    member.linkname = '/private/unrelated'
                    member.size = 0
                    archive.addfile(member)
                else:
                    archive.addfile(member, io.BytesIO(data))
                    if duplicate and name == 'config.json':
                        archive.addfile(member, io.BytesIO(data))

    def test_context_rejects_other_actors_branches_sources_and_retries(self):
        publication.require_context(self.env)
        for key, value in [('GITHUB_ACTOR', 'other'), ('GITHUB_REF', 'refs/heads/other'),
                           ('RUNNER_ENVIRONMENT', 'self-hosted'), ('GITHUB_RUN_ATTEMPT', '2'),
                           ('GITHUB_EVENT_NAME', 'push'), ('EXPECTED_SOURCE_SHA', '2' * 40),
                           ('GITHUB_RUN_ID', '0')]:
            with self.subTest(key=key), self.assertRaises(ValueError):
                publication.require_context({**self.env, key: value})

    def test_candidate_requires_same_run_source_completed_checks_and_clean_scan(self):
        self.assertEqual(publication.check_candidate(self.record, self.env), self.archive)
        for change in [{'validated': False}, {'source_revision': '2' * 40}, {'run_id': '124'},
                       {'architecture': 'arm64'}, {'compatibility': {}}, {'diff_ids': []},
                       {'scan': {'secret_findings': 1, 'findings': []}},
                       {'scan': {'secret_findings': False, 'findings': []}},
                       {'scan': {'secret_findings': 0, 'findings': [{'Severity': 'HIGH'}]}},
                       {'scan': {'secret_findings': 0, 'findings': [{'Severity': 'CRITICAL'}]}}]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                publication.check_candidate({**self.record, **change}, self.env)

    def test_actual_candidate_writer_round_trips_private_validated_image_handoff(self):
        path = self.root / 'candidate.json'
        previous_umask = os.umask(0)
        try:
            images.write_candidate_record(path, self.record)
        finally:
            os.umask(previous_umask)
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        retained = json.loads(path.read_text())
        self.assertEqual(retained, self.record)
        self.assertEqual(publication.check_candidate(retained, self.env), self.archive)
        identity, manifest = publication.canonical_layout(self.archive, self.root / 'retained-oci', retained)
        self.assertEqual(manifest['config']['digest'], retained['image_id'])
        publication.verify_layout(self.root / 'retained-oci', identity, manifest)

    def test_actual_candidate_writer_refuses_existing_files_and_symlinks(self):
        existing = self.root / 'existing.json'
        existing.write_text('preserve existing evidence')
        symlink = self.root / 'candidate-link.json'
        symlink.symlink_to(existing)
        dangling = self.root / 'dangling-link.json'
        dangling.symlink_to(self.root / 'absent.json')
        for path in (existing, symlink, dangling):
            with self.subTest(path=path.name), self.assertRaises(FileExistsError):
                images.write_candidate_record(path, self.record)
        self.assertEqual(existing.read_text(), 'preserve existing evidence')
        self.assertTrue(symlink.is_symlink())
        self.assertTrue(dangling.is_symlink())
        self.assertFalse((self.root / 'absent.json').exists())

    def test_actual_candidate_writer_removes_partial_records_and_propagates_failure(self):
        path = self.root / 'candidate.json'
        original_dump = images.json.dump

        def interrupted_dump(candidate, output):
            output.write('{"validated":true,')
            raise OSError('synthetic interrupted write')

        for fail_at in ('write', 'flush', 'fsync', 'close'):
            with self.subTest(stage=fail_at):
                original_open = open

                @contextlib.contextmanager
                def interrupted_stream(*args, **kwargs):
                    with original_open(*args, **kwargs) as output:
                        if fail_at == 'flush':
                            from unittest.mock import Mock
                            proxy = Mock(wraps=output)
                            proxy.flush.side_effect = OSError('synthetic flush failure')
                            yield proxy
                        else:
                            yield output
                    if fail_at == 'close':
                        raise OSError('synthetic close failure')

                with patch.object(images.json, 'dump', side_effect=interrupted_dump if fail_at == 'write' else original_dump), \
                        patch.object(images.os, 'fsync', side_effect=OSError('synthetic sync failure') if fail_at == 'fsync' else None), \
                        patch('builtins.open', side_effect=interrupted_stream):
                    with self.assertRaises(OSError):
                        images.write_candidate_record(path, self.record)
                self.assertFalse(path.exists())
                with patch.object(publication, 'run') as tool, patch.object(publication, 'http_json') as network:
                    with self.assertRaises(FileNotFoundError):
                        publication.publish(path, 'synthetic-token', self.env)
                    tool.assert_not_called()
                    network.assert_not_called()

    def test_unreviewed_lower_findings_stop_without_hiding_the_existing_advisory(self):
        record = json.loads(json.dumps(self.record))
        record['scan']['findings'].append({'Severity': 'MEDIUM', 'VulnerabilityID': 'CVE-new'})
        with self.assertRaisesRegex(ValueError, 'unreviewed_candidate_advisory'):
            publication.check_candidate(record, self.env)
        self.assertEqual(len(record['scan']['findings']), 2)

    def test_changed_archive_and_symlink_fail_before_registry_access(self):
        self.archive.write_bytes(b'changed after validation')
        with self.assertRaises(ValueError):
            publication.check_candidate(self.record, self.env)
        link = self.root / 'archive-link'
        link.symlink_to(self.archive)
        with self.assertRaises(ValueError):
            publication.check_candidate({**self.record, 'archive': str(link)}, self.env)

    def test_validation_export_rejects_other_services_and_existing_records_before_build(self):
        spec = importlib.util.spec_from_file_location('image_validation', ROOT / 'scripts/ci-staging-images.py')
        images = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(images)
        record_path = self.root / 'candidate.json'
        with patch.object(images, 'require_hosted_ci'), patch.dict(images.os.environ, self.env):
            with self.assertRaisesRegex(ValueError, 'restricted_api_validation'):
                images.validate('worker', Path('/bin/true'), record_path)
            record_path.write_text('{}')
            with self.assertRaisesRegex(ValueError, 'new_runner_private'):
                images.validate('api', Path('/bin/true'), record_path)

    def test_publication_archive_is_created_inside_runner_private_storage(self):
        spec = importlib.util.spec_from_file_location('image_validation', ROOT / 'scripts/ci-staging-images.py')
        images = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(images)
        with patch.object(images, 'require_hosted_ci'), patch.dict(images.os.environ, self.env), \
                patch.object(images.shutil, 'disk_usage') as disk, \
                patch.object(images.tempfile, 'mkdtemp', side_effect=RuntimeError('stop_before_build')) as create:
            disk.return_value.free = 20 * 1024 ** 3
            with self.assertRaisesRegex(RuntimeError, 'stop_before_build'):
                images.validate('api', Path('/bin/true'), self.root / 'candidate.json')
            self.assertEqual(create.call_args.kwargs['dir'], str(self.root))

    def test_oci_preserves_config_and_every_validated_filesystem_hash(self):
        identity, manifest = publication.canonical_layout(self.archive, self.root / 'oci', self.record)
        self.assertEqual(manifest['config']['digest'], self.record['image_id'])
        layer = self.root / 'oci/blobs/sha256' / manifest['layers'][0]['digest'][7:]
        self.assertEqual('sha256:' + hashlib.sha256(gzip.decompress(layer.read_bytes())).hexdigest(), self.diff_id)
        publication.verify_layout(self.root / 'oci', identity, manifest)
        self.assertNotEqual(identity['digest'], self.record['image_id'], 'manifest is not config ID')

    def test_modified_layer_is_rejected_without_rebuilding_or_publishing(self):
        self.write_archive(layer=b'tampered image contents')
        with self.assertRaisesRegex(ValueError, 'validated_layer_contents_required'):
            publication.canonical_layout(self.archive, self.root / 'oci', self.record)

    def test_modified_config_and_wrong_platform_fail(self):
        changed = json.loads(self.config)
        changed['architecture'] = 'arm64'
        self.write_archive(config=json.dumps(changed).encode())
        with self.assertRaisesRegex(ValueError, 'validated_config_and_filesystem_required'):
            publication.canonical_layout(self.archive, self.root / 'oci', self.record)

    def test_duplicate_archive_members_paths_and_symlinks_fail_closed(self):
        for index, options in enumerate([{'duplicate': True}, {'symlink': True},
                                        {'manifest': [{'Config': '../private', 'Layers': ['layer.tar']}]},
                                        {'manifest': [{'Config': '/private', 'Layers': ['layer.tar']}]}]):
            self.write_archive(**options)
            with self.subTest(options=options), self.assertRaises(ValueError):
                publication.canonical_layout(self.archive, self.root / f'oci-{index}', self.record)

    def test_downloaded_config_and_layers_are_independently_verified(self):
        identity, manifest = publication.canonical_layout(self.archive, self.root / 'oci', self.record)
        path = self.root / 'oci/blobs/sha256' / manifest['layers'][0]['digest'][7:]
        path.write_bytes(b'x' * path.stat().st_size)
        with self.assertRaisesRegex(ValueError, 'registry_content_digest_mismatch'):
            publication.verify_layout(self.root / 'oci', identity, manifest)

    def test_public_or_inaccessible_package_and_denied_scope_stop_preflight(self):
        for replies in [[(200, {'visibility': 'public'})], [(403, None)],
                        [(404, None), (200, {'visibility': 'public'})],
                        [(404, None), (404, None), (403, None)]]:
            with patch.object(publication, 'http_json', side_effect=replies), self.assertRaises(ValueError):
                publication.registry_preflight('synthetic-token')

    def test_new_private_default_is_not_claimed_as_verified_write_permission(self):
        with patch.object(publication, 'http_json', side_effect=[(404, None), (404, None),
                                                                (200, {'token': 'never-print-this'}),
                                                                (403, None)]):
            result = publication.registry_preflight('synthetic-token')
        self.assertTrue(result['requested_scope_granted'])
        self.assertTrue(result['private_default_if_created'])
        self.assertNotIn('never-print-this', json.dumps(result))

    def test_existing_public_registry_is_rejected_even_when_package_api_returns_404(self):
        replies = [(404, None), (404, None), (200, {'token': 'scoped'}),
                   (200, {'token': 'anonymous'}), (200, {'tags': ['existing']})]
        with patch.object(publication, 'http_json', side_effect=replies):
            with self.assertRaisesRegex(ValueError, 'repository_anonymously_retrievable'):
                publication.registry_preflight('synthetic-token')

    def test_anonymous_auth_challenge_is_followed_to_detect_public_manifest(self):
        with patch.object(publication, 'http_json', side_effect=[(200, {'token': 'anonymous'}), (200, {})]):
            with self.assertRaisesRegex(ValueError, 'anonymously_retrievable'):
                publication.anonymous_pull_denied(publication.REPOSITORY + '@sha256:' + '1' * 64)
        with patch.object(publication, 'http_json', return_value=(403, None)):
            self.assertTrue(publication.anonymous_pull_denied(publication.REPOSITORY + '@sha256:' + '1' * 64))

    def test_failed_candidate_never_contacts_registry_or_invokes_copy(self):
        candidate = self.root / 'rejected.json'
        candidate.write_text(json.dumps({**self.record, 'validated': False}))
        with patch.object(publication, 'run') as tool, patch.object(publication, 'http_json') as network:
            with self.assertRaises(ValueError):
                publication.publish(candidate, 'synthetic-token', self.env)
            tool.assert_not_called()
            network.assert_not_called()

    def test_registry_config_tampering_fails_independent_blob_verification(self):
        identity, manifest = publication.canonical_layout(self.archive, self.root / 'oci', self.record)
        config = self.root / 'oci/blobs/sha256' / manifest['config']['digest'][7:]
        config.write_bytes(b'x' * config.stat().st_size)
        with self.assertRaisesRegex(ValueError, 'registry_content_digest_mismatch'):
            publication.verify_layout(self.root / 'oci', identity, manifest)

    def mocked_publish(self, *, fail_copy=False, tamper_manifest=False, public_package=False):
        candidate = self.root / 'candidate.json'
        candidate.write_text(json.dumps(self.record))
        calls = []
        state = {}

        def tool(args, **kwargs):
            calls.append(args)
            if args[1] == 'login':
                state['work'] = Path(args[args.index('--authfile') + 1]).parent
            elif args[1] == 'inspect':
                index = json.loads((state['work'] / 'canonical/index.json').read_text())
                digest = index['manifests'][0]['digest'][7:]
                raw = (state['work'] / 'canonical/blobs/sha256' / digest).read_bytes()
                return raw + b' ' if tamper_manifest else raw
            elif args[1] == 'copy' and args[-1].startswith('docker://'):
                if fail_copy:
                    raise RuntimeError('registry_tool_failed_exit_1')
            elif args[1] == 'copy':
                shutil.copytree(state['work'] / 'canonical', state['work'] / 'download')
            return b''

        output = io.StringIO()
        with patch.object(publication, 'run', side_effect=tool), \
                patch.object(publication, 'registry_preflight', return_value={}), \
                patch.object(publication, 'http_json', return_value=(200, {'visibility': 'public' if public_package else 'private'})), \
                patch.object(publication, 'anonymous_pull_denied', return_value=True), \
                contextlib.redirect_stdout(output):
            if fail_copy or tamper_manifest or public_package:
                with self.assertRaises((RuntimeError, ValueError)):
                    publication.publish(candidate, 'synthetic-do-not-log', self.env)
            else:
                publication.publish(candidate, 'synthetic-do-not-log', self.env)
        return calls, state, output.getvalue()

    def test_mocked_success_publishes_once_preserves_unknown_and_never_builds(self):
        calls, state, output = self.mocked_publish()
        uploads = [args for args in calls if args[1] == 'copy' and args[-1].startswith('docker://')]
        self.assertEqual(len(uploads), 1)
        self.assertIn('--preserve-digests', uploads[0])
        self.assertIn('private_postgres_published_verified', output)
        self.assertIn('CVE-2026-39824', output)
        self.assertNotIn('synthetic-do-not-log', output)
        self.assertFalse((state['work'] / 'registry-auth.json').exists())
        self.assertTrue(all(args[0] == 'skopeo' for args in calls))

    def test_ambiguous_copy_never_retries_or_claims_success(self):
        calls, state, output = self.mocked_publish(fail_copy=True)
        self.assertEqual(sum(args[1] == 'copy' for args in calls), 1)
        self.assertIn('publication_outcome_requires_review', output)
        self.assertNotIn('private_postgres_published_verified', output)
        self.assertFalse((state['work'] / 'registry-auth.json').exists())

    def test_registry_manifest_mismatch_and_privacy_failure_require_review(self):
        for options in [{'tamper_manifest': True}, {'public_package': True}]:
            calls, state, output = self.mocked_publish(**options)
            self.assertEqual(sum(args[1] == 'copy' and args[-1].startswith('docker://') for args in calls), 1)
            self.assertIn('publication_outcome_requires_review', output)
            self.assertNotIn('private_postgres_published_verified', output)
            self.assertFalse((state['work'] / 'registry-auth.json').exists())

    def test_manual_workflow_has_one_publication_step_and_no_automatic_trigger(self):
        workflow = (ROOT / '.github/workflows/postgres-publication.yml').read_text()
        self.assertIn('workflow_dispatch:', workflow)
        self.assertNotIn('  push:', workflow)
        self.assertNotIn('pull_request', workflow)
        self.assertEqual(workflow.count('packages: write'), 1)
        self.assertEqual(workflow.count('--authorize-private-publication'), 1)
        self.assertIn('persist-credentials: false', workflow)
        self.assertNotIn('railway ', workflow)
        self.assertNotIn('upload-artifact', workflow)
        self.assertIn('github.actor == \'pyco404\'', workflow)
        self.assertLess(workflow.index('--publication-record'), workflow.index('--authorize-private-publication'))


if __name__ == '__main__':
    unittest.main(verbosity=2)
