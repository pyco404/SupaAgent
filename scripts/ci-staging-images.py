#!/usr/bin/env python3
"""Manual GitHub-hosted CI only: build, execute and scan one disposable image.
No registry push, Railway commands, production credentials or database imports.
"""
import argparse
import base64
import hashlib
import http.client
import ipaddress
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parent.parent
SERVICES = {'frontend': 3000, 'api': 8080, 'worker': 8081}
CANARY = b'SUPA_CI_EXCLUDED_CONTEXT_CANARY_6A_NOT_A_CREDENTIAL'


def require_hosted_ci(env):
    if any(env.get(key) != value for key, value in {
        'GITHUB_ACTIONS': 'true', 'GITHUB_REPOSITORY': 'pyco404/SupaAgent',
        'GITHUB_REF': 'refs/heads/main', 'RUNNER_ENVIRONMENT': 'github-hosted',
        'GITHUB_EVENT_NAME': 'workflow_dispatch',
    }.items()):
        raise ValueError('manual_official_main_github_hosted_ci_required')


def scan_shape(report):
    if not isinstance(report, dict) or report.get('SchemaVersion') != 2 or not isinstance(report.get('Results'), list):
        raise ValueError('scanner_report_missing_or_unsupported')


def migration_inventory(directory):
    """Exact reviewed source inventory, never a hard-coded historical count."""
    inventory = {}
    for source in sorted(directory.glob('*.sql')):
        match = re.fullmatch(r'(\d{4})_[a-z0-9_]+\.sql', source.name)
        if not match or source.is_symlink() or int(match[1]) in inventory:
            raise ValueError('invalid_migration_source_inventory')
        inventory[int(match[1])] = hashlib.sha384(source.read_bytes()).hexdigest()
    if not inventory or sorted(inventory) != list(range(1, len(inventory) + 1)):
        raise ValueError('noncontiguous_migration_source_inventory')
    return inventory


def vulnerability_gate(report):
    scan_shape(report)
    high = sum(v.get('Severity') in ('HIGH', 'CRITICAL')
               for result in report.get('Results', []) for v in result.get('Vulnerabilities') or [])
    leaked = sum(len(result.get('Secrets') or []) for result in report.get('Results', []))
    if high or leaked:
        raise ValueError(f'vulnerability_gate_failed_high_critical_{high}_secrets_{leaked}')


def scan_evidence(report):
    """Only public advisory metadata; never emit matched secret text."""
    scan_shape(report)
    return {'findings': [
        {'target': result.get('Target'), **{key: item.get(key) for key in
         ('VulnerabilityID', 'PkgName', 'Severity', 'InstalledVersion', 'FixedVersion')}}
        for result in report['Results'] for item in result.get('Vulnerabilities') or []],
        'secret_findings': sum(len(result.get('Secrets') or []) for result in report['Results'])}


def clean_shutdown(service, exit_code):
    # Next.js may report conventional SIGTERM (128+15) after graceful shutdown.
    # Docker's timeout/SIGKILL (137), crashes and all nonzero Rust exits still fail.
    return exit_code in ((0, 143) if service == 'frontend' else (0,))


def copy_context(source, destination):
    paths = ['Cargo.toml', 'Cargo.lock', 'migrations', '.dockerignore', 'deployment']
    for crate in ('domain', 'persistence', 'providers', 'capabilities', 'runtime', 'api'):
        paths.extend([f'crates/{crate}/Cargo.toml', f'crates/{crate}/src'])
    paths.extend(['web/' + path for path in ('package.json', 'package-lock.json', 'next.config.ts',
                 'next-env.d.ts', 'tsconfig.json', 'postcss.config.mjs', 'src', 'public')])
    paths.extend(['reference-web/' + path for path in ('index.html', 'not-found.html', 'style.css', 'app.js',
                 'scene.js', 'wheel.js', 'platform.js', 'backend.mjs', 'launch.mjs', 'backend.d.mts', 'public')])
    for relative in paths:
        src, dst = source / relative, destination / relative
        if src.is_symlink() or (src.is_dir() and any(p.is_symlink() for p in src.rglob('*'))):
            raise ValueError('symlinks_forbidden_in_ci_build_context')
        dst.parent.mkdir(parents=True, exist_ok=True)
        if src.is_dir():
            shutil.copytree(src, dst)
        else:
            shutil.copy2(src, dst)
    for relative in ('.env.ci-sentinel', 'web/.env.local', '.supa-secrets/canary'):
        path = destination / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(CANARY)


