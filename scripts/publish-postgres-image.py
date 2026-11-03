#!/usr/bin/env python3
"""Manual owner-dispatched CI only. Copy one validated image, never rebuild it."""
import argparse
import base64
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import urllib.error
import urllib.request

REPOSITORY = 'ghcr.io/pyco404/supa-agent-postgres'
PACKAGE_API = 'https://api.github.com/users/pyco404/packages/container/supa-agent-postgres'
DIGEST = re.compile(r'sha256:[a-f0-9]{64}')


def require_context(env):
    required = {'GITHUB_ACTIONS': 'true', 'GITHUB_REPOSITORY': 'pyco404/SupaAgent',
                'GITHUB_REF': 'refs/heads/main', 'GITHUB_EVENT_NAME': 'workflow_dispatch',
                'RUNNER_ENVIRONMENT': 'github-hosted', 'GITHUB_ACTOR': 'pyco404',
                'GITHUB_RUN_ATTEMPT': '1'}
    if any(env.get(key) != value for key, value in required.items()):
        raise ValueError('owner_manual_main_hosted_first_attempt_required')
    sha = env.get('GITHUB_SHA', '')
    if not re.fullmatch(r'[a-f0-9]{40}', sha) or sha != env.get('EXPECTED_SOURCE_SHA'):
        raise ValueError('exact_approved_source_revision_required')
    if not re.fullmatch(r'[1-9][0-9]*', env.get('GITHUB_RUN_ID', '')):
        raise ValueError('valid_ci_run_required')


def file_hash(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source, 'sha256').hexdigest()


def descriptor(data, media_type):
    return {'mediaType': media_type, 'digest': 'sha256:' + hashlib.sha256(data).hexdigest(),
            'size': len(data)}


def check_candidate(record, env):
    if (record.get('validated') is not True or record.get('source_revision') != env['GITHUB_SHA'] or
            record.get('run_id') != env['GITHUB_RUN_ID'] or record.get('architecture') != 'amd64' or
            not DIGEST.fullmatch(record.get('image_id', '')) or
            not record.get('diff_ids') or
            any(not DIGEST.fullmatch(value) for value in record['diff_ids']) or
            record.get('compatibility') != {'xml': True, 'tls': True, 'locale': True,
                                          'restricted_api': True, 'restore': True}):
        raise ValueError('exact_validated_candidate_required')
    scan = record.get('scan', {})
    if (type(scan.get('secret_findings')) is not int or scan['secret_findings'] != 0 or
            not isinstance(scan.get('findings'), list) or
            any(row.get('Severity') in ('HIGH', 'CRITICAL') for row in scan['findings'])):
        raise ValueError('candidate_scan_failed_closed')
    # This is the operator's exact accepted PostgreSQL finding, not a scanner
    # exclusion. Preserve every row in evidence; new findings require review.
    for row in scan['findings']:
        if (row.get('VulnerabilityID'), row.get('PkgName'), row.get('InstalledVersion'), row.get('Severity')) != (
                'CVE-2026-39824', 'golang.org/x/sys', 'v0.1.0', 'UNKNOWN'):
            raise ValueError('unreviewed_candidate_advisory')
    archive = Path(record.get('archive', ''))
    runner = Path(env['RUNNER_TEMP']).resolve(strict=True)
    if (archive.is_symlink() or not archive.is_file() or
            not archive.resolve(strict=True).is_relative_to(runner) or
            file_hash(archive) != record.get('archive_sha256')):
        raise ValueError('validated_archive_identity_required')
    return archive


