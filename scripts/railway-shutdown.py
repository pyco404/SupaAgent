#!/usr/bin/env python3
"""Approval-gated deadline watchdog. Validation is offline; no scheduled install."""
import argparse
import fcntl
import hashlib
import json
import os
import stat
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

PROJECT = '99648d2a-69a1-4d8c-98d0-7f2a26543c9c'
ENVIRONMENT = 'ff0b2092-e42b-4a89-ae25-441ab95d92d8'
REVISION = 'bfc29f500af08e0e2cc4676b8950b5b17f26007a'
IMAGE = ('ghcr.io/pyco404/supa-agent-postgres-infra-98c1006fcb1d@sha256:'
         '9409009a40c0929052306d068c14bef92f3ecc67662c453540e2c9b95ae18fc8')
TERMINAL = {'REMOVED', 'FAILED', 'CRASHED', 'SKIPPED'}
PENDING = {'BUILDING', 'INITIALIZING', 'NEEDS_APPROVAL', 'QUEUED', 'WAITING'}
RUNNING = {'SUCCESS', 'DEPLOYING', 'SLEEPING'}
STATUSES = TERMINAL | PENDING | RUNNING | {'REMOVING'}


class GuardError(Exception):
    """Stable local codes only: never relay remote error bodies or credentials."""


def require(condition, code):
    if not condition:
        raise GuardError(code)


def identifier(value):
    try:
        require(isinstance(value, str) and str(uuid.UUID(value)) == value
                and uuid.UUID(value).int != 0, 'invalid_identifier')
    except (ValueError, TypeError, AttributeError):
        raise GuardError('invalid_identifier') from None
    return value


def load_scope(raw):
    require(len(raw) <= 16384, 'scope_too_large')
    try:
        def unique_fields(pairs):
            require(len({key for key, _ in pairs}) == len(pairs), 'duplicate_scope_field')
            return dict(pairs)
        scope = json.loads(raw, object_pairs_hook=unique_fields)
    except (ValueError, UnicodeError):
        raise GuardError('invalid_scope_json') from None
    require(isinstance(scope, dict) and set(scope) == {
        'version', 'project_id', 'environment_id', 'application_revision',
        'postgres_image', 'services', 'started_at_unix', 'shutdown_at_unix',
        'deadline_unix', 'independent_runner_verified', 'autodeploy_disabled',
        'synthetic_only', 'worker_replicas'}, 'scope_fields')
    require(type(scope['version']) is int and scope['version'] == 1 and scope['project_id'] == PROJECT
            and scope['environment_id'] == ENVIRONMENT, 'wrong_target')
    require(scope['application_revision'] == REVISION
            and scope['postgres_image'] == IMAGE, 'wrong_revision_or_image')
    require(scope['independent_runner_verified'] is True
            and scope['autodeploy_disabled'] is True
            and scope['synthetic_only'] is True
            and type(scope['worker_replicas']) is int
            and scope['worker_replicas'] == 0, 'prerequisites_not_attested')
    services = scope['services']
    require(isinstance(services, dict) and set(services) in (
        {'frontend', 'api', 'postgres'}, {'frontend', 'api', 'postgres', 'worker'}),
        'service_allowlist_required')
    ids = [identifier(value) for value in services.values()]
    require(len(set(ids)) == len(ids), 'duplicate_service')
    for key in ('started_at_unix', 'shutdown_at_unix', 'deadline_unix'):
        require(type(scope[key]) is int and scope[key] > 0, 'invalid_deadline')
    require(scope['shutdown_at_unix'] == scope['started_at_unix'] + 19800
            and scope['deadline_unix'] == scope['started_at_unix'] + 21600,
            'six_hour_deadline_required')
    return scope, hashlib.sha256(raw).hexdigest()


def check_rows(scope, rows):
    require(isinstance(rows, list) and len(rows) <= 1000, 'inventory_bound')
    seen = set()
    for row in rows:
        require(isinstance(row, dict) and set(row) == {
            'id', 'projectId', 'environmentId', 'serviceId', 'status',
            'deploymentStopped'}, 'inventory_fields')
        identifier(row['id'])
        require(row['projectId'] == PROJECT and row['environmentId'] == ENVIRONMENT
                and row['serviceId'] in scope['services'].values(), 'inventory_target_mismatch')
        require(row['id'] not in seen, 'duplicate_deployment')
        require(row['status'] in STATUSES
                and type(row['deploymentStopped']) is bool, 'unknown_deployment_state')
        seen.add(row['id'])


