#!/usr/bin/env python3
"""Non-paid checks against the locked, scoped disposable API. Never submits paid consent."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import urllib.error
import urllib.request
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('scoped', ROOT / 'scripts/openai-work-api.py')
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)


def main():
    os.chdir(ROOT)
    plan = api.frozen()
    runtime = api.private_json()
    assert runtime['scope_sha256'] == api.SCOPE and runtime['paid_dispatch_authorized'] is False
    assert api.process_matches(runtime), 'Scoped API process unavailable'
    before = api.protected()
    baseline = json.loads((api.OUT / 'protected-before.json').read_text())
    assert before == baseline['protected']
    for path, digest in baseline['historical_files'].items():
        assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest
    pg_env = {**api.BASE_ENV, 'PGHOST': str(api.SOCKET), 'PGPORT': '55681', 'PGUSER': api.ROLE, 'PGPASSWORD': runtime['db_password'], 'PGDATABASE': api.DATABASE, 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null', 'PGCONNECT_TIMEOUT': '3', 'PGOPTIONS': '-c statement_timeout=5000'}

    def role_sql(query, database=None):
        env = {**pg_env, 'PGDATABASE': database or api.DATABASE}
        return subprocess.run([api.PG + 'psql', '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1', '-c', query], env=env, capture_output=True, text=True, timeout=8)

    def snapshot():
        tables = json.loads(api.sql("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='public'"))
        result = {}
        for table in tables:
            assert table.replace('_', '').isalnum()
            result[table] = api.sql('SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),\'[]\') FROM public."' + table + '" t')
        return hashlib.sha256(json.dumps(result, sort_keys=True).encode()).hexdigest()

    rows_before = snapshot()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def request(path, token=None, method='GET', body=None, extra=None):
        headers = dict(extra or {})
        if token:
            headers['Authorization'] = 'Bearer ' + token
        if body is not None:
            headers['Content-Type'] = 'application/json'
        req = urllib.request.Request('http://127.0.0.1:55682' + path, headers=headers, method=method, data=None if body is None else json.dumps(body).encode())
        try:
            with opener.open(req, timeout=4) as response:
                return response.status, response.read(128001)
        except urllib.error.HTTPError as error:
            return error.code, error.read(128001)

    checks = []
    agent = plan['agent_id']
    path = '/agents/' + agent + '/live-work'
    assert request('/health')[0] == 401
    status, body = request('/health', runtime['service_token'])
    assert status == 200 and json.loads(body)['epoch'] is None
    status, body = request(path, runtime['owner_token'])
    view = json.loads(body)
    assert status == 200 and view['enabled'] is True and view['continuous'] is False
    assert view['project_id'] == plan['project_id'] and view['policy']['model'] == plan['model']
    assert view['policy']['budget_id'] == plan['budget_id'] and view['policy']['budget_limit_microusd'] == 5000
    assert view['minimum_reservation_microusd'] == 3536 and not view['sessions']
    assert view['policy']['input_token_bound'] == 24192 and view['policy']['max_output_tokens'] == 1024
    assert len(view['jobs']) == 1 and view['jobs'][0]['id'] == plan['job_id'] and view['jobs'][0]['input_sha256'] == plan['input_sha256']
    checks.append('real_adapter_exact_project_model_budget_visible_without_request')
    status, body = request('/jobs/' + plan['job_id'], runtime['owner_token'])
    saved = json.loads(body)
    assert status == 200, 'Exact execution job read failed'
    assert saved['job']['id'] == plan['job_id'] and saved['job']['state'] == 'created'
    assert saved['job']['received_atoms'] == 0 and saved['job']['earned_atoms'] == 0
    assert saved['job']['input'] == plan['preflight']['task_input']
    assert saved['payment_request']['recipient'] is None
    assert request('/jobs/' + plan['job_id'], runtime['service_token'])[0] == 403
    checks.append('exact_unpaid_job_read_succeeds_without_wallet_update_privilege')
    assert request(path, runtime['service_token'])[0] == 403
    assert request(path, runtime['service_token'], extra={'X-User-Id': plan['preflight']['test_owner_id'], 'X-Role': 'owner'})[0] == 403
    assert request('/agents/00000000-0000-4000-8000-000000000001/live-work', runtime['owner_token'])[0] == 403
    checks.append('public_identity_spoofing_and_other_agent_routes_denied')
    # Invalid/non-consenting bodies cannot reach generation even if a lock regresses.
    for method in ['POST', 'PUT', 'PATCH', 'DELETE']:
        assert request(path, runtime['owner_token'], method, {'consent': {'authorize_paid': False}})[0] == 403
    for target in ['/agents', '/agents/' + agent + '/start', '/agents/' + agent + '/tick', '/agents/' + agent + '/live-inference']:
        assert request(target, runtime['owner_token'], 'POST', {})[0] == 403
    checks.append('dispatch_lock_and_all_other_mutation_routes_denied')
    identity = role_sql("SELECT current_user,rolsuper,rolcreatedb,rolcreaterole FROM pg_roles WHERE rolname=current_user")
    assert identity.returncode == 0 and identity.stdout.strip() == api.ROLE + '|f|f|f'
    wallet_write = role_sql("SELECT has_table_privilege(current_user,'wallet_identities','UPDATE') OR has_any_column_privilege(current_user,'wallet_identities','UPDATE')")
    assert wallet_write.returncode == 0 and wallet_write.stdout.strip() == 'f'
    until = role_sql("SELECT extract(epoch FROM rolvaliduntil)::bigint FROM pg_roles WHERE rolname=current_user")
    assert until.returncode == 0 and int(until.stdout.strip()) == runtime['deadline']
    create = role_sql("SELECT has_schema_privilege(current_user,'public','CREATE')")
    assert create.returncode == 0 and create.stdout.strip() == 'f'
    other = role_sql('SELECT current_database()', 'postgres')
    assert other.returncode != 0
    for query in ["UPDATE agents SET data=data WHERE false", "UPDATE economic_jobs SET input=input WHERE false", "UPDATE live_budgets SET limit_microusd=limit_microusd WHERE false", "DELETE FROM live_budgets WHERE false", "SELECT pg_read_file('/home/pikoo/Desktop/supa-agent/postgres-data/infrastructure-v1/data/PG_VERSION')"]:
        assert role_sql(query).returncode != 0
    locks = role_sql('BEGIN; SELECT id FROM agents FOR UPDATE; SELECT id FROM economic_jobs FOR UPDATE; SELECT id FROM live_budgets FOR UPDATE; ROLLBACK;')
    assert locks.returncode == 0
    checks.append('restricted_role_cannot_change_inputs_budgets_or_read_files_or_connect_elsewhere')
    # Test the same mount layout as the running API, without credentials.
    for hidden in [ROOT / 'postgres-data', ROOT / '.env.eval', ROOT / '.env.claude', api.DATA, '/tmp/supa-claude-pg-3f722ca85789/s']:
        result = subprocess.run(api.sandbox(['/usr/bin/test', '!', '-e', str(hidden)]), env=api.BASE_ENV, capture_output=True, timeout=5)
        assert result.returncode == 0
    checks.append('filesystem_hides_primary_data_private_files_and_other_cluster_sockets')
    # Exercise the real runner's complete preparation, not a parallel approximation.
    # GETs reach the locked API. Its final POST is intercepted in Python BEFORE
    # network dispatch; any journal is synthetic and lives in a temporary directory.
    spec = importlib.util.spec_from_file_location('runner', ROOT / 'scripts/live-work-evaluation.py')
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    forwarded_gets = []
    blocked_posts = []
    real_run = subprocess.run

    class BoundaryOnly:
        def open(self, req, timeout):
            assert req.full_url.startswith(plan['api_origin'] + '/')
            if req.data is not None:
                assert req.full_url == plan['api_origin'] + path and req.get_method() == 'POST'
                consent = json.loads(req.data)
                assert consent['job_id'] == plan['job_id'] and consent['input_sha256'] == plan['input_sha256']
                assert consent['consent']['request_id'] == plan['preflight']['request_id']
                assert consent['consent']['max_requests'] == 1 and consent['consent']['max_cost_microusd'] == 4000
                blocked_posts.append(True)
                raise RuntimeError('synthetic_boundary_no_network_dispatch')
            assert req.get_method() == 'GET'
            forwarded_gets.append(req.full_url)
            return opener.open(req, timeout=timeout)

    with tempfile.TemporaryDirectory(prefix='supa-runner-boundary-') as temporary:
        def preflight(command, **kwargs):
            assert command == ['node', str(Path(temporary) / 'scripts/live-work-preflight.mjs'), '--plan-stdin', '--check-database', '--for-execution']
            return real_run(['node', str(ROOT / 'scripts/live-work-preflight.mjs'), *command[2:]], **kwargs)
        with patch.object(runner, 'ROOT', Path(temporary)), patch.dict(os.environ, {'SUPA_WORK_OPERATOR_TOKEN': runtime['owner_token']}), patch.object(runner.subprocess, 'run', side_effect=preflight), patch.object(runner.urllib.request, 'build_opener', return_value=BoundaryOnly()):
            try:
                runner.run(plan, api.SCOPE, plan['model'], 4000)
            except ValueError as error:
                assert str(error) == 'outcome_unknown_review_saved_session_no_retry' and blocked_posts == [True]
            else:
                raise AssertionError('Runner must stop at the intercepted dispatch boundary')
    assert forwarded_gets == [plan['api_origin'] + '/health', plan['api_origin'] + path, plan['api_origin'] + '/jobs/' + plan['job_id']]
    assert not (ROOT / 'artifacts/milestone-10/live-requests' / (api.SCOPE + '.jsonl')).exists()
    checks.append('real_runner_all_gets_pass_final_post_intercepted_before_network')
    checked = subprocess.run(['node', str(ROOT / 'scripts/live-work-preflight.mjs'), '--scope', str(api.PLAN), '--check-database', '--for-execution'], env={**api.BASE_ENV, 'PATH': os.environ['PATH']}, capture_output=True, text=True, timeout=12)
    assert checked.returncode == 0
    db = json.loads(checked.stdout)
    assert db['scope_sha256'] == api.SCOPE and db['database']['budget_state'] == 'installed_zero_spent'
    remaining = float(api.sql("SELECT extract(epoch FROM (expires_at-clock_timestamp())) FROM economic_jobs WHERE id='5cdfe440-dc36-4f94-8fc3-93b766ea31c5'"))
    assert remaining > 300
    assert snapshot() == rows_before and api.protected() == before
    checks.append('frozen_scope_expiry_all_public_rows_and_protected_hashes_preserved')
    report = {'passed': True, 'checks': checks, 'scope_sha256': api.SCOPE, 'paid_dispatch_authorized': False, 'api_running': True, 'database': db['database'], 'remaining_seconds': remaining, 'credential_expires_unix': runtime['deadline'], 'external_provider_requests': 0, 'paid_requests': 0, 'primary_started': False, 'public_table_fingerprint': rows_before}
    (api.OUT / 'scoped-api-validation.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