def canonical_layout(archive_path, destination, record):
    """Docker save -> canonical OCI, preserving config and every filesystem diff ID.

    No tar extraction, code execution, image modification or rebuild occurs here.
    The archive is already secret/vulnerability-scanned and runtime-validated.
    """
    destination = Path(destination)
    destination.mkdir(mode=0o700)
    blobs = destination / 'blobs' / 'sha256'
    blobs.mkdir(parents=True, mode=0o700)
    with tarfile.open(archive_path, 'r:*') as archive:
        members = archive.getmembers()
        names = [member.name for member in members]
        if len(names) > 4096 or len(set(names)) != len(names):
            raise ValueError('ambiguous_archive_members')

        def member(name):
            if not isinstance(name, str) or Path(name).is_absolute() or '..' in Path(name).parts:
                raise ValueError('unsafe_archive_path')
            value = archive.getmember(name)
            if not value.isfile():
                raise ValueError('regular_archive_member_required')
            return value

        manifest_member = member('manifest.json')
        if manifest_member.size > 1024 * 1024:
            raise ValueError('bounded_archive_manifest_required')
        images = json.load(archive.extractfile(manifest_member))
        if not isinstance(images, list) or len(images) != 1:
            raise ValueError('one_validated_platform_image_required')
        saved = images[0]
        config_member = member(saved['Config'])
        if config_member.size > 1024 * 1024:
            raise ValueError('bounded_image_config_required')
        config = archive.extractfile(config_member).read()
        config_descriptor = descriptor(config, 'application/vnd.oci.image.config.v1+json')
        metadata = json.loads(config)
        if (config_descriptor['digest'] != record['image_id'] or metadata.get('os') != 'linux' or
                metadata.get('architecture') != 'amd64' or
                metadata.get('rootfs', {}).get('diff_ids') != record['diff_ids'] or
                len(saved['Layers']) != len(record['diff_ids'])):
            raise ValueError('validated_config_and_filesystem_required')
        (blobs / config_descriptor['digest'][7:]).write_bytes(config)
        layers = []
        for path, expected in zip(saved['Layers'], record['diff_ids']):
            entry = member(path)
            digest = hashlib.sha256()
            output = blobs / ('pending-' + str(len(layers)))
            with archive.extractfile(entry) as source, output.open('wb') as target:
                # Canonical compression changes representation only; validate
                # the uncompressed filesystem hash before accepting the blob.
                with gzip.GzipFile(filename='', mode='wb', fileobj=target, mtime=0) as compressed:
                    while block := source.read(1024 * 1024):
                        digest.update(block)
                        compressed.write(block)
            if 'sha256:' + digest.hexdigest() != expected:
                raise ValueError('validated_layer_contents_required')
            blob_digest = file_hash(output)
            size = output.stat().st_size
            output.replace(blobs / blob_digest)
            layers.append({'mediaType': 'application/vnd.oci.image.layer.v1.tar+gzip',
                           'digest': 'sha256:' + blob_digest, 'size': size})
    manifest = json.dumps({'schemaVersion': 2, 'mediaType': 'application/vnd.oci.image.manifest.v1+json',
                           'config': config_descriptor, 'layers': layers},
                          sort_keys=True, separators=(',', ':')).encode()
    identity = descriptor(manifest, 'application/vnd.oci.image.manifest.v1+json')
    (blobs / identity['digest'][7:]).write_bytes(manifest)
    (destination / 'oci-layout').write_text('{"imageLayoutVersion":"1.0.0"}')
    (destination / 'index.json').write_text(json.dumps({'schemaVersion': 2, 'manifests': [
        {**identity, 'annotations': {'org.opencontainers.image.ref.name': 'validated'}}]}))
    return identity, json.loads(manifest)


def verify_layout(layout, identity, manifest):
    for row in [identity, manifest['config'], *manifest['layers']]:
        if not DIGEST.fullmatch(row['digest']):
            raise ValueError('invalid_content_digest')
        path = Path(layout) / 'blobs' / 'sha256' / row['digest'][7:]
        if (path.is_symlink() or path.stat().st_size != row['size'] or
                'sha256:' + file_hash(path) != row['digest']):
            raise ValueError('registry_content_digest_mismatch')


