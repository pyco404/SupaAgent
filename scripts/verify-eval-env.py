#!/usr/bin/env python3
"""Test private file loading using --check only. Never authorize a paid run."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
binary = root / 'target/debug/examples/live_eval'
base = {k: v for k, v in os.environ.items() if not k.startswith(('SUPA_', 'OPENAI_'))}
results = []
with tempfile.TemporaryDirectory(prefix='supa-eval-env-') as directory:
    scratch = Path(directory)
    config = scratch / '.env.eval'
    config.write_text('SUPA_EVAL_DATABASE_URL=postgresql://supa_eval@127.0.0.1:55432/supa_eval\nOPENAI_API_KEY=sk-fake-private-file-test\nOPENAI_MODEL=gpt-6-luna\nSUPA_ALLOW_OPENAI=false\n')
    config.chmod(0o600)

    def check(extra=None):
        env = dict(base, SUPA_EVAL_ENV_FILE=str(config))
        env.update(extra or {})
        run = subprocess.run([str(binary), '--check'], cwd=scratch, env=env,
                             capture_output=True, text=True, timeout=15)
        assert 'sk-fake' not in run.stdout + run.stderr
        return run

    run = check()
    assert run.returncode == 0
    data = json.loads(run.stdout)
    assert data['evaluation_database_configured'] is True
    assert data['openai_api_key_detected'] == 'yes'
    assert data['ready_for_paid'] is False and data['paid_calls_made'] == 0
    results.append('private file loaded from a fresh process without sourcing')

    run = check({'OPENAI_MODEL': 'gpt-6.1-sol'})
    assert json.loads(run.stdout)['routine_model'] == 'gpt-6.1-sol'
    results.append('explicit exported environment takes precedence')

    config.chmod(0o644)
    run = check()
    assert run.returncode == 1 and json.loads(run.stderr)['error'] == 'permission_denied'
    config.chmod(0o600)
    results.append('file readable by other users rejected')

    config.write_text('OPENAI_API_KEY="sk-fake-unclosed-value\n')
    run = check()
    assert run.returncode == 1 and 'sk-fake' not in run.stderr
    results.append('malformed secret input rejected without echoing values')

    config.write_text('PATH=/untrusted\n')
    assert check().returncode == 1
    results.append('unrelated environment keys rejected')

    config.unlink()
    config.symlink_to(root / '.env.example')
    assert check().returncode == 1
    results.append('symbolic links rejected')
    config.unlink()

    # Audit repository ignore rules without touching the unusable real .git metadata.
    subprocess.run(['git', 'init', '--quiet', str(scratch)], check=True, capture_output=True)
    shutil.copyfile(root / '.gitignore', scratch / '.gitignore')
    assert subprocess.run(['git', '-C', str(scratch), 'check-ignore', '--quiet', '.env.eval']).returncode == 0
    assert subprocess.run(['git', '-C', str(scratch), 'check-ignore', '--quiet', '.env.example']).returncode == 1
    results.append('private file ignored; public example remains trackable')

print(json.dumps({'environment_loading_checks_passed': len(results), 'checks': results, 'paid_calls': 0}))
