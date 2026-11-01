#!/usr/bin/env python3
"""Private Claude enrollment/checks. External metadata GET requires explicit separate approval."""
import argparse
import getpass
import http.client
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import sys
import tempfile
import warnings

ROOT = Path(__file__).resolve().parent.parent
PRIVATE = ROOT / '.env.claude'
MODELS = ('claude-haiku-5-5', 'claude-sonnet-5-5')
KEYS = ('ANTHROPIC_API_KEY', 'ANTHROPIC_WORKSPACE_ID', 'ANTHROPIC_MODEL')


def require(condition):
    if not condition:
        raise ValueError('private_claude_configuration_rejected')


def read_private(path=PRIVATE):
    try:
        info = path.lstat()
    except FileNotFoundError:
        return {}, None
    require(stat.S_ISREG(info.st_mode) and info.st_uid == os.geteuid() and not info.st_mode & 0o077 and info.st_nlink == 1 and info.st_size <= 8192)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    with os.fdopen(fd, 'rb') as stream:
        opened = os.fstat(stream.fileno())
        require((info.st_dev, info.st_ino) == (opened.st_dev, opened.st_ino))
        raw = stream.read(8193)
    require(len(raw) <= 8192)
    values = {}
    for line in raw.decode('ascii').splitlines():
        if not line.strip() or line.startswith('#'):
            continue
        key, separator, value = line.partition('=')
        require(separator and key in KEYS and key not in values and re.fullmatch(r'[A-Za-z0-9_-]+', value))
        values[key] = value
    return values, (info.st_dev, info.st_ino)


def write_private(values, expected, path=PRIVATE):
    require(set(values).issubset(KEYS))
    require(all(re.fullmatch(r'[A-Za-z0-9_-]+', value) for value in values.values()))
    payload = ''.join(key + '=' + values[key] + '\n' for key in KEYS if key in values)
    require(len(payload.encode('ascii')) <= 8192)
    # Concurrent replacement is detected; no overwriting of unsafe or unrelated config.
    _, identity = read_private(path)
    require(identity == expected)
    fd, name = tempfile.mkstemp(prefix='.env.claude-', dir=path.parent)
    temporary = Path(name)
    try:
        os.fchmod(fd, 0o600)
        with os.fdopen(fd, 'w', encoding='ascii') as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        if expected is None:
            # Exclusive link avoids racing an existing file; final file has one link after unlink.
            os.link(temporary, path, follow_symlinks=False)
            temporary.unlink()
        else:
            _, identity = read_private(path)
            require(identity == expected)
            os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


def key_valid(key):
    return bool(re.fullmatch(r'sk-ant-[A-Za-z0-9_-]{8,500}', key))


def configuration(values, environ=None):
    environ = os.environ if environ is None else environ
    return {key: environ.get(key, values.get(key, '')) for key in KEYS}


def check(values, file_present=False):
    effective = configuration(values)
    key, workspace, model = (effective[k] for k in KEYS)
    return {'private_file': '.env.claude', 'private_file_present': file_present,
            'key_present': bool(key), 'basic_key_format_valid': key_valid(key),
            'key_source': 'process_environment' if 'ANTHROPIC_API_KEY' in os.environ else 'private_file' if 'ANTHROPIC_API_KEY' in values else 'missing',
            'workspace_configured': bool(workspace), 'workspace_format_valid': bool(re.fullmatch(r'wrkspc_[A-Za-z0-9_]{1,113}', workspace)),
            'model': model if model in MODELS else None, 'authentication_verified': False,
            'billing_verified': False, 'external_requests': 0, 'generation_requests': 0,
            'inference_enabled': False}