def http_json(url, authorization=None):
    accept = ('application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json'
              if '/manifests/' in url else 'application/vnd.github+json')
    headers = {'Accept': accept, 'User-Agent': 'supa-postgres-publication'}
    if authorization:
        headers['Authorization'] = authorization
    try:
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self, request, fp, code, msg, response_headers, newurl):
                raise ValueError('registry_redirect_requires_review')
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
        with opener.open(urllib.request.Request(url, headers=headers), timeout=30) as response:
            raw = response.read(1024 * 1024 + 1)
            if len(raw) > 1024 * 1024:
                raise ValueError('bounded_registry_metadata_required')
            return response.status, json.loads(raw)
    except urllib.error.HTTPError as error:
        return error.code, None  # Never emit headers, response prose or token values.


def registry_preflight(token):
    status, package = http_json(PACKAGE_API, 'Bearer ' + token)
    if status == 200:
        if package.get('visibility') != 'private':
            raise ValueError('existing_package_must_be_private')
    elif status != 404:
        raise ValueError('package_metadata_access_failed_http_' + str(status))
    # GitHub documents first publication as private by default. An inaccessible
    # existing public package must also be rejected using anonymous metadata.
    anonymous_status, anonymous_package = http_json(PACKAGE_API)
    if anonymous_status == 200:
        raise ValueError('anonymous_package_visibility_requires_review')
    if anonymous_status not in (401, 403, 404):
        raise ValueError('anonymous_metadata_check_failed_http_' + str(anonymous_status))
    basic = base64.b64encode(('pyco404:' + token).encode()).decode()
    scope = 'https://ghcr.io/token?service=ghcr.io&scope=repository:pyco404/supa-agent-postgres:pull,push'
    grant_status, grant = http_json(scope, 'Basic ' + basic)
    if grant_status != 200 or not isinstance(grant, dict) or not grant.get('token'):
        raise ValueError('registry_publication_scope_denied_http_' + str(grant_status))
    anonymous_pull_denied()
    # A scope grant is not proof of effective write permission. The validated
    # copy must succeed before publication permission can be reported verified.
    return {'package_metadata_status': status, 'requested_scope_granted': True,
            'private_default_if_created': status == 404}


def anonymous_pull_denied(reference=None):
    url = 'https://ghcr.io/token?service=ghcr.io&scope=repository:pyco404/supa-agent-postgres:pull'
    status, grant = http_json(url)
    if status in (401, 403):
        return True
    if status != 200 or not isinstance(grant, dict) or not grant.get('token'):
        raise ValueError('anonymous_pull_check_inconclusive')
    base = 'https://ghcr.io/v2/pyco404/supa-agent-postgres/'
    url = (base + 'manifests/' + reference.rsplit('@', 1)[1]
           if reference else base + 'tags/list?n=1')
    status, _ = http_json(url, 'Bearer ' + grant['token'])
    if status == 200:
        raise ValueError('private_manifest_anonymously_retrievable' if reference
                         else 'private_repository_anonymously_retrievable')
    if status not in (401, 403, 404):
        raise ValueError('anonymous_manifest_check_inconclusive')
    return True


def run(args, *, input=None):
    result = subprocess.run(args, input=input, capture_output=True, timeout=900)
    if result.returncode:
        raise RuntimeError('registry_tool_failed_exit_' + str(result.returncode))
    return result.stdout


