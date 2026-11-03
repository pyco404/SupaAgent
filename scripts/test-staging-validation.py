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
    def test_node_bundled_sqlite_is_checked_independently_of_removed_os_packages(self):
        self.assertFalse(images.node_sqlite_coverage({'node':'26.13.0','sqlite':'3.53.4'})['upstream_fix_present'])
        self.assertTrue(images.node_sqlite_coverage({'node':'26.13.0','sqlite':'3.54.0'})['upstream_fix_present'])
        for value in [{}, {'node':'27.0.0','sqlite':'3.54.0'}, {'node':'26.13.0','sqlite':'unknown'}]:
            with self.assertRaises(ValueError):
                images.node_sqlite_coverage(value)

    def test_xml_critical_fix_is_separate_from_all_known_high_advisory_coverage(self):
        candidate = images.postgres_xml_coverage('libxml2-2.13.9-r2 description:\nXML library')
        self.assertTrue(candidate['critical_upstream_fix_present'])
        self.assertFalse(candidate['all_known_upstream_fixes_present'])
        self.assertEqual(len(candidate['advisories_requiring_coverage_verification']), 7)
        fixed = images.postgres_xml_coverage('libxml2-16-2.15.4-r2\n')
        self.assertTrue(fixed['all_known_upstream_fixes_present'])
        self.assertEqual(fixed['advisories_requiring_coverage_verification'], [])
        self.assertFalse(images.postgres_xml_coverage('libxml2-2.9.14-r0')['critical_upstream_fix_present'])
        for value in ['', 'libxml2-unknown', 'libxml2-2.13.9-r2\nlibxml2-2.15.4-r0']:
            with self.assertRaises(ValueError):
                images.postgres_xml_coverage(value)

    def test_exact_filesystem_package_inventory_retains_versions_and_rejects_missing_metadata(self):
        import io
        import tarfile
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'filesystem.tar'
            content = b'Package: libc6\nVersion: 2.41-12+deb13u1\n\nPackage: libgcc-s1\nVersion: 14.2.0-19\n'
            with tarfile.open(path, 'w') as archive:
                member = tarfile.TarInfo('var/lib/dpkg/status.d/libc6')
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
            self.assertEqual(images.runtime_package_inventory(path), {'libc6':'2.41-12+deb13u1','libgcc-s1':'14.2.0-19'})
            with tarfile.open(path, 'w'):
                pass
            with self.assertRaisesRegex(ValueError, 'actual_runtime_package_inventory_required'):
                images.runtime_package_inventory(path)

    def test_exported_filesystem_checks_do_not_need_image_shell_or_extraction(self):
        import tarfile
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / 'filesystem.tar'
            for names, expected in [(['app/web/package.json'], True),
                                    (['./app/web/.env.local'], False),
                                    (['app/.supa-secrets/canary'], False),
                                    (['/app/.env.ci-sentinel'], False)]:
                with tarfile.open(path, 'w') as archive:
                    for name in names:
                        archive.addfile(tarfile.TarInfo(name))
                self.assertEqual(images.excluded_runtime_paths_absent(path), expected)

    def test_actual_container_process_uids_reject_root_empty_and_malformed_inventory(self):
        self.assertEqual(images.nonroot_process_uids('  PID   UID\n123 65532\n124 65532\n'), [65532])
        for value in ['', 'PID UID\n', 'PID UID\n123 0\n', 'PID UID\n123 65532\n124 0\n',
                      'UID\n65532\n', 'PID USER\n123 nonroot\n', 'PID UID\n123 root\n',
                      'PID UID\n0 65532\n', 'PID UID\n123 65532 extra\n']:
            with self.assertRaises(ValueError):
                images.nonroot_process_uids(value)

    def test_docker_top_command_includes_pid_and_verifies_nonroot(self):
        from unittest.mock import Mock
        docker = Mock(return_value=Mock(stdout='PID UID\n123 65532\n'))
        self.assertEqual(images.inspect_nonroot_processes(docker, 'owned-test-container'), [65532])
        docker.assert_called_once_with('top', 'owned-test-container', '-eo', 'pid,uid')
        docker.return_value.stdout = 'PID UID\n123 0\n'
        with self.assertRaises(ValueError):
            images.inspect_nonroot_processes(docker, 'owned-test-container')

    def test_security_candidates_are_pinned_and_keep_runtime_contracts(self):
        for service in ('api', 'worker', 'frontend'):
            source = (ROOT / f'deployment/{service}.Dockerfile').read_text()
            runtime = source.split(' AS runtime\n')[1]
            self.assertRegex(source, r'FROM gcr.io/distroless/[a-z0-9-]+:nonroot@sha256:[a-f0-9]{64} AS runtime')
            self.assertIn('USER 65532:65532', runtime)
            self.assertNotIn('apt-get', runtime)
            self.assertNotIn('"sh"', runtime)
            if service == 'frontend':
                self.assertIn('nodejs26-debian13', source)
                self.assertIn('node:26.13.0-trixie-slim@sha256:', source)
        source = (ROOT / images.POSTGRES_DOCKERFILE).read_text()
        self.assertRegex(source, r'FROM cgr.dev/chainguard/wolfi-base:latest@sha256:[a-f0-9]{64} AS runtime')
        self.assertNotIn('--allow-untrusted', source)
        self.assertNotIn('/edge/', source)
        self.assertIn('libxml2-16=2.15.4-r2', source)
        self.assertIn('postgresql-18-base=18.6-r7', source)
        self.assertIn('posix-libc-utils-2.44=2.44-r13', source)
        self.assertIn('glibc-2.44=2.44-r13', source)
        self.assertIn('glibc-2.44-locales=2.44-r13', source)
        self.assertIn('libLLVM-19=19.1.7-r31', source)
        self.assertNotIn('glibc-locales=2.43', source)
        self.assertNotIn('posix-libc-utils=2.43', source)
        self.assertIn("-name 'libxml2.so.2*'", source)
        self.assertIn('PGDATA=/var/lib/postgresql/data/pgdata', source)
        self.assertIn('deployment/postgres-entrypoint.sh', source)

    def test_failed_auth_contract_can_collect_more_evidence_but_remains_failed(self):
        from unittest.mock import Mock
        import contextlib
        import io
        failures = []
        with contextlib.redirect_stdout(io.StringIO()):
            images.expect_http(Mock(return_value=403),'/agents',200,'viewer',failures=failures)
            images.expect_http(Mock(return_value=401),'/agents',401,'expired',failures=failures)
        self.assertEqual(failures,['http_contract_failed_viewer'])

    def test_individual_test_evidence_survives_a_later_failure(self):
        log = "Running tests/live_work.rs (target/debug/deps/work)\ntest work_ok ... ok\n" + \
              "Running tests/wallet_read_permissions.rs (target/debug/deps/read)\ntest bad_read ... FAILED\n"
        events = postgres.rust_test_results(log)
        self.assertEqual([e['outcome'] for e in events],['ok','FAILED'])
        self.assertEqual(events[0]['target'],'tests/live_work.rs')
        unit = postgres.rust_test_results('Running unittests src/lib.rs (target/debug/deps/supa_domain-abc)\ntest safe ... ok')
        self.assertEqual(unit[0]['binary'],'supa_domain-abc')
        failure = postgres.rust_failure_evidence("thread 'bad_read' (123) panicked at crates/api/tests/wallet_read_permissions.rs:51:5:\nprivate prose")
        self.assertEqual(failure['panic_sites'][0]['line'],51)
        self.assertNotIn('private',json.dumps(failure))

    def test_http_error_only_allows_known_authentication_codes(self):
        from unittest.mock import Mock
        import io
        for code, expected in [('permission_denied',{'error_code':'permission_denied'}),
                               ('private_customer_prose',{})]:
            opener = Mock()
            opener.open.side_effect = images.urllib.error.HTTPError('http://fixture/agents',403,'private',{},
                io.BytesIO(json.dumps({'error':code}).encode()))
            diagnostic = {}
            with patch.object(images.urllib.request,'build_opener',return_value=opener):
                self.assertEqual(images.request(8080,'/agents',diagnostic=diagnostic),403)
            self.assertEqual(diagnostic,expected)

    def test_rust_failure_evidence_omits_panic_values_and_credentials(self):
        log = "test fixture_test ... FAILED\nthread 'fixture_test' panicked at crates/api/tests/live_work.rs:48:5:\n" + \
              'private customer prose secret postgres://fixture-host/fixture-db code: "42501" error[E0308]'
        evidence = postgres.rust_failure_evidence(log)
        self.assertEqual(evidence['failed_tests'],['fixture_test'])
        self.assertEqual(evidence['panic_sites'][0]['line'],48)
        self.assertEqual(evidence['sqlstates'],['42501'])
        self.assertEqual(evidence['compiler_codes'],['E0308'])
        self.assertNotIn('secret',json.dumps(evidence))
        self.assertNotIn('customer',json.dumps(evidence))

    def test_http_contract_reports_status_and_stage_without_secret_or_body(self):
        from unittest.mock import Mock
        import contextlib
        import io
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            with self.assertRaisesRegex(AssertionError, 'http_contract_failed_named_viewer'):
                images.expect_http(Mock(return_value=403), '/agents', 200, 'named_viewer', 'Bearer private-fixture')
        evidence = json.loads(output.getvalue())
        self.assertEqual(evidence['http_status'],403)
        self.assertEqual(evidence['stage'],'named_viewer')
        self.assertNotIn('private-fixture',output.getvalue())

    def test_held_postgres_package_is_installed_without_relaxing_health_check(self):
        import subprocess
        # A private dpkg database: no system package selections are changed.
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)
            for selection in ['hold', 'install']:
                (path/'status').write_text('Package: supa-package-fixture\n'
                    f'Status: {selection} ok installed\nPriority: optional\nSection: misc\n'
                    'Architecture: all\nVersion: 18.3-1\nDescription: synthetic fixture\n\n')
                query = ['dpkg-query', '--admindir='+temp, '-W']
                actual = subprocess.run(query+['-f=${Status}', 'supa-package-fixture'],
                    capture_output=True, text=True, check=True).stdout
                installed = subprocess.run(query+['-f=${db:Status-Status}', 'supa-package-fixture'],
                    capture_output=True, text=True, check=True).stdout
                self.assertEqual(actual, selection+' ok installed')
                self.assertEqual(installed, 'installed')
                self.assertEqual(subprocess.run(query+['-f=${db:Status-Eflag}', 'supa-package-fixture'],
                    capture_output=True, text=True, check=True).stdout, 'ok')
            (path/'status').write_text((path/'status').read_text().replace('install ok installed','hold reinstreq installed'))
            self.assertEqual(subprocess.run(query+['-f=${db:Status-Eflag}', 'supa-package-fixture'],
                capture_output=True, text=True, check=True).stdout, 'reinstreq')
            (path/'status').write_text((path/'status').read_text().replace('hold reinstreq installed','deinstall ok config-files'))
            self.assertEqual(subprocess.run(query+['-f=${db:Status-Status}', 'supa-package-fixture'],
                capture_output=True, text=True, check=True).stdout, 'config-files')
        candidate = (ROOT/'deployment/postgres.Dockerfile').read_text()
        self.assertIn("${db:Status-Status}", candidate)
        self.assertNotIn("= 'install ok installed'", candidate)

    def test_bridge_probe_accepts_only_one_owned_private_ipv4_network(self):
        self.assertEqual(images.container_address({'owned':{'IPAddress':'172.20.0.3'}},'owned'),'172.20.0.3')
        for ip in ['8.8.8.8','127.0.0.1','0.0.0.0','::1','192.0.2.5','169.254.1.1']:
            with self.subTest(ip=ip):
                with self.assertRaises(ValueError):
                    images.container_address({'owned':{'IPAddress':ip}},'owned')
        with self.assertRaises(ValueError):
            images.container_address({'owned':{'IPAddress':'172.20.0.3'},'egress':{}},'owned')

    def test_bridge_transport_keeps_logical_host_and_authentication(self):
        from unittest.mock import Mock
        response = Mock()
        response.__enter__ = Mock(return_value=Mock(status=401))
        response.__exit__ = Mock(return_value=False)
        opener = Mock(); opener.open.return_value = response
        with patch.object(images.urllib.request, 'build_opener', return_value=opener):
            self.assertEqual(images.request(3000,'/operator','Basic synthetic',host='172.20.0.3'),401)
        request = opener.open.call_args.args[0]
        self.assertEqual(request.full_url,'http://172.20.0.3:3000/operator')
        self.assertEqual(request.get_header('Host'),'127.0.0.1:3000')
        self.assertEqual(request.get_header('Authorization'),'Basic synthetic')

    def test_transport_diagnostics_exclude_url_headers_and_error_prose(self):
        from unittest.mock import Mock
        opener = Mock()
        opener.open.side_effect = images.urllib.error.URLError(ConnectionRefusedError(111,'private fixture prose'))
        diagnostic = {}
        with patch.object(images.urllib.request,'build_opener',return_value=opener):
            self.assertIsNone(images.request(8080,'/ready','Bearer private fixture',diagnostic=diagnostic))
        self.assertEqual(diagnostic,{'error_type':'ConnectionRefusedError','errno':111})
        opener.open.side_effect = images.urllib.error.HTTPError('http://127.0.0.1:8080/ready',503,'unavailable',{},None)
        with patch.object(images.urllib.request,'build_opener',return_value=opener):
            self.assertEqual(images.request(8080,'/ready',diagnostic=diagnostic),503)
        self.assertEqual(diagnostic,{})

    def test_internal_probe_enters_only_network_namespace_of_running_container(self):
        import subprocess
        from unittest.mock import Mock
        docker = Mock(return_value=subprocess.CompletedProcess([],0,'12345\n',''))
        with patch.object(images.subprocess,'run',return_value=subprocess.CompletedProcess([],0,'200\n','')) as run:
            self.assertTrue(images.internal_health(docker,'owned-container',8080,'/ready'))
        command = run.call_args.args[0]
        self.assertEqual(command[:8],['sudo','-n','nsenter','--target','12345','--net','--','/usr/bin/python3'])
        self.assertNotIn('--mount',command)
        docker.return_value = subprocess.CompletedProcess([],0,'1\n','')
        with self.assertRaises(ValueError): images.internal_health(docker,'owned-container',8080,'/ready')

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

    def test_scan_evidence_retains_all_severities_without_changing_gate(self):
        report = {'SchemaVersion': 2, 'Results': [{'Target': 'synthetic-package-inventory',
            'Vulnerabilities': [{'VulnerabilityID': 'CVE-' + severity, 'PkgName': 'example',
                                 'Severity': severity, 'InstalledVersion': '1', 'FixedVersion': '2'}
                                for severity in ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')]}]}
        evidence = images.scan_evidence(report)
        self.assertEqual({row['Severity'] for row in evidence['findings']}, {'LOW', 'MEDIUM', 'HIGH', 'CRITICAL'})
        with self.assertRaises(ValueError):
            images.vulnerability_gate(report)

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

    def test_build_failure_evidence_retains_first_solver_error_and_redacts_every_chunk(self):
        import contextlib, io
        secret = 'synthetic-sensitive-value-12345'
        text = 'ERROR: unable to select packages: ' + secret + '\n' + ('dependency chain\n' * 300) + 'final build failed'
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            images.emit_build_failure_evidence(text, [secret])
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        combined = ''.join(row['diagnostic'] for row in rows)
        self.assertIn('ERROR: unable to select packages:', combined)
        self.assertIn('dependency chain\n' * 300, combined)
        self.assertIn('final build failed', combined)
        self.assertNotIn(secret, combined)
        self.assertEqual([row['chunk'] for row in rows], list(range(1, len(rows) + 1)))
        self.assertTrue(all(not row['truncated'] and len(row['diagnostic']) <= 6000 for row in rows))

    def test_oversized_build_diagnostics_keep_head_and_tail_and_flag_truncation(self):
        import contextlib, io
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            images.emit_build_failure_evidence('first solver error\n' + 'x' * (3 * 1024 * 1024) + '\nlast build error')
        rows = [json.loads(line) for line in output.getvalue().splitlines()]
        combined = ''.join(row['diagnostic'] for row in rows)
        self.assertTrue(all(row['truncated'] for row in rows))
        self.assertTrue(combined.startswith('first solver error'))
        self.assertTrue(combined.endswith('last build error'))
        self.assertLess(len(combined), 2 * 1024 * 1024 + 100)

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
