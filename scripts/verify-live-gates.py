#!/usr/bin/env python3
"""Offline subprocess checks. Never invoke a fully authorized --run configuration."""
import json
import os
from pathlib import Path
import subprocess

binary = 'target/debug/examples/live_eval'
keys = ['OPENAI_API_KEY', 'SUPA_ALLOW_OPENAI', 'SUPA_LIVE_BUDGET_ID',
        'SUPA_LIVE_BUDGET_MICROUSD', 'SUPA_EVAL_DATABASE_URL', 'OPENAI_MODEL',
        'OPENAI_COMPLEX_MODEL', 'SUPA_PLANNING_MODE', 'SUPA_MAX_OUTPUT_TOKENS',
        'SUPA_INPUT_RATE_MICROUSD_PER_MILLION', 'SUPA_CACHED_RATE_MICROUSD_PER_MILLION',
        'SUPA_CACHE_WRITE_RATE_MICROUSD_PER_MILLION', 'SUPA_OUTPUT_RATE_MICROUSD_PER_MILLION']
base = {k: v for k, v in os.environ.items() if k not in keys}
# Isolate subprocess safeguards from the operator's private persisted configuration.
base['SUPA_EVAL_ENV_FILE'] = ''
fake = dict(base, OPENAI_API_KEY='sk-fake-offline-gate-test', SUPA_ALLOW_OPENAI='true',
            SUPA_LIVE_BUDGET_ID='760892da-bd50-450e-a9f2-e8981d232754',
            SUPA_LIVE_BUDGET_MICROUSD='100000',
            SUPA_EVAL_DATABASE_URL='postgresql://local@127.0.0.1:1/never_connect')
cases = [
    ('absent_configuration', ['--check'], base),
    ('credentials_do_not_enable_calls', ['--check'], fake),
    ('run_requires_explicit_authorization', ['--run'], fake),
    ('ceiling_required', ['--run', '--authorize-paid'], {k: v for k, v in fake.items() if k != 'SUPA_LIVE_BUDGET_MICROUSD'}),
    ('key_required', ['--run', '--authorize-paid'], {k: v for k, v in fake.items() if k != 'OPENAI_API_KEY'}),
    ('environment_opt_in_required', ['--run', '--authorize-paid'], dict(fake, SUPA_ALLOW_OPENAI='false')),
    ('underquoted_prices_rejected', ['--run', '--authorize-paid'], dict(fake, SUPA_INPUT_RATE_MICROUSD_PER_MILLION='1')),
    ('unsupported_model_redacted', ['--check'], dict(fake, OPENAI_MODEL='sk-invalid-model-secret')),
]
results = []
for name, args, env in cases:
    run = subprocess.run([binary, *args], env=env, capture_output=True, text=True, timeout=10)
    report = json.loads(run.stdout.splitlines()[0])
    assert report['ready_for_paid'] is False
    assert report['paid_calls_made'] == 0 and report['database_connectivity_checked'] is False
    assert 'sk-fake' not in run.stdout + run.stderr
    assert 'sk-invalid' not in run.stdout + run.stderr
    if '--run' in args:
        assert run.returncode == 1
        assert json.loads(run.stderr)['error'] == 'permission_denied'
    else:
        assert run.returncode == 0
    results.append({'case': name, 'passed': True, 'paid_calls': 0})
path = Path('artifacts/live-2a')
path.mkdir(parents=True, exist_ok=True)
(path / 'gate-tests.json').write_text(json.dumps(results, indent=2))
print(json.dumps({'offline_gate_cases_passed': len(results), 'paid_calls': 0}))
