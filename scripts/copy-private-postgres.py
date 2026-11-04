#!/usr/bin/env python3
"""One owner-dispatched registry copy of the exact previously validated image."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import tempfile

spec = importlib.util.spec_from_file_location('publication', Path(__file__).with_name('publish-postgres-image.py'))
publication = importlib.util.module_from_spec(spec)
spec.loader.exec_module(publication)

SOURCE = 'ghcr.io/pyco404/supa-agent-postgres'
DESTINATION = 'ghcr.io/pyco404/supa-agent-postgres-private'
APPROVED_DIGEST = 'sha256:9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8'
APPROVED_CONFIG = 'sha256:0228d9646b8387201f20918ea903474f1b283d43c9a049e29ccda88701771abd'
SOURCE_API = 'https://api.github.com/users/pyco404/packages/container/supa-agent-postgres'
DESTINATION_API = SOURCE_API + '-private'


def metadata_preflight(token):
    # A known readable package is the control: never interpret an unprivileged
    # token's 404 as proof that a private destination is absent.
    status, source = publication.http_json(SOURCE_API, 'Bearer ' + token)
    if (status != 200 or not isinstance(source, dict) or
            source.get('name') != 'supa-agent-postgres' or source.get('visibility') != 'public'):
        raise ValueError('source_package_metadata_read_required')
    status, _ = publication.http_json(DESTINATION_API, 'Bearer ' + token)
    if status != 404:
        raise ValueError('new_destination_absence_required')
    anonymous_denied()


def anonymous_denied(reference=None):
    # Fresh urllib requests in http_json use no cookies, account token, Docker
    # auth file or environment proxy. This bearer is granted anonymously.
    status, grant = publication.http_json(
        'https://ghcr.io/token?service=ghcr.io&scope=repository:pyco404/supa-agent-postgres-private:pull')
    if status in (401, 403):
        return
    if status != 200 or not isinstance(grant, dict) or not grant.get('token'):
        raise ValueError('anonymous_check_inconclusive')
    suffix = 'manifests/' + reference.rsplit('@', 1)[1] if reference else 'tags/list?n=1'
    status, _ = publication.http_json('https://ghcr.io/v2/pyco404/supa-agent-postgres-private/' + suffix,
                                      'Bearer ' + grant['token'])
    # Once a manifest exists, a 404 is inconclusive rather than proof of privacy.
    if status not in ((401, 403) if reference else (401, 403, 404)):
        raise ValueError('anonymous_access_denial_required')


def manifest_identity(raw):
    if len(raw) > 1024 * 1024 or 'sha256:' + hashlib.sha256(raw).hexdigest() != APPROVED_DIGEST:
        raise ValueError('approved_source_manifest_required')
    manifest = json.loads(raw)
    if (manifest.get('schemaVersion') != 2 or manifest.get('config', {}).get('digest') != APPROVED_CONFIG or
            not isinstance(manifest.get('layers'), list) or not manifest['layers']):
        raise ValueError('approved_source_configuration_required')
    return {'digest': APPROVED_DIGEST, 'size': len(raw), 'mediaType': manifest['mediaType']}, manifest


def copy_image(token, env):
    publication.require_context(env)
    runner = Path(env['RUNNER_TEMP']).resolve(strict=True)
    work = Path(tempfile.mkdtemp(prefix='supa-private-copy-', dir=runner))
    auth = work / 'registry-auth.json'
    attempted = False
    try:
        metadata_preflight(token)
        with open(auth, 'x', opener=lambda name, flags: os.open(name, flags, 0o600)) as output:
            output.write('{"auths":{}}')
        publication.run(['skopeo', 'login', '--authfile', str(auth), '--username', 'pyco404',
                         '--password-stdin', 'ghcr.io'], input=(token + '\n').encode())
        source = SOURCE + '@' + APPROVED_DIGEST
        raw = publication.run(['skopeo', 'inspect', '--raw', '--authfile', str(auth), 'docker://' + source])
        identity, manifest = manifest_identity(raw)
        common = ['skopeo', 'copy', '--retry-times', '0', '--preserve-digests', '--authfile', str(auth)]
        publication.run(common + ['docker://' + source, 'oci:' + str(work / 'source') + ':validated'])
        publication.verify_layout(work / 'source', identity, manifest)
        # Recheck absence immediately before the single provisional write.
        metadata_preflight(token)
        reference = DESTINATION + '@' + APPROVED_DIGEST
        tag = 'validated-' + env['GITHUB_RUN_ID']
        attempted = True
        publication.run(common + ['oci:' + str(work / 'source') + ':validated', 'docker://' + DESTINATION + ':' + tag])
        # No other registry operation intervenes between upload and privacy read.
        status, package = publication.http_json(DESTINATION_API, 'Bearer ' + token)
        if (status != 200 or not isinstance(package, dict) or
                package.get('name') != 'supa-agent-postgres-private' or package.get('visibility') != 'private'):
            raise ValueError('destination_explicit_private_required')
        anonymous_denied(reference)
        copied = publication.run(['skopeo', 'inspect', '--raw', '--authfile', str(auth), 'docker://' + reference])
        if copied != raw:
            raise ValueError('destination_manifest_bytes_mismatch')
        publication.run(common + ['docker://' + reference, 'oci:' + str(work / 'destination') + ':verified'])
        publication.verify_layout(work / 'destination', identity, manifest)
        report = {'stage': 'private_copy_verified', 'source': source, 'reference': reference,
                  'visibility': package['visibility'], 'package_id': package.get('id'),
                  'anonymous_pull_denied': True, 'manifest': identity, 'config': manifest['config'],
                  'layers': manifest['layers'], 'source_revision': env['GITHUB_SHA'],
                  'run_id': env['GITHUB_RUN_ID'], 'registry_copy_attempts': 1,
                  'rebuilds': 0, 'deployments': 0,
                  'prior_validation_run': 'https://github.com/pyco404/SupaAgent/actions/runs/41723686098',
                  'residual_advisory': 'CVE-2026-39824 / golang.org/x/sys v0.1.0 / UNKNOWN; not rescanned by this copy'}
        evidence = work / 'verification.json'
        with open(evidence, 'x', opener=lambda name, flags: os.open(name, flags, 0o600)) as output:
            json.dump(report, output)
        print(json.dumps(report), flush=True)
        if env.get('GITHUB_STEP_SUMMARY'):
            with open(env['GITHUB_STEP_SUMMARY'], 'a') as output:
                output.write('\n```json\n' + json.dumps(report, indent=2) + '\n```\n')
        return report
    except Exception:
        print(json.dumps({'stage': 'copy_stopped', 'provisional_write_attempted': attempted,
                          'automatic_retry_permitted': False}), flush=True)
        raise
    finally:
        auth.unlink(missing_ok=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--authorize-provisional-private-copy', action='store_true')
    args = parser.parse_args()
    try:
        if not args.authorize_provisional_private_copy or not os.environ.get('GH_TOKEN'):
            raise ValueError('explicit_copy_and_job_token_required')
        copy_image(os.environ['GH_TOKEN'], os.environ)
    except Exception as error:
        code = str(error) if isinstance(error, (ValueError, RuntimeError)) else type(error).__name__
        if not re.fullmatch(r'[a-z_0-9]+', code):
            code = 'copy_failed_closed'
        print(json.dumps({'stage': 'copy_no_go', 'code': code}), flush=True)
        raise SystemExit(1)
