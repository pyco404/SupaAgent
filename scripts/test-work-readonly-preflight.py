#!/usr/bin/env python3
"""Actual passive M10A HTTP checks; fixed disposable DB, no provider or private dotenv."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
DATA = Path('/tmp/supa-m10a-pg-0b780645f120/data')
SCOPE = 'cf2f80163a97e914725972664c33962cb615cb43a77693573ec759553fb2cef2'
PG = '/usr/lib/postgresql/16/bin/'


def main():
    os.chdir(ROOT)
    out = ROOT / 'artifacts/milestone-10a' / ('final-readiness-' + secrets.token_hex(6))
    out.mkdir(mode=0o700, parents=True)
    spec = importlib.util.spec_from_file_location('guard', ROOT / 'scripts/validate-stage-two-migrations.py')
    guard = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(guard)
    protected = guard.protected()
    assert not (DATA / 'postmaster.pid').exists()
    scope = json.loads(subprocess.check_output(['python3', 'scripts/live-work-evaluation.py', '--plan', 'config/live-work-evaluation-10a.json']))
    assert scope['scope_sha256'] == SCOPE and not scope['paid_execution_authorized']
    base_env = {'PATH': '/usr/bin:/bin', 'HOME': str(Path.home()), 'LANG': 'C.UTF-8'}
    pg_env = {**base_env, 'PGHOST': str(DATA.parent / 's'), 'PGPORT': '55681', 'PGDATABASE': 'supa_m10a_0b780645f120_test', 'PGUSER': 'pikoo', 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null', 'PGOPTIONS': '-c default_transaction_read_only=on -c statement_timeout=5000'}

    def sql(query):
        return subprocess.check_output([PG + 'psql', '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1', '-c', query], env=pg_env, text=True, stderr=subprocess.PIPE).strip()

    def fingerprint():
        tables = json.loads(sql("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='public'"))
        values = {}
        for table in tables:
            assert table.replace('_', '').isalnum()
            values[table] = sql('SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),\'[]\') FROM public."' + table + '" t')
        return hashlib.sha256(json.dumps(values, sort_keys=True).encode()).hexdigest()

    started = False
    api = None
    checks = []
    try:
        subprocess.run([PG + 'pg_ctl', '-D', str(DATA), '-l', str(DATA.parent / 'server.log'), '-w', 'start'], env=base_env, check=True, stdout=subprocess.DEVNULL)
        started = True
        before = fingerprint()
        db = json.loads(subprocess.check_output([shutil.which('node'), 'scripts/live-work-preflight.mjs', '--check-database', '--for-execution'], env=base_env))
        assert db['scope_sha256'] == SCOPE and db['budget_state'] == 'installed_zero_spent'
        remaining = float(sql("SELECT extract(epoch FROM (expires_at-clock_timestamp())) FROM economic_jobs WHERE id='ec1f95dc-cf3b-4907-83b4-624937802061'"))
        assert remaining > 300, 'Insufficient validity; do not extend scope'
        owner, public = secrets.token_hex(32), secrets.token_hex(32)
        # Short-lived test-only process credentials, never written or installed.
        with (out / 'passive-api.log').open('wb') as log:
            api = subprocess.Popen(['/tmp/supa-m9-target/debug/examples/work_readonly_preflight'], env={**base_env, 'SUPA_PREFLIGHT_TEST_TOKEN': owner, 'SUPA_PREFLIGHT_SERVICE_TOKEN': public}, stdout=log, stderr=log)
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

            def request(path, token=None, method='GET'):
                req = urllib.request.Request('http://127.0.0.1:55682' + path, headers={} if token is None else {'Authorization': 'Bearer ' + token}, method=method)
                try:
                    with opener.open(req, timeout=2) as response:
                        return response.status, response.read()
                except urllib.error.HTTPError as e:
                    return e.code, e.read()

            for _ in range(60):
                assert api.poll() is None, 'Passive API exited'
                try:
                    if request('/health', public)[0] == 200:
                        break
                except OSError:
                    pass
                time.sleep(.1)
            else:
                raise AssertionError('Passive API readiness timeout')
            assert request('/health')[0] == 401
            checks.append('unauthenticated_rejected')
            agent = 'f9a17921-f31f-4154-843d-1f348622a2f4'
            status, body = request('/agents', owner)
            assert status == 200 and any(a['id'] == agent and a['lifecycle'] == 'paused' for a in json.loads(body))
            checks.append('scoped_paused_agent_visible')
            status, body = request('/agents/' + agent + '/live-work', owner)
            value = json.loads(body)
            assert status == 200 and value['enabled'] is False and value['continuous'] is False and value['policy'] is None and value['sessions'] == []
            assert len(value['jobs']) == 1 and value['jobs'][0]['input_sha256'] == db['input_sha256']
            checks.append('exact_job_visible_live_provider_disabled')
            assert request('/agents/' + agent + '/live-work', public)[0] == 403
            checks.append('public_credential_cannot_read_private_work')
            for method in ['POST', 'PUT', 'PATCH', 'DELETE']:
                assert request('/agents/' + agent + '/live-work', owner, method)[0] == 403
            checks.append('all_mutating_methods_blocked')
            api.terminate()
            assert api.wait(timeout=10) == 0
            api = None
        assert fingerprint() == before
        checks.append('all_public_table_rows_unchanged')
        assert guard.protected() == protected
        checks.append('26_protected_hashes_unchanged')
        report = {'passed': True, 'scope_sha256': SCOPE, 'database': db['database'], 'checks': checks, 'remaining_seconds_at_check': remaining, 'project_billing_verified': False, 'openai_requests': 0, 'generation_requests': 0, 'runtime_credentials_installed': False, 'temporary_test_credentials_destroyed': True, 'primary_started': False}
    finally:
        if api is not None:
            api.terminate()
            api.wait(timeout=10)
        if started:
            subprocess.run([PG + 'pg_ctl', '-D', str(DATA), '-m', 'fast', '-w', 'stop'], env=base_env, check=True, stdout=subprocess.DEVNULL)
    assert guard.protected() == protected
    report['both_clusters_stopped'] = not (DATA / 'postmaster.pid').exists()
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({**report, 'evidence_directory': str(out)}, indent=2))


if __name__ == '__main__':
    main()