def actions(scope, rows):
    """Cancel replacements; observe application shutdown before stopping PostgreSQL."""
    check_rows(scope, rows)
    pg = scope['services']['postgres']
    applications_stopped = all(r['status'] in TERMINAL and r['deploymentStopped']
                               for r in rows if r['serviceId'] != pg)
    result = []
    for row in rows:
        if row['status'] in PENDING:
            result.append(('deploymentCancel', row['id']))
        elif row['status'] in RUNNING or (
                row['status'] in TERMINAL and not row['deploymentStopped']):
            if row['serviceId'] != pg or applications_stopped:
                result.append(('deploymentStop', row['id']))
    return result


class Audit:
    def __init__(self, path):
        self.fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        info = os.fstat(self.fd)
        if not (stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
                and stat.S_IMODE(info.st_mode) == 0o600 and info.st_nlink == 1):
            os.close(self.fd)
            raise GuardError('unsafe_audit_file')
        try:
            fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            os.close(self.fd)
            raise GuardError('watchdog_audit_already_locked') from None

    def write(self, event, **fields):
        require(os.fstat(self.fd).st_size < 2_000_000, 'audit_size_bound')
        data = (json.dumps({'event': event, **fields}, sort_keys=True) + '\n').encode()
        require(os.write(self.fd, data) == len(data), 'audit_write_failed')
        os.fsync(self.fd)

    def close(self):
        os.close(self.fd)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise GuardError('provider_redirect_rejected')


class Railway:
    """Only environment-scoped project tokens; no CLI/account credential fallback."""
    def __init__(self, token):
        require(isinstance(token, str) and 16 <= len(token) <= 4096
                and token.isascii() and not any(c.isspace() for c in token), 'project_token_required')
        self.token = token
        self.client = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    def query(self, query, variables=None):
        request = urllib.request.Request('https://backboard.railway.com/graphql/v2',
            data=json.dumps({'query': query, 'variables': variables or {}}).encode(),
            headers={'Project-Access-Token': self.token, 'Content-Type': 'application/json'})
        try:
            with self.client.open(request, timeout=8) as response:
                raw = response.read(2_000_001)
            require(len(raw) <= 2_000_000, 'provider_response_bound')
            result = json.loads(raw)
            require(isinstance(result, dict) and not result.get('errors')
                    and isinstance(result.get('data'), dict), 'provider_schema_error')
            return result['data']
        except urllib.error.HTTPError as error:
            raise GuardError('provider_http_' + str(error.code)) from None
        except (urllib.error.URLError, TimeoutError, OSError):
            raise GuardError('provider_connection_failure') from None
        except (ValueError, UnicodeError):
            raise GuardError('provider_invalid_json') from None

    def snapshot(self, scope):
        identity = self.query('query { projectToken { projectId environmentId } }')
        require(identity.get('projectToken') == {'projectId': PROJECT, 'environmentId': ENVIRONMENT},
                'project_token_target_mismatch')
        env = self.query('query($id: String!) { environment(id: $id) { id projectId '
                         'serviceInstances { edges { node { serviceId } } pageInfo { hasNextPage } } } }', {'id': ENVIRONMENT})['environment']
        require(env['id'] == ENVIRONMENT and env['projectId'] == PROJECT, 'environment_mismatch')
        ids = [edge['node']['serviceId'] for edge in env['serviceInstances']['edges']]
        require(env['serviceInstances']['pageInfo']['hasNextPage'] is False
                and len(ids) == len(set(ids)) and set(ids) == set(scope['services'].values()),
                'service_inventory_mismatch')
        rows, cursor, cursors = [], None, set()
        for _ in range(10):
            data = self.query('query($input: DeploymentListInput!, $after: String) {'
                ' deployments(input: $input, first: 100, after: $after) {'
                ' edges { node { id projectId environmentId serviceId status deploymentStopped } }'
                ' pageInfo { hasNextPage endCursor } } }',
                {'input': {'projectId': PROJECT, 'environmentId': ENVIRONMENT,
                           'includeDeleted': True}, 'after': cursor})['deployments']
            rows.extend(edge['node'] for edge in data['edges'])
            if not data['pageInfo']['hasNextPage']:
                check_rows(scope, rows)
                return rows
            cursor = data['pageInfo']['endCursor']
            require(isinstance(cursor, str) and cursor and cursor not in cursors,
                    'invalid_pagination')
            cursors.add(cursor)
        raise GuardError('inventory_page_bound')

    def mutate(self, operation, deployment_id):
        require(operation in {'deploymentStop', 'deploymentCancel'}, 'operation_forbidden')
        identifier(deployment_id)
        data = self.query('mutation($id: String!) { ' + operation + '(id: $id) }',
                          {'id': deployment_id})
        require(data.get(operation) is True, 'shutdown_not_acknowledged')