def contains_canary(path):
    tail = b''
    with path.open('rb') as data:
        while chunk := data.read(1024 * 1024):
            if CANARY in tail + chunk:
                return True
            tail = chunk[-len(CANARY):]
    return False


def safe_ci_diagnostic(text, private_values=()):
    """Bound failure evidence; never output container environment/inspect config."""
    text = re.sub(r'\x1b\[[0-?]*[ -/]*[@-~]', '', text)
    for value in sorted(set(private_values), key=len, reverse=True):
        if value and len(value) >= 16:
            text = text.replace(value, '[redacted]')
    text = re.sub(r'(?i)(?:postgres(?:ql)?|https?)://[^\s/@]+:[^\s/@]+@', '[redacted-url]@', text)
    text = re.sub(r'\bsk-[A-Za-z0-9_-]{20,}', '[redacted-key]', text)
    text = ''.join(c for c in text if c in '\n\t' or ord(c) >= 32)
    return '\n'.join(text.splitlines()[-40:])[-12000:]


def wait_for_database(docker, database, private_values):
    """Fail at the actual startup stage and retain bounded, scrubbed logs."""
    for _ in range(120):
        running = docker('inspect', '--format', '{{.State.Running}}', database).stdout.strip()
        if running != 'true':
            break
        if docker('exec', database, 'pg_isready', '-h', '127.0.0.1',
                  '-U', 'postgres', '-d', 'supa_ci_test', check=False).returncode == 0:
            return
        time.sleep(1)
    logs = docker('logs', '--tail', '40', database, check=False)
    print(json.dumps({'stage': 'database_startup_failed',
                      'diagnostic': safe_ci_diagnostic(logs.stdout + logs.stderr, private_values)}), flush=True)
    raise RuntimeError('database_startup_failed')


def container_address(networks, network):
    if set(networks) != {network}:
        raise ValueError('application_must_have_only_its_owned_internal_network')
    address = ipaddress.ip_address(networks[network]['IPAddress'])
    ranges = [ipaddress.ip_network(cidr) for cidr in ('10.0.0.0/8','172.16.0.0/12','192.168.0.0/16')]
    if address.version != 4 or not any(address in subnet for subnet in ranges):
        raise ValueError('private_container_ipv4_required')
    return str(address)


def internal_health(docker, application, port, path):
    # Join only the owned container's network namespace, not its mount/user namespace.
    # Uses the hosted Linux runner's Python; no app privileges or outbound route added.
    pid = int(docker('inspect', '--format', '{{.State.Pid}}', application).stdout.strip())
    if pid <= 1:
        raise ValueError('running_owned_container_required')
    code = "import urllib.request; o=urllib.request.build_opener(urllib.request.ProxyHandler({})); print(o.open('http://127.0.0.1:%d%s',timeout=8).status)" % (port, path)
    result = subprocess.run(['sudo', '-n', 'nsenter', '--target', str(pid), '--net',
                             '--', '/usr/bin/python3', '-c', code],
                            capture_output=True, text=True, timeout=15)
    return result.returncode == 0 and result.stdout.strip() == '200'


