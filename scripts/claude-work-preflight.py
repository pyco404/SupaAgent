#!/usr/bin/env python3
"""Network-free Claude scope and optional read-only disposable database verification."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / 'config/claude-work-evaluation-plan.json'


def require(condition):
    if not condition:
        raise ValueError('claude_scope_or_database_mismatch')


def inspect(plan, check_database=False):
    spec = importlib.util.spec_from_file_location('work', ROOT / 'scripts/live-work-evaluation.py')
    work = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(work)
    report, fixture = work.inspect(plan)
    p = plan['preflight']
    db = p['database']
    require(plan.get('provider') == 'anthropic')
    require(re.fullmatch(r'supa_claude_[0-9a-f]{12}_test', db['name']))
    require(re.fullmatch(r'/tmp/supa-claude-pg-[0-9a-f]{12}/data', db['data_directory']))
    require(db['socket_directory'] == db['data_directory'][:-4] + 's' and db['port'] == 55683)
    require(not (ROOT / 'postgres-data/infrastructure-v1/data/postmaster.pid').exists())
    require(plan['budget_id'] not in ('ad91e475-f1da-495a-8247-139643d2d8b1', 'e6076529-e302-403e-b846-1ff902c1f641'))
    require(work.digest({'plan': plan, 'fixture': fixture}) == report['scope_sha256'])
    require(hashlib.sha256(p['task_input'].encode()).hexdigest() == plan['input_sha256'])
    require(json.loads(p['task_input']) == {k: fixture[k] for k in ['objective', 'material', 'requirements']})
    require(p['approved_tools'] == ['inspect_input', 'extract_evidence', 'check_requirements'] and p['memory_policy'] == 'none')
    report.update(provider='anthropic', workspace_verified=False, account_credit_verified=False)
    if check_database:
        data = Path(db['data_directory'])
        for directory in (data, data.parent, Path(db['socket_directory'])):
            s = directory.lstat()
            require(directory.is_dir() and not directory.is_symlink() and s.st_uid == os.geteuid() and not s.st_mode & 0o077)
        env = {'PATH': '/usr/bin:/bin', 'PGHOST': db['socket_directory'], 'PGPORT': str(db['port']), 'PGDATABASE': db['name'], 'PGUSER': 'pikoo', 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null', 'PGCONNECT_TIMEOUT': '3', 'PGOPTIONS': '-c default_transaction_read_only=on -c statement_timeout=5000'}
        def sql(query):
            return subprocess.check_output(['/usr/lib/postgresql/16/bin/psql', '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1', '-c', query], env=env, text=True, stderr=subprocess.PIPE, timeout=8).strip()
        identity = json.loads(sql("SELECT json_build_array(current_database(),current_setting('data_directory'),system_identifier::text,current_setting('listen_addresses')) FROM pg_control_system()"))
        require(identity == [db['name'], db['data_directory'], db['system_identifier'], ''])
        # Exact UUIDs only; no frontend identity or SQL fragments are accepted.
        import uuid
        for key in ['agent_id', 'job_id', 'budget_id']:
            require(str(uuid.UUID(plan[key])) == plan[key])
        owner = str(uuid.UUID(p['test_owner_id']))
        agent, job, budget = (plan[k] for k in ['agent_id', 'job_id', 'budget_id'])
        require(sql('SELECT count(*) FROM application_epoch') == '0')
        require(sql('SELECT count(*) FROM agents') == '1' and sql('SELECT count(*) FROM economic_jobs') == '1')
        require(sql('SELECT count(*) FROM agent_access') == '1')
        require(sql(f"SELECT principal_id::text FROM agent_access WHERE agent_id='{agent}' AND role='owner'") == owner)
        require(sql(f"SELECT data->>'lifecycle' FROM agents WHERE id='{agent}'") == 'paused')
        saved = json.loads(sql(f"SELECT json_build_object('input',input,'state',state,'network',network,'agent',agent_id,'customer',customer_id,'revision',revision,'received',received_atoms,'earned',earned_atoms,'expires',to_char(expires_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"'),'remaining',extract(epoch FROM (expires_at-clock_timestamp()))) FROM economic_jobs WHERE id='{job}'"))
        require(saved['input'] == p['task_input'] and saved['state'] == 'created' and saved['network'] == 'simulated' and saved['agent'] == agent and saved['customer'] == owner)
        require(saved['revision'] == saved['received'] == saved['earned'] == 0 and saved['expires'] == p['job_expires_at'] and saved['remaining'] > 60)
        require(sql('SELECT count(*) FROM live_budgets') == '1')
        require(json.loads(sql(f"SELECT json_build_array(limit_microusd,committed_microusd) FROM live_budgets WHERE id='{budget}'")) == [5000, 0])
        for table in ['live_sessions', 'inference_calls', 'cycles', 'actions', 'schedule_authorizations', 'economic_entries']:
            require(sql('SELECT count(*) FROM ' + table) == '0')
        migrations = json.loads(sql("SELECT json_agg(json_build_object('version',version,'checksum',encode(checksum,'hex'),'success',success) ORDER BY version) FROM _sqlx_migrations"))
        require(len(migrations) == len(list((ROOT / 'migrations').glob('*.sql'))))
        for row in migrations:
            file, = (ROOT / 'migrations').glob(f"{row['version']:04d}_*.sql")
            require(row['success'] and hashlib.sha384(file.read_bytes()).hexdigest() == row['checksum'])
        report['database'] = {'verified': True, 'budget_state': 'installed_zero_spent', 'remaining_seconds': saved['remaining'], 'migrations_verified': len(migrations)}
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan-stdin', action='store_true')
    parser.add_argument('--check-database', action='store_true')
    args = parser.parse_args()
    plan = json.load(sys.stdin) if args.plan_stdin else json.loads(PLAN.read_text())
    print(json.dumps(inspect(plan, args.check_database), indent=2))


if __name__ == '__main__':
    try:
        main()
    except Exception:
        print(json.dumps({'error': 'claude_preflight_failed_closed', 'paid_requests': 0}), file=sys.stderr)
        sys.exit(1)
