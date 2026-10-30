#!/usr/bin/env python3
"""Manual GitHub-hosted CI only: build, execute and scan one disposable image.
No registry push, Railway commands, production credentials or database imports.
"""
import argparse
import base64
import hashlib
import http.client
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import socket
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


def vulnerability_gate(report):
    scan_shape(report)
    high = sum(v.get('Severity') in ('HIGH', 'CRITICAL')
               for result in report.get('Results', []) for v in result.get('Vulnerabilities') or [])
    leaked = sum(len(result.get('Secrets') or []) for result in report.get('Results', []))
    if high or leaked:
        raise ValueError(f'vulnerability_gate_failed_high_critical_{high}_secrets_{leaked}')


def copy_context(source, destination):
    paths = ['Cargo.toml', 'Cargo.lock', 'migrations', '.dockerignore', 'deployment']
    for crate in ('domain', 'persistence', 'providers', 'capabilities', 'runtime', 'api'):
        paths.extend([f'crates/{crate}/Cargo.toml', f'crates/{crate}/src'])
    paths.extend(['web/' + path for path in ('package.json', 'package-lock.json', 'next.config.ts',
                 'next-env.d.ts', 'tsconfig.json', 'postcss.config.mjs', 'src', 'public')])
    paths.extend(['reference-web/' + path for path in ('index.html', 'style.css', 'app.js',
                 'scene.js', 'wheel.js', 'platform.js', 'backend.mjs', 'backend.d.mts', 'public')])
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