def probe(values, authorized=False):
    require(authorized)
    effective = configuration(values)
    key, workspace, model = (effective[k] for k in KEYS)
    require(key_valid(key) and re.fullmatch(r'wrkspc_[A-Za-z0-9_]{1,113}', workspace) and model in MODELS)
    report = {'method': 'GET', 'model': model, 'metadata_requests': 1, 'generation_requests': 0,
              'automatic_retries': 0, 'redirects_followed': 0, 'authentication_verified': False,
              'billing_verified': False, 'inference_permission_verified': False}
    connection = http.client.HTTPSConnection('api.anthropic.com', timeout=15)
    try:
        connection.request('GET', '/v1/models/' + model, headers={'Authorization': 'Bearer ' + key,
                           'anthropic-version': '2026-10-21', 'anthropic-workspace-id': workspace})
        response = connection.getresponse()
        report['http_status'] = response.status
        body = response.read(16385)
        if len(body) > 16384:
            report['result'] = 'response_limit'
            return report
        if response.status == 200:
            try:
                value = json.loads(body)
            except (ValueError, UnicodeError):
                value = {}
            valid = value.get('type') == 'model' and value.get('id') == model
            report['authentication_verified'] = valid
            report['result'] = 'authenticated_metadata_access' if valid else 'invalid_metadata_response'
        else:
            report['result'] = {401: 'authentication_rejected', 403: 'scope_access_denied', 404: 'model_unavailable', 429: 'rate_limited'}.get(response.status, 'http_error')
    except Exception:
        report['result'] = 'transport_failed_no_automatic_retry'
    finally:
        connection.close()
    return report


class QuietParser(argparse.ArgumentParser):
    def error(self, message):
        raise ValueError('invalid_private_configuration_arguments')


def main():
    parser = QuietParser(description=__doc__)
    parser.add_argument('action', choices=['set-key', 'set-workspace', 'check', 'probe'])
    parser.add_argument('--workspace-id')
    parser.add_argument('--model', choices=MODELS)
    parser.add_argument('--authorize-metadata-check', action='store_true')
    args = parser.parse_args()
    if args.action != 'probe':
        require(not args.authorize_metadata_check)
    # Normal repository policy: do not store secrets unless this exact path is ignored/untracked.
    require(subprocess.run([str(ROOT / 'scripts/git-local'), 'check-ignore', '--quiet', '.env.claude'], cwd=ROOT).returncode == 0)
    require(not subprocess.check_output([str(ROOT / 'scripts/git-local'), 'ls-files', '--', '.env.claude'], cwd=ROOT))
    values, identity = read_private()
    if args.action == 'set-key':
        require(sys.stdin.isatty() and not args.workspace_id)
        with warnings.catch_warnings():
            warnings.simplefilter('error', getpass.GetPassWarning)
            key = getpass.getpass('Anthropic API key (hidden): ')
        require(key_valid(key))
        values['ANTHROPIC_API_KEY'] = key
        values.setdefault('ANTHROPIC_MODEL', args.model or MODELS[0])
        write_private(values, identity)
        print(json.dumps({'key_saved': True, 'private_file': '.env.claude', 'mode': '0600', 'external_requests': 0, 'inference_enabled': False}))
    elif args.action == 'set-workspace':
        require(args.workspace_id and re.fullmatch(r'wrkspc_[A-Za-z0-9_]{1,113}', args.workspace_id))
        values['ANTHROPIC_WORKSPACE_ID'] = args.workspace_id
        if args.model:
            values['ANTHROPIC_MODEL'] = args.model
        write_private(values, identity)
        print(json.dumps({'workspace_saved': True, 'evaluation_scope_updated': False, 'external_requests': 0}))
    elif args.action == 'check':
        require(not args.workspace_id and not args.model)
        print(json.dumps(check(values, identity is not None), indent=2))
    else:
        effective = configuration(values)
        require(args.workspace_id and args.model and effective['ANTHROPIC_WORKSPACE_ID'] == args.workspace_id and effective['ANTHROPIC_MODEL'] == args.model)
        print(json.dumps(probe(values, args.authorize_metadata_check), indent=2))


if __name__ == '__main__':
    try:
        main()
    except (Exception, KeyboardInterrupt):
        # Never render secret input, parser values, raw provider data, or exception details.
        print(json.dumps({'error': 'private_claude_operation_failed_closed', 'generation_requests': 0}), file=sys.stderr)
        sys.exit(1)