def publish(record_path, token, env):
    runner = Path(env['RUNNER_TEMP']).resolve(strict=True)
    record_path = Path(record_path)
    if record_path.is_symlink() or not record_path.resolve(strict=True).is_relative_to(runner):
        raise ValueError('runner_candidate_record_required')
    record = json.loads(record_path.read_text())
    archive = check_candidate(record, env)
    work = Path(tempfile.mkdtemp(prefix='supa-publication-', dir=runner))
    auth = work / 'registry-auth.json'
    attempted = False
    try:
        identity, manifest = canonical_layout(archive, work / 'canonical', record)
        verify_layout(work / 'canonical', identity, manifest)
        registry_preflight(token)
        auth.write_text('{"auths":{}}')
        auth.chmod(0o600)
        run(['skopeo', 'login', '--authfile', str(auth), '--username', 'pyco404',
             '--password-stdin', 'ghcr.io'], input=(token + '\n').encode())
        tag = 'candidate-' + env['GITHUB_SHA'] + '-' + env['GITHUB_RUN_ID']
        reference = REPOSITORY + '@' + identity['digest']
        print(json.dumps({'stage': 'validated_canonical_image', 'reference': reference,
                          'config': manifest['config'], 'layers': manifest['layers'],
                          'findings': record['scan']['findings']}), flush=True)
        attempted = True
        run(['skopeo', 'copy', '--retry-times', '0', '--preserve-digests', '--authfile', str(auth),
             'oci:' + str(work / 'canonical') + ':validated', 'docker://' + REPOSITORY + ':' + tag])
        raw = run(['skopeo', 'inspect', '--raw', '--authfile', str(auth), 'docker://' + reference])
        if ('sha256:' + hashlib.sha256(raw).hexdigest() != identity['digest'] or
                json.loads(raw) != manifest):
            raise ValueError('published_manifest_identity_mismatch')
        run(['skopeo', 'copy', '--retry-times', '0', '--preserve-digests', '--authfile', str(auth),
             'docker://' + reference, 'oci:' + str(work / 'download') + ':verified'])
        verify_layout(work / 'download', identity, manifest)
        status, package = http_json(PACKAGE_API, 'Bearer ' + token)
        if status != 200 or package.get('visibility') != 'private':
            raise ValueError('published_package_privacy_unverified')
        anonymous_pull_denied(reference)
        report = {'stage': 'private_postgres_published_verified', 'reference': reference,
                  'source_revision': env['GITHUB_SHA'], 'run_id': env['GITHUB_RUN_ID'],
                  'config': manifest['config'], 'layers': manifest['layers'], 'private': True,
                  'anonymous_pull_denied': True, 'findings': record['scan']['findings'],
                  'secret_findings': 0, 'registry_pushes': 1, 'deployments': 0}
        print(json.dumps(report), flush=True)
        summary = env.get('GITHUB_STEP_SUMMARY')
        if summary:
            with open(summary, 'a') as output:
                output.write('\n```json\n' + json.dumps(report, indent=2) + '\n```\n')
    except Exception:
        if attempted:
            print(json.dumps({'stage': 'publication_outcome_requires_review',
                              'automatic_retry_permitted': False}), flush=True)
        raise
    finally:
        auth.unlink(missing_ok=True)
        # Canonical artifact and sanitized evidence remain on this disposable
        # runner. No credential or entire work-directory upload is permitted.


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', type=Path)
    parser.add_argument('--preflight-only', action='store_true')
    parser.add_argument('--authorize-private-publication', action='store_true')
    args = parser.parse_args()
    require_context(os.environ)
    token = os.environ.get('GH_TOKEN')
    if not token:
        raise ValueError('job_scoped_package_token_required')
    if args.preflight_only:
        print(json.dumps({'stage': 'publication_permission_preflight', **registry_preflight(token)}))
    elif not args.authorize_private_publication or args.candidate is None:
        raise ValueError('explicit_publication_and_candidate_required')
    else:
        publish(args.candidate, token, os.environ)


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        # Never serialize transport exceptions, subprocess stderr or credentials.
        code = str(error) if isinstance(error, (ValueError, RuntimeError)) else type(error).__name__
        if not re.fullmatch(r'[a-z_0-9]+', code):
            code = 'publication_failed_closed'
        print(json.dumps({'stage': 'publication_stopped', 'code': code}), flush=True)
        raise SystemExit(1)
