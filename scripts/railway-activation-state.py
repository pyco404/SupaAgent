#!/usr/bin/env python3
"""Pure, bounded Railway activation observation; performs no mutations or network I/O."""
from dataclasses import dataclass
import math
import re
import time


class ActivationFailure(Exception):
    """Stable, non-sensitive failure code suitable for a recovery decision."""


class ObservationUnavailable(Exception):
    """A read failed; never implies that a deployment mutation should be retried."""


@dataclass(frozen=True)
class ProbeResult:
    state: str  # pass, pending, unsafe


TERMINAL = frozenset({'FAILED', 'CRASHED', 'REMOVED', 'CANCELED', 'CANCELLED', 'SKIPPED'})


def require(value, code):
    if not value:
        raise ActivationFailure(code)


def native_image_access(row, expected):
    """Verify native deployment availability, without handling registry credentials.

    This is deployment-time evidence, not a preflight token-validity test: Railway
    may cache images. Package privacy/association and registry integrity remain
    independent publication requirements. Never serialize raw deployment metadata.
    """
    require(isinstance(expected.get('image'), str) and
            re.fullmatch(r'ghcr\.io/[a-z0-9._-]+/[a-z0-9._-]+@sha256:[0-9a-f]{64}',
                         expected['image']) is not None, 'immutable_private_image_required')
    require(isinstance(row, dict), 'deployment_observation_invalid')
    for field in ('id', 'projectId', 'environmentId', 'serviceId'):
        require(row.get(field) == expected[field], 'deployment_identity_mismatch')
    require(row.get('status') not in TERMINAL, 'target_deployment_failed')
    if row.get('status') != 'SUCCESS' or row.get('deploymentStopped') is not False:
        return False
    meta = row.get('meta') or {}
    require(meta.get('image') == expected['image'], 'target_image_mismatch')
    require(meta.get('imageDigest') == expected['image'].split('@', 1)[1], 'target_digest_mismatch')
    return True


def deployment_ready(row, expected):
    return native_image_access(row, expected)


def inspect_inventory(services, baseline, expected):
    require(set(services) == set(baseline), 'service_inventory_changed')
    for name in ('supa-api', 'supa-postgres'):
        require(services[name] == baseline[name], 'protected_backend_changed')
    web = services['supa-web']
    require(web.get('source') == {'image': expected['image'], 'repo': None}, 'frontend_source_changed')
    stable = lambda value: {k: v for k, v in value.items() if k != 'activeDeployments'}
    require(stable(web) == stable(baseline['supa-web']), 'frontend_settings_changed')
    rows = web.get('activeDeployments')
    require(isinstance(rows, list), 'active_inventory_invalid')
    known = {r['id'] for r in baseline['supa-web']['activeDeployments']}
    allowed = known | {expected['id']}
    require(all(isinstance(r, dict) and r.get('id') in allowed for r in rows), 'unknown_active_deployment')
    require(len({r['id'] for r in rows}) == len(rows), 'duplicate_active_deployment')
    target = next((r for r in rows if r['id'] == expected['id']), None)
    if target:
        require(target.get('status') not in TERMINAL, 'target_active_deployment_failed')
    # Terminal/stale entries for known OLD deployments are normal teardown metadata.
    return {
        'target_listed_ready': bool(target and target.get('status') == 'SUCCESS'),
        'inventory_settled': rows == [{'id': expected['id'], 'status': 'SUCCESS'}],
        'known_old_entries': sum(r['id'] in known for r in rows),
    }


def await_ready(fetch, probe, expected, baseline, *, timeout=300, interval=5,
                stable_passes=2, clock=time.monotonic, sleep=time.sleep, record=lambda row: None):
    """Accept two full security passes, never singleton inventory alone.

    Callbacks receive the remaining time and must bound their own read timeouts.
    Only ObservationUnavailable is retried; all mutations live outside this module.
    Audit records contain enumerated facts, never raw exceptions, variables or output.
    """
    require(math.isfinite(timeout) and timeout > 0 and math.isfinite(interval)
            and interval > 0 and type(stable_passes) is int and stable_passes >= 2,
            'invalid_polling_bounds')
    deadline = clock() + timeout
    passes = 0
    # A second bound also protects against a non-advancing clock in a bad harness.
    max_polls = math.ceil(timeout / interval) + stable_passes + 1
    for number in range(1, max_polls + 1):
        remaining = deadline - clock()
        require(remaining > 0, 'activation_readiness_deadline_exceeded')
        audit = {'poll': number, 'read_available': False, 'security_passes': passes}
        try:
            observed = fetch(remaining)
            require(clock() < deadline, 'activation_readiness_deadline_exceeded')
            ready = deployment_ready(observed['deployment'], expected)
            inventory = inspect_inventory(observed['services'], baseline, expected)
            audit.update(read_available=True, target_ready=ready, **inventory)
            if ready and inventory['target_listed_ready']:
                result = probe(deadline - clock())
                require(clock() < deadline, 'activation_readiness_deadline_exceeded')
                require(isinstance(result, ProbeResult) and result.state in {'pass', 'pending', 'unsafe'},
                        'security_probe_invalid')
                require(result.state != 'unsafe', 'public_private_security_failure')
                passes = passes + 1 if result.state == 'pass' else 0
                audit['probe_state'] = result.state
            else:
                passes = 0
        except ObservationUnavailable:
            passes = 0
            audit['read_unavailable'] = True
        except ActivationFailure as error:
            audit['failure_code'] = str(error)
            record(audit)
            raise
        except Exception:
            record({**audit, 'failure_code': 'controller_observation_error'})
            raise ActivationFailure('controller_observation_error') from None
        audit['security_passes'] = passes
        record(audit)
        if passes >= stable_passes:
            return {'ready': True, 'security_passes': passes,
                    'inventory_settled': audit['inventory_settled'],
                    'known_old_entries': audit['known_old_entries'], 'polls': number}
        remaining = deadline - clock()
        require(remaining > 0, 'activation_readiness_deadline_exceeded')
        sleep(min(interval, remaining))
    raise ActivationFailure('activation_readiness_deadline_exceeded')
