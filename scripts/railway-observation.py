#!/usr/bin/env python3
"""Deadline-bound read-only Railway transport for the deployment controller.

No mutations, credentials, output of CLI stderr, or deployment retries. All reads
share the caller's original readiness deadline; unavailable evidence never passes.
"""
import json
import re
import subprocess
import time
import urllib.error
import urllib.request
import urllib.parse
import ssl

DEPLOYMENT_QUERY = 'query($d:String!){deployment(id:$d){id status deploymentStopped projectId environmentId serviceId meta}}'
PROJECT_QUERY = 'query($id:String!){project(id:$id){id name workspaceId environments{edges{node{id name}}} services{edges{node{id name serviceInstances{edges{node{environmentId numReplicas hasEverDeployed source{image repo} activeDeployments{id status}}}}}}}}}'


class Observer:
    def __init__(self, guard, cli, project, environment, workspace, target, *,
                 record=lambda row: None, clock=time.monotonic):
        self.guard, self.cli = guard, cli
        self.project, self.environment, self.workspace, self.target = project, environment, workspace, target
        self.record, self.clock = record, clock

    def unavailable(self, stage, reason):
        self.record({'stage': stage, 'observation_available': False, 'reason': reason})
        raise self.guard['ObservationUnavailable']() from None

    def budget(self, deadline):
        remaining = deadline - self.clock()
        if remaining <= 0:
            raise self.guard['ActivationFailure']('activation_readiness_deadline_exceeded')
        return min(10, remaining)

    def read(self, stage, query, variables, deadline):
        if not re.match(r'^query(?:\s*\(|\s*\{)', query) or re.search(r'\bmutation\b', query):
            raise self.guard['ActivationFailure']('read_only_observation_query_required')
        try:
            result = subprocess.run([self.cli, 'api', query, '--compact', '--variables', '@-'],
                                    input=json.dumps(variables), text=True, capture_output=True,
                                    timeout=self.budget(deadline))
        except subprocess.TimeoutExpired:
            self.unavailable(stage, 'request_timeout')
        except OSError:
            self.unavailable(stage, 'transport_unavailable')
        if result.returncode:
            # Classify locally; never retain stderr, stdout, tokens or arguments.
            text = result.stderr.lower()
            reason = ('rate_limited' if '429' in text or 'rate limit' in text else
                      'upstream_unavailable' if any(str(code) in text for code in (500, 502, 503, 504)) else
                      'request_timeout' if 'timeout' in text or 'timed out' in text else
                      'authentication_unavailable' if '401' in text or '403' in text else
                      'request_failed')
            self.unavailable(stage, reason)
        try:
            if len(result.stdout) > 1048576:
                self.unavailable(stage, 'oversized_response')
            payload = json.loads(result.stdout)
            if not isinstance(payload, dict) or payload.get('errors') or not isinstance(payload.get('data'), dict):
                self.unavailable(stage, 'api_evidence_unavailable')
            return payload['data']
        except (ValueError, TypeError):
            self.unavailable(stage, 'invalid_response')

    def read_until(self, stage, query, variables, deadline, *, sleep=time.sleep):
        # One deadline across all attempts, including the subprocess time itself.
        max_reads = max(1, int(max(0, deadline - self.clock()) / 5) + 2)
        for _ in range(max_reads):
            try:
                value = self.read(stage, query, variables, deadline)
                self.budget(deadline)
                return value
            except self.guard['ObservationUnavailable']:
                sleep(min(5, self.budget(deadline)))
        raise self.guard['ActivationFailure']('activation_readiness_deadline_exceeded')

    def https_until(self, opener, request, deadline, *, sleep=time.sleep):
        url = urllib.parse.urlsplit(request.full_url)
        if (request.get_method() not in ('GET', 'HEAD') or url.scheme != 'https'
                or url.hostname not in ('supaagency.xyz', 'www.supaagency.xyz',
                                        'supa-web-staging.up.railway.app')
                or url.username or url.password):
            raise self.guard['ActivationFailure']('approved_read_only_https_required')
        max_reads = max(1, int(max(0, deadline - self.clock()) / 5) + 2)
        for _ in range(max_reads):
            reason = 'transport_unavailable'
            try:
                try:
                    response = opener.open(request, timeout=self.budget(deadline))
                except urllib.error.HTTPError as error:
                    response = error
                if self.clock() >= deadline:
                    response.close()
                    self.budget(deadline)
                if response.status != 429 and not 500 <= response.status <= 599:
                    return response  # Caller still validates the exact expected status/body.
                response.close()
                reason = 'upstream_unavailable'
            except urllib.error.URLError as error:
                if isinstance(error.reason, ssl.SSLCertVerificationError):
                    raise self.guard['ActivationFailure']('https_certificate_verification_failed') from None
            except ssl.SSLCertVerificationError:
                raise self.guard['ActivationFailure']('https_certificate_verification_failed') from None
            except OSError:
                reason = 'request_timeout'
            self.record({'stage': 'https_acceptance', 'observation_available': False, 'reason': reason})
            sleep(min(5, self.budget(deadline)))
        raise self.guard['ActivationFailure']('activation_readiness_deadline_exceeded')

    def fetch(self, remaining):
        deadline = self.clock() + remaining
        row = self.read('deployment', DEPLOYMENT_QUERY, {'d': self.target}, deadline).get('deployment')
        project = self.read('inventory', PROJECT_QUERY, {'id': self.project}, deadline).get('project')
        if not isinstance(row, dict) or not isinstance(project, dict):
            self.unavailable('inventory', 'missing_observation')
        # A missing field is unavailable evidence; a populated conflicting identity is critical.
        required = ('id', 'status', 'deploymentStopped', 'projectId', 'environmentId', 'serviceId')
        if not all(key in row and row[key] is not None for key in required):
            self.unavailable('deployment', 'incomplete_observation')
        if not all(key in project for key in ('id', 'name', 'workspaceId', 'environments', 'services')):
            self.unavailable('inventory', 'incomplete_observation')
        require = self.guard['require']
        require(project['id'] == self.project and project['workspaceId'] == self.workspace
                and project['name'] == 'Supa Agent', 'project_identity_mismatch')
        try:
            environments = [x['node'] for x in project['environments']['edges']]
            require({'id': self.environment, 'name': 'staging'} in environments, 'environment_identity_mismatch')
            services = {}
            for edge in project['services']['edges']:
                service = edge['node']
                matches = [x['node'] for x in service['serviceInstances']['edges']
                           if x['node']['environmentId'] == self.environment]
                if len(matches) != 1 or service['name'] in services:
                    self.unavailable('inventory', 'ambiguous_observation')
                instance = matches[0]
                if (not all(k in instance for k in ('source', 'hasEverDeployed', 'numReplicas', 'activeDeployments'))
                        or not isinstance(instance['source'], dict)
                        or not all(k in instance['source'] for k in ('image', 'repo'))
                        or not isinstance(instance['activeDeployments'], list)
                        or any(not isinstance(x, dict) or not isinstance(x.get('id'), str)
                               or not isinstance(x.get('status'), str) for x in instance['activeDeployments'])):
                    self.unavailable('inventory', 'incomplete_observation')
                services[service['name']] = instance
        except (KeyError, TypeError):
            self.unavailable('inventory', 'incomplete_observation')
        if row['status'] == 'SUCCESS' and (not isinstance(row.get('meta'), dict)
                or not all(isinstance(row['meta'].get(k), str) and row['meta'][k]
                           for k in ('image', 'imageDigest'))):
            self.unavailable('deployment', 'incomplete_observation')
        self.budget(deadline)
        return {'deployment': row, 'services': services}

    def probe(self, remaining):
        deadline = self.clock() + remaining
        checks = [('https://supaagency.xyz', p, 401) for p in
                  ('/operator', '/api/agents', '/api/operator-status', '/agents/new', '/wallets')]
        checks += [('https://supa-web-staging.up.railway.app', '/', 401)]
        checks += [('https://supaagency.xyz', p, 200) for p in ('/', '/home', '/agents', '/about')]
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
        for origin, path, expected in checks:
            try:
                try:
                    response = opener.open(urllib.request.Request(origin + path,
                        headers={'User-Agent': 'Supa-ReadOnly-Acceptance'}), timeout=self.budget(deadline))
                except urllib.error.HTTPError as error:
                    response = error
                with response:
                    body = response.read(600001)
                    status = response.status
                self.budget(deadline)
            except urllib.error.URLError as error:
                if isinstance(error.reason, ssl.SSLCertVerificationError):
                    raise self.guard['ActivationFailure']('https_certificate_verification_failed') from None
                self.unavailable('https_probe', 'transport_unavailable')
            except ssl.SSLCertVerificationError:
                raise self.guard['ActivationFailure']('https_certificate_verification_failed') from None
            except (OSError, TimeoutError):
                self.unavailable('https_probe', 'request_timeout')
            if len(body) > 600000:
                raise self.guard['ActivationFailure']('bounded_https_response_required')
            if status == 429 or 500 <= status <= 599:
                self.unavailable('https_probe', 'upstream_unavailable')
            if expected == 401 and status != expected:
                return self.guard['ProbeResult']('unsafe')
            if expected == 200 and (status != 200 or b'class="enter" href="/home"' not in body):
                return self.guard['ProbeResult']('pending')
        return self.guard['ProbeResult']('pass')


def monitor(guard, observer, expected, baseline, *, record=lambda row: None, **bounds):
    """The exact production observation path; only the guard decides readiness."""
    return guard['await_ready'](observer.fetch, observer.probe, expected, baseline,
                               record=record, **bounds)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args):
        return None