def watch(scope, digest, provider, audit, wall=time.time, monotonic=time.monotonic, sleep=time.sleep):
    started_wall, started_mono = wall(), monotonic()
    require(started_wall <= scope['deadline_unix'] + 1800, 'cleanup_authorization_expired')
    require(started_wall >= scope['started_at_unix'] - 300, 'start_time_too_far_ahead')
    effective_now = lambda: max(wall(), started_wall + monotonic() - started_mono)
    sent, stable, initial = set(), 0, True
    while True:
        now = effective_now()
        require(now <= scope['deadline_unix'] + 1800, 'shutdown_unverified_by_cleanup_deadline')
        rows = provider.snapshot(scope)
        check_rows(scope, rows)
        now = effective_now()
        worker = scope['services'].get('worker')
        # A zero-replica worker must never have pending or live deployments.
        require(all(r['status'] in TERMINAL and r['deploymentStopped']
                    for r in rows if r['serviceId'] == worker), 'worker_not_at_zero')
        if initial:
            audit.write('armed', scope_sha256=digest, deadline_unix=scope['deadline_unix'])
            initial = False
        if now >= scope['shutdown_at_unix']:
            for operation, deployment_id in actions(scope, rows):
                if (operation, deployment_id) in sent:
                    continue
                # Durable intent before any mutation; no automatic mutation retry on errors.
                audit.write('shutdown_intent', operation=operation, deployment_id=deployment_id)
                sent.add((operation, deployment_id))
                provider.mutate(operation, deployment_id)
                audit.write('shutdown_acknowledged', operation=operation, deployment_id=deployment_id)
            quiet = all(r['status'] in TERMINAL and r['deploymentStopped'] for r in rows)
            stable = stable + 1 if quiet else 0
            if now >= scope['deadline_unix'] and stable >= 2:
                audit.write('compute_stopped_verified', scope_sha256=digest,
                            storage_cleanup_authorized=False)
                return {'compute_stopped_verified': True, 'storage_cleanup_authorized': False}
        sleep(10)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['validate', 'run'])
    parser.add_argument('--scope', required=True)
    parser.add_argument('--scope-sha256')
    parser.add_argument('--authorize-shutdown', action='store_true')
    parser.add_argument('--audit-file')
    args = parser.parse_args(argv)
    audit = None
    try:
        with Path(args.scope).open('rb') as source:
            scope, digest = load_scope(source.read(16385))
        if args.command == 'validate':
            print(json.dumps({'valid': True, 'scope_sha256': digest, 'network_requests': 0}))
            return 0
        require(args.authorize_shutdown and args.scope_sha256 == digest,
                'exact_scope_shutdown_approval_required')
        require(args.audit_file is not None, 'audit_file_required')
        audit = Audit(args.audit_file)
        provider = Railway(os.environ.get('SUPA_SHUTDOWN_PROJECT_TOKEN'))
        print(json.dumps(watch(scope, digest, provider, audit)))
        return 0
    except Exception as error:
        code = str(error) if isinstance(error, GuardError) else 'local_or_provider_protocol_failure'
        if audit:
            try:
                audit.write('shutdown_failed_requires_operator', code=code)
            except Exception:
                pass
        print(json.dumps({'ok': False, 'code': code, 'operator_action_required': True}))
        return 1
    finally:
        if audit:
            audit.close()


if __name__ == '__main__':
    raise SystemExit(main())