def request(port, path, authorization=None):
    headers = {'Authorization': authorization} if authorization else {}
    try:
        with urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}{path}', headers=headers), timeout=8) as result:
            return result.status
    except urllib.error.HTTPError as error:
        return error.code
    except (OSError, TimeoutError, http.client.HTTPException):
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
    made_network = False
    build_log = work / 'build.log'

    def docker(*args, check=True):
        result = subprocess.run(['docker', *map(str, args)], capture_output=True, text=True, timeout=180)
        if check and result.returncode:
            raise RuntimeError('docker_validation_operation_failed_' + args[0])
        return result

    def sql(query):
        return docker('exec', database, 'psql', '-XqAt', '-U', 'postgres', '-d', 'supa_ci_test',
                      '-v', 'ON_ERROR_STOP=1', '-c', query).stdout.strip()

    try:
        metadata = work / 'build-metadata.json'
        with build_log.open('w') as log:
            build = subprocess.run(['docker', 'buildx', 'build', '--pull', '--load', '--provenance=false',
                                    '--metadata-file', str(metadata), '--tag', image, '--file',
                                    str(context / f'deployment/{service}.Dockerfile'), str(context)],
                                   stdout=log, stderr=subprocess.STDOUT, timeout=3000)
        if build.returncode:
            raise RuntimeError('image_build_failed_see_disposable_runner_build_log')
        built = json.loads(metadata.read_text())
        digest = built.get('containerimage.digest')
        if not digest or not re.fullmatch(r'sha256:[0-9a-f]{64}', digest):
            raise RuntimeError('buildkit_manifest_digest_missing')
        config = json.loads(docker('image', 'inspect', image).stdout)[0]
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
        dependency_report = {'Results': []}
        if service == 'api':
            manifests = work / 'manifests'
            for relative in ('Cargo.lock', 'Cargo.toml', 'web/package-lock.json', 'web/package.json',
                             '.railway/package-lock.json', '.railway/package.json'):
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
        # Do runtime checks even when advisories exist, but never mark the job passed.
        docker('network', 'create', '--internal', network)
        made_network = True
        password = secrets.token_hex(32)
        public, operator = secrets.token_hex(32), secrets.token_hex(32)
        credential = {'id': str(uuid.uuid4()), 'role': 'viewer', 'token': operator}
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            port = listener.getsockname()[1]
        env = {'SUPA_PRIVATE_STAGING': 'true', 'SUPA_PROVIDER': 'offline', 'SUPA_ALLOW_OPENAI': 'false',
               'SUPA_CONTROLLED_INFERENCE': 'false', 'SUPA_API_SCHEDULER_ENABLED': 'false',
               'SUPA_WALLET_ENABLED': 'false', 'SUPA_API_TOKEN': public,
               'SUPA_OPERATOR_CREDENTIALS': json.dumps([credential])}
        visitor_password = secrets.token_hex(24)
        if service != 'frontend':
            postgres = 'ghcr.io/railwayapp-templates/postgres-ssl:18'
            docker('pull', postgres)
            pg_env = work / 'postgres.env'
            pg_env.write_text('POSTGRES_DB=supa_ci_test\nPOSTGRES_PASSWORD=' + password + '\n')
            pg_env.chmod(0o600)
            # Fresh anonymous volume, removed with this owned test container only.
            owned_containers.append(database)
            docker('run', '--detach', '--name', database, '--network', network, '--env-file', pg_env, postgres)
            for _ in range(120):
                if docker('exec', database, 'pg_isready', '-U', 'postgres', '-d', 'supa_ci_test', check=False).returncode == 0:
                    break
                time.sleep(1)
            assert sql('SHOW server_version_num').startswith('18')
            postgres_digest = json.loads(docker('image', 'inspect', postgres).stdout)[0]['RepoDigests']
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
                dependency_report.setdefault('Results', []).extend(pg_report['Results'])
            authority = 'postgres:' + urllib.parse.quote(password, safe='') + '@' + database + ':5432'
            env['DATABASE_URL'] = urllib.parse.urlunsplit(('postgresql', authority, '/supa_ci_test', '', ''))
        else:
            postgres_digest = []
            env.update(SUPA_SITE_MODE='staging', SUPA_SITE_ORIGIN=f'http://127.0.0.1:{port}',
                       SUPA_BACKEND_URL='http://unavailable-ci-backend:8080',
                       SUPA_STAGING_USERNAME='ci-visitor', SUPA_STAGING_PASSWORD=visitor_password,
                       SUPA_WEB_OPERATORS=json.dumps([{'username': 'ci-operator',
                                                      'password': secrets.token_hex(24), 'api_token': operator}]))
        app_env = work / 'application.env'
        app_env.write_text(''.join(f'{key}={value}\n' for key, value in env.items()))
        app_env.chmod(0o600)
        owned_containers.insert(0, application)
        docker('run', '--detach', '--name', application, '--network', network, '--env-file', app_env,
               '--publish', f'127.0.0.1:{port}:{SERVICES[service]}', image)
        health = '/healthz' if service == 'frontend' else '/ready'
        for _ in range(120):
            if request(port, health) == 200:
                break
            time.sleep(1)
        assert request(port, health) == 200
        assert docker('exec', application, 'id', '-u').stdout.strip() != '0'
        if service == 'frontend':
            assert request(port, '/') == 401
            basic = 'Basic ' + base64.b64encode(('ci-visitor:' + visitor_password).encode()).decode()
            assert request(port, '/', basic) == 200
            assert request(port, '/operator', basic) == 401
        elif service == 'api':
            assert request(port, '/agents') == 401
            assert request(port, '/agents', 'Bearer ' + public) == 200
        else:
            defaults = dict(entry.split('=', 1) for entry in config['Config']['Env'])
            assert defaults.get('SUPA_WORKER_ENABLED') == 'false'
            assert sql('SELECT count(*) FROM worker_health WHERE enabled') == '0'
            assert sql('SELECT count(*) FROM cycles') == '0'
        if service != 'frontend':
            assert sql('SELECT count(*) FROM _sqlx_migrations WHERE success') == '10'
            docker('restart', database)
        docker('restart', application)
        for _ in range(120):
            if request(port, health) == 200:
                break
            time.sleep(1)
        assert request(port, health) == 200
        if service == 'worker':
            assert sql('SELECT count(*) FROM cycles') == '0'
        docker('stop', '--time', '20', application)
        assert docker('inspect', '--format', '{{.State.ExitCode}}', application).stdout.strip() == '0'
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
                  'sigterm_exit_code': 0, 'excluded_context_canary_absent': True,
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