def request(port, path, authorization=None, host='127.0.0.1', diagnostic=None):
    if diagnostic is not None:
        diagnostic.clear()
    headers = {'Authorization': authorization} if authorization else {}
    # Preserve the reviewed logical origin while routing over the private bridge.
    headers['Host'] = f'127.0.0.1:{port}'
    try:
        # Never send local health/auth probes through runner proxy configuration.
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        with opener.open(urllib.request.Request(f'http://{host}:{port}{path}', headers=headers), timeout=8) as result:
            return result.status
    except urllib.error.HTTPError as error:
        return error.code
    except (OSError, TimeoutError, http.client.HTTPException) as error:
        if diagnostic is not None:
            reason = getattr(error, 'reason', error)
            diagnostic.update(error_type=type(reason).__name__, errno=getattr(reason, 'errno', None))
        return None


def validate(service, trivy):
    require_hosted_ci(os.environ)
    if shutil.disk_usage(ROOT).free < 12 * 1024 ** 3:
        raise ValueError('at_least_12_gib_free_required_on_disposable_ci_runner')
    work = Path(tempfile.mkdtemp(prefix='supa-ci-image-'))
    context = work / 'context'
    context.mkdir()
    copy_context(ROOT, context)
    suffix = uuid.uuid4().hex
    image = 'supa-ci-' + service + ':' + suffix
    network = 'supa-ci-' + suffix
    database = 'supa-db-' + suffix
    application = 'supa-app-' + suffix
    owned_containers = []
    private_values = []
    made_network = False
    build_log = work / 'build.log'

    def docker(*args, check=True, timeout=180):
        result = subprocess.run(['docker', *map(str, args)], capture_output=True, text=True, timeout=timeout)
        if check and result.returncode:
            print(json.dumps({'stage': 'docker_operation_failed', 'operation': args[0],
                              'diagnostic': safe_ci_diagnostic(result.stderr, private_values)}), flush=True)
            raise RuntimeError('docker_validation_operation_failed_' + args[0])
        return result

    def sql(query):
        return docker('exec', database, 'psql', '-XqAt', '-h', '127.0.0.1', '-U', 'postgres', '-d', 'supa_ci_test',
                      '-v', 'ON_ERROR_STOP=1', '-c', query).stdout.strip()

    try:
        metadata = work / 'build-metadata.json'
        with build_log.open('w') as log:
            build = subprocess.run(['docker', 'buildx', 'build', '--pull', '--load', '--provenance=false',
                                    '--metadata-file', str(metadata), '--tag', image, '--file',
                                    str(context / f'deployment/{service}.Dockerfile'), str(context)],
                                   stdout=log, stderr=subprocess.STDOUT, timeout=3000)
        if build.returncode:
            print(json.dumps({'stage': 'image_build_failed', 'service': service,
                              'diagnostic': safe_ci_diagnostic(build_log.read_text(errors='replace'))}), flush=True)
            raise RuntimeError('image_build_failed_see_disposable_runner_build_log')
        built = json.loads(metadata.read_text())
        digest = built.get('containerimage.digest')
        if not digest or not re.fullmatch(r'sha256:[0-9a-f]{64}', digest):
            raise RuntimeError('buildkit_manifest_digest_missing')
        config = json.loads(docker('image', 'inspect', image).stdout)[0]
        assert config['Os'] == 'linux' and config['Architecture'] == 'amd64'
        print(json.dumps({'stage': 'image_built', 'service': service,
                          'manifest_digest': digest, 'config_image_id': config['Id'],
                          'architecture': config['Architecture'],
                          'runtime_validated': False, 'scan_passed': False}), flush=True)
        assert config['Config']['User'] not in ('', '0', 'root', '0:0')
        archive = work / 'image.tar'
        docker('image', 'save', '--output', archive, image)
        assert not contains_canary(archive), 'excluded_canary_leaked_into_image_layers'
        docker('run', '--rm', '--network', 'none', '--entrypoint', '/bin/sh', image, '-c',
               'test ! -e /app/web/.env.local && test ! -e /app/.env.ci-sentinel && test ! -e /app/.supa-secrets/canary')
        scanner_result = work / 'image-scan.json'
        scan = subprocess.run([str(trivy), 'image', '--input', str(archive), '--scanners', 'vuln,secret',
                               '--ignorefile', '/dev/null', '--format', 'json', '--output', str(scanner_result)],
                              cwd=work, stdout=subprocess.DEVNULL, timeout=900)
        if scan.returncode:
            raise RuntimeError('container_scanner_failed_closed')
        image_report = json.loads(scanner_result.read_text())
        scan_shape(image_report)
        print(json.dumps({'stage': 'image_scanned', 'service': service,
                          'manifest_digest': digest,
                          'high_critical': sum(v.get('Severity') in ('HIGH', 'CRITICAL')
                              for r in image_report['Results'] for v in r.get('Vulnerabilities') or []),
                          'high_critical_findings': [
                              {'Target': r.get('Target'), **{k:v.get(k) for k in ('VulnerabilityID','PkgName','Severity','InstalledVersion','FixedVersion')}}
                              for r in image_report['Results'] for v in r.get('Vulnerabilities') or []
                              if v.get('Severity') in ('HIGH','CRITICAL')],
                          'secret_findings': sum(len(r.get('Secrets') or []) for r in image_report['Results']),
                          'runtime_validated': False}), flush=True)
        dependency_report = {'Results': []}
        if service == 'api':
            manifests = work / 'manifests'
            for relative in ('Cargo.lock', 'Cargo.toml', 'web/package-lock.json', 'web/package.json',
                             '.railway/package-lock.json', '.railway/package.json',
                             *[str(p.relative_to(ROOT)) for p in (ROOT / 'crates').glob('*/Cargo.toml')]):
                target = manifests / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, target)
            deps = work / 'dependencies.json'
            checked = subprocess.run([str(trivy), 'fs', '--scanners', 'vuln', '--ignorefile', '/dev/null',
                                      '--format', 'json', '--output', str(deps), str(manifests)],
                                     cwd=work, stdout=subprocess.DEVNULL, timeout=900)
            if checked.returncode:
                raise RuntimeError('dependency_scanner_failed_closed')
            dependency_report = json.loads(deps.read_text())
            scan_shape(dependency_report)
            print(json.dumps({'stage': 'dependencies_scanned', **scan_evidence(dependency_report)}), flush=True)
        # Do runtime checks even when advisories exist, but never mark the job passed.
        docker('network', 'create', '--internal', network)
        made_network = True
        password = secrets.token_hex(32)
        public, operator = secrets.token_hex(32), secrets.token_hex(32)
        credential = {'id': str(uuid.uuid4()), 'role': 'viewer', 'token': operator}
        port = SERVICES[service]
        env = {'SUPA_PRIVATE_STAGING': 'true', 'SUPA_PROVIDER': 'offline', 'SUPA_ALLOW_OPENAI': 'false',
               'SUPA_ALLOW_ANTHROPIC': 'false', 'SUPA_CONTROLLED_WORK': 'false',
               'SUPA_CONTROLLED_INFERENCE': 'false', 'SUPA_API_SCHEDULER_ENABLED': 'false',
               'SUPA_WALLET_ENABLED': 'false', 'SUPA_API_TOKEN': public,
               'SUPA_OPERATOR_CREDENTIALS': json.dumps([credential]),
               'SUPA_OPERATOR_SESSION_DEADLINES': json.dumps({credential['id']: int(time.time()) + 1800})}
        visitor_password, operator_password = secrets.token_hex(24), secrets.token_hex(24)
        private_values.extend([password,public,operator,visitor_password,operator_password])
        if service != 'frontend':
            postgres = 'supa-ci-postgres:' + suffix
            pg_metadata = work / 'postgres-build-metadata.json'
            docker('buildx', 'build', '--pull', '--load', '--provenance=false',
                   '--metadata-file', pg_metadata, '--tag', postgres,
                   '--file', context / 'deployment/postgres.Dockerfile', context, timeout=900)
            postgres_manifest_digest = json.loads(pg_metadata.read_text()).get('containerimage.digest')
            assert re.fullmatch(r'sha256:[0-9a-f]{64}', postgres_manifest_digest or '')
            pg_config = json.loads(docker('image', 'inspect', postgres).stdout)[0]
            assert pg_config['Os'] == 'linux' and pg_config['Architecture'] == config['Architecture']
            postgres_digest = pg_config['RepoDigests']
            print(json.dumps({'stage': 'postgres_candidate_built', 'repo_digests': postgres_digest,
                              'candidate_manifest_digest': postgres_manifest_digest,
                              'architecture': pg_config['Architecture']}), flush=True)
            # Self-test the actual privilege-drop binary and dynamic XML linkage.
            gosu_version = docker('run', '--rm', '--network', 'none', '--entrypoint',
                                  '/usr/local/bin/gosu', postgres, '--version').stdout.strip()
            assert 'go1.27.3' in gosu_version
            print(json.dumps({'stage': 'postgres_gosu_verified', 'version': gosu_version}), flush=True)
            pg_env = work / 'postgres.env'
            # PGPASSWORD is inherited by exec probes; never put it in argv/logs.
            pg_env.write_text('POSTGRES_DB=supa_ci_test\nPGDATA=/var/lib/postgresql/data/pgdata\nPOSTGRES_PASSWORD=' + password + '\nPGPASSWORD=' + password + '\n')
            pg_env.chmod(0o600)
            # Fresh anonymous volume, removed with this owned test container only.
            owned_containers.append(database)
            docker('run', '--detach', '--name', database, '--network', network,
                   '--mount', 'type=volume,destination=/var/lib/postgresql/data', '--env-file', pg_env, postgres)
            # The initdb bootstrap server is socket-only. Wait for the final
            # TCP listener and diagnose an exited container before SQL probes.
            wait_for_database(docker, database, private_values)
            postgres_version = sql('SHOW server_version_num')
            assert postgres_version.startswith('18')
            assert sql("SELECT xml_is_well_formed('<supa/>')") == 't'
            print(json.dumps({'stage':'postgres_initialized','server_version_num':postgres_version,
                              'xml_support_preserved':True}),flush=True)
            if service == 'api':
                pg_archive, pg_scan = work / 'postgres.tar', work / 'postgres-scan.json'
                docker('image', 'save', '--output', pg_archive, postgres)
                checked = subprocess.run([str(trivy), 'image', '--input', str(pg_archive), '--scanners',
                                          'vuln,secret', '--ignorefile', '/dev/null', '--format', 'json',
                                          '--output', str(pg_scan)], cwd=work, timeout=900)
                if checked.returncode:
                    raise RuntimeError('postgres_image_scanner_failed_closed')
                pg_report = json.loads(pg_scan.read_text())
                scan_shape(pg_report)
                print(json.dumps({'stage': 'postgres_scanned', **scan_evidence(pg_report)}), flush=True)
                dependency_report.setdefault('Results', []).extend(pg_report['Results'])
            authority = 'postgres:' + urllib.parse.quote(password, safe='') + '@' + database + ':5432'
            admin_url = urllib.parse.urlunsplit(('postgresql', authority, '/supa_ci_test', '', ''))
            migration_env = work / 'migration.env'
            migration_env.write_text('SUPA_DISPOSABLE_SCHEMA=true\nDATABASE_URL=' + admin_url + '\n')
            migration_env.chmod(0o600)
            # One explicit administrator bootstrap; ordinary application startup never migrates.
            docker('run', '--rm', '--network', network, '--env-file', migration_env,
                   '--entrypoint', '/usr/local/bin/supa-staging-schema', image)
            runtime_password = secrets.token_hex(32)
            private_values.append(runtime_password)
            sql("CREATE ROLE supa_ci_runtime LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE "
                "NOREPLICATION NOBYPASSRLS PASSWORD '" + runtime_password + "'")
            sql('REVOKE CREATE ON SCHEMA public FROM PUBLIC; '
                'GRANT USAGE ON SCHEMA public TO supa_ci_runtime; '
                'GRANT SELECT ON ALL TABLES IN SCHEMA public TO supa_ci_runtime; '
                'GRANT INSERT,UPDATE ON worker_health TO supa_ci_runtime; '
                'GRANT UPDATE ON live_sessions TO supa_ci_runtime')
            for forbidden in ('CREATE TABLE public.forbidden_ci(id int)',
                              'UPDATE _sqlx_migrations SET success=false',
                              'UPDATE agents SET simulated_balance_microusd=1',
                              'DELETE FROM epoch_guard.approvals'):
                denied = docker('exec', database, 'psql', '-XqAt', '-h', '127.0.0.1',
                                '-U', 'postgres', '-d', 'supa_ci_test', '-v', 'ON_ERROR_STOP=1',
                                '-c', 'SET ROLE supa_ci_runtime; ' + forbidden, check=False)
                assert denied.returncode != 0 and 'permission denied' in denied.stderr
            runtime_authority = 'supa_ci_runtime:' + runtime_password + '@' + database + ':5432'
            env['DATABASE_URL'] = urllib.parse.urlunsplit(('postgresql', runtime_authority, '/supa_ci_test', '', ''))
        else:
            postgres_digest = []
            postgres_manifest_digest = None
            env.update(SUPA_SITE_MODE='staging', SUPA_SITE_ORIGIN=f'http://127.0.0.1:{port}',
                       SUPA_BACKEND_URL='http://unavailable-ci-backend:8080',
                       SUPA_STAGING_USERNAME='ci-visitor', SUPA_STAGING_PASSWORD=visitor_password,
                       SUPA_WEB_OPERATORS=json.dumps([{'username': 'ci-operator',
                                                      'password': operator_password, 'api_token': operator}]))
        app_env = work / 'application.env'
        app_env.write_text(''.join(f'{key}={value}\n' for key, value in env.items()))
        app_env.chmod(0o600)
        owned_containers.insert(0, application)
        docker('run', '--detach', '--name', application, '--network', network, '--env-file', app_env, image)
        assert json.loads(docker('network', 'inspect', network).stdout)[0]['Internal'] is True
        def address():
            return container_address(json.loads(docker('inspect', '--format',
                '{{json .NetworkSettings.Networks}}', application).stdout), network)
        host = address()
        transport_diagnostic = {}
        def probe(path, authorization=None):
            return request(port, path, authorization, host=host, diagnostic=transport_diagnostic)
        health = '/healthz' if service == 'frontend' else '/ready'
        for _ in range(120):
            if probe(health) == 200:
                break
            time.sleep(1)
        health_status = probe(health)
        if health_status != 200:
            logs = docker('logs', '--tail', '40', application, check=False)
            print(json.dumps({'stage': 'application_health_failed', 'service': service,
                              'http_status': health_status,
                              'transport': transport_diagnostic,
                              'diagnostic': safe_ci_diagnostic(logs.stdout + logs.stderr, private_values)}), flush=True)
            if service == 'frontend':
                probe = docker('exec', application, 'node', '-e',
                    "require('http').get('http://127.0.0.1:3000/healthz',r=>{console.log(JSON.stringify({internal_http_status:r.statusCode}));r.resume()}).on('error',e=>{console.log(JSON.stringify({internal_error:e.code}));process.exitCode=1})",
                    check=False)
                print(json.dumps({'stage': 'frontend_internal_probe',
                                  'diagnostic': safe_ci_diagnostic(probe.stdout + probe.stderr)}), flush=True)
            raise RuntimeError('application_health_failed')
        assert internal_health(docker, application, port, health), 'internal_namespace_health_failed'
        print(json.dumps({'stage': 'application_health_passed', 'service': service,
                          'internal_http_status': 200, 'runner_bridge_http_status': 200,
                          'network_internal': True, 'published_ports': 0}), flush=True)
        assert docker('exec', application, 'id', '-u').stdout.strip() != '0'
        if service == 'frontend':
            assert probe('/') == 401
            basic = 'Basic ' + base64.b64encode(('ci-visitor:' + visitor_password).encode()).decode()
            assert probe('/', basic) == 200
            assert probe('/operator', basic) == 401
        elif service == 'api':
            assert probe('/agents') == 401
            assert probe('/agents', 'Bearer ' + public) == 200
            assert probe('/agents', 'Bearer invalid-test-token') == 401
        else:
            defaults = dict(entry.split('=', 1) for entry in config['Config']['Env'])
            assert defaults.get('SUPA_WORKER_ENABLED') == 'false'
            assert sql('SELECT count(*) FROM worker_health WHERE enabled') == '0'
            assert sql('SELECT count(*) FROM cycles') == '0'
        if service != 'frontend':
            actual = json.loads(sql("SELECT json_agg(json_build_object('version',version,"
                                    "'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) "
                                    'FROM _sqlx_migrations'))
            expected = migration_inventory(ROOT / 'migrations')
            assert len(actual) == len(expected)
            assert {row['version']: row['checksum'] for row in actual if row['success']} == expected
            docker('restart', database)
        docker('restart', application)
        host = address()
        for _ in range(120):
            if probe(health) == 200:
                break
            time.sleep(1)
        assert probe(health) == 200
        assert internal_health(docker, application, port, health)
        if service == 'worker':
            assert sql('SELECT count(*) FROM cycles') == '0'
        docker('stop', '--time', '20', application)
        exit_code = int(docker('inspect', '--format', '{{.State.ExitCode}}', application).stdout.strip())
        assert clean_shutdown(service, exit_code)
        image_report.setdefault('Results', []).extend(dependency_report.get('Results', []))
        findings = [{'id': v.get('VulnerabilityID'), 'package': v.get('PkgName'),
                     'severity': v.get('Severity'), 'installed': v.get('InstalledVersion'),
                     'fixed': v.get('FixedVersion')}
                    for result in image_report.get('Results', [])
                    for v in result.get('Vulnerabilities') or [] if v.get('Severity') in ('HIGH', 'CRITICAL')]
        from collections import Counter
        counts = dict(Counter(v.get('Severity', 'UNKNOWN') for result in image_report['Results']
                              for v in result.get('Vulnerabilities') or []))
        other_ids = sorted({v.get('VulnerabilityID', 'unknown') for result in image_report['Results']
                            for v in result.get('Vulnerabilities') or []
                            if v.get('Severity') not in ('HIGH', 'CRITICAL')})
        report = {'vulnerability_severity_counts': counts, 'other_advisory_ids': other_ids,
                  'high_critical_findings': findings, 'service': service, 'manifest_digest': digest, 'config_image_id': config['Id'],
                  'postgres_repo_digests': postgres_digest, 'non_root': True, 'health_start_and_restart': True,
                  'postgres_candidate_manifest_digest': postgres_manifest_digest,
                  'sigterm_exit_code': exit_code, 'excluded_context_canary_absent': True,
                  'restricted_runtime_credentials': service != 'frontend',
                  'runtime_schema_create_and_financial_update_denied': service != 'frontend',
                  'worker_disabled': service == 'worker', 'registry_pushes': 0, 'deployments': 0}
        print(json.dumps(report), flush=True)
        summary = os.environ.get('GITHUB_STEP_SUMMARY')
        if summary:
            with open(summary, 'a') as output:
                output.write('\n```json\n' + json.dumps(report, indent=2) + '\n```\n')
        vulnerability_gate(image_report)
    finally:
        for name in owned_containers:
            docker('rm', '--force', '--volumes', name, check=False)
        if made_network:
            docker('network', 'rm', network, check=False)
        for path in work.glob('*.env'):
            path.unlink(missing_ok=True)
        # Test images/caches/logs exist only on the disposable hosted runner.


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--service', choices=SERVICES, required=True)
    parser.add_argument('--trivy', type=Path, required=True)
    arguments = parser.parse_args()
    validate(arguments.service, arguments.trivy.resolve(strict=True))
