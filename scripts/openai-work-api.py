#!/usr/bin/env python3
"""Pinned disposable OpenAI API. Startup never calls a provider; paid dispatch stays locked by default."""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import secrets
import shlex
import signal
import stat
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
PLAN = ROOT / 'config/openai-work-evaluation-current.json'
SCOPE = '0453528bc228c83e6137eabd53ebeb4a7b98a198e7e6937e397ece240a51a6f0'
DATA = Path('/tmp/supa-m10a-pg-a13db38ce0a5/data')
SOCKET = DATA.parent / 's'
DATABASE = 'supa_m10a_a13db38ce0a5_test'
ROLE = 'supa_openai_eval'
SYSTEM_IDENTIFIER = '7703509812952711377'
JOB_ID = '5cdfe440-dc36-4f94-8fc3-93b766ea31c5'
PG = '/usr/lib/postgresql/16/bin/'
OUT = ROOT / 'artifacts/openai-final-readiness'
PRIVATE = OUT / 'runtime.json'
BINARY = ROOT / 'target/claude-validation/debug/examples/openai_work_api'
BASE_ENV = {'PATH': '/usr/bin:/bin', 'HOME': str(Path.home()), 'LANG': 'C.UTF-8'}


def require(ok, code):
    if not ok:
        raise ValueError(code)


def protected():
    spec = importlib.util.spec_from_file_location('guard', ROOT / 'scripts/validate-stage-two-migrations.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.protected()


def frozen():
    require(not (ROOT / 'postgres-data/infrastructure-v1/data/postmaster.pid').exists(), 'primary_must_remain_stopped')
    plan = json.loads(PLAN.read_text())
    spec = importlib.util.spec_from_file_location('evaluation', ROOT / 'scripts/live-work-evaluation.py')
    work = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(work)
    report, _ = work.inspect(plan)
    require(report['ready'] and report['scope_sha256'] == SCOPE, 'frozen_scope_changed')
    require(plan['preflight']['database']['name'] == DATABASE and plan['preflight']['database']['data_directory'] == str(DATA), 'wrong_disposable_target')
    require(plan['job_id'] == JOB_ID and plan['preflight']['database']['system_identifier'] == SYSTEM_IDENTIFIER, 'wrong_disposable_job_or_cluster')
    return plan


def private_json():
    st = PRIVATE.lstat()
    require(stat.S_ISREG(st.st_mode) and st.st_uid == os.getuid() and not st.st_mode & 0o077 and st.st_nlink == 1, 'unsafe_runtime_file')
    return json.loads(PRIVATE.read_text())


def credentials():
    path = ROOT / '.env.eval'
    st = path.lstat()
    require(stat.S_ISREG(st.st_mode) and st.st_uid == os.getuid() and not st.st_mode & 0o077 and st.st_nlink == 1, 'unsafe_private_environment')
    result = {}
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW)) as stream:
        opened = os.fstat(stream.fileno())
        require((opened.st_dev, opened.st_ino) == (st.st_dev, st.st_ino), 'private_file_changed_during_open')
        for line in stream:
            key, sep, value = line.strip().partition('=')
            if sep and key in ('OPENAI_API_KEY', 'OPENAI_MODEL', 'OPENAI_PROJECT_ID'):
                require(key not in result, 'duplicate_private_setting')
                values = shlex.split(value, comments=True)
                require(len(values) == 1, 'invalid_private_setting')
                result[key] = values[0]
    require(result.get('OPENAI_PROJECT_ID') == 'proj_npsyg93Ofk7ivKheGa8IDw5q' and result.get('OPENAI_MODEL') == 'gpt-6-luna', 'project_model_mismatch')
    key = result.get('OPENAI_API_KEY', '')
    require(key.startswith('sk-') and key.isascii() and all(33 <= ord(c) <= 126 for c in key), 'missing_private_application_key')
    return result


def pg(command, *args):
    result = subprocess.run([PG + command, *map(str, args)], env=BASE_ENV, capture_output=True, text=True, timeout=40)
    require(result.returncode == 0, 'disposable_postgres_' + command + '_failed')
    return result.stdout.strip()


def sql(query, database=None):
    database = DATABASE if database is None else database
    env = {**BASE_ENV, 'PGHOST': str(SOCKET), 'PGPORT': '55681', 'PGUSER': 'pikoo', 'PGDATABASE': database, 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null', 'PGOPTIONS': '-c statement_timeout=5000 -c log_statement=none -c log_min_error_statement=panic'}
    result = subprocess.run([PG + 'psql', '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1'], input=query, env=env, capture_output=True, text=True, timeout=10)
    require(result.returncode == 0, 'disposable_sql_failed_no_details')
    return result.stdout.strip()


def sandbox(command=None):
    args = ['/usr/bin/bwrap', '--unshare-user', '--unshare-pid', '--as-pid-1', '--new-session', '--cap-drop', 'ALL', '--ro-bind', '/usr', '/usr', '--ro-bind', '/lib', '/lib', '--ro-bind', '/lib64', '/lib64', '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--dir', '/app', '--ro-bind', str(BINARY), '/app/api', '--ro-bind', str(SOCKET), str(SOCKET), '--chdir', '/app']
    for path in ['/etc/resolv.conf', '/etc/hosts', '/etc/nsswitch.conf', '/etc/ssl/certs']:
        if Path(path).exists():
            args += ['--ro-bind', path, path]
    return args + (command or ['/app/api'])


def child_identity(pid):
    process = Path('/proc') / str(pid)
    require(process.stat().st_uid == os.getuid(), 'wrong_process_owner')
    command = (process / 'cmdline').read_bytes().split(b'\0')[0]
    mounts = (process / 'mountinfo').read_text()
    require(command == b'/app/api' and str(BINARY) in mounts and str(SOCKET) in mounts, 'wrong_sandbox_process')
    return (process / 'stat').read_text().split()[21]


def process_matches(runtime):
    try:
        pid = int(runtime['child_pid'])
        require(pid > 1, 'unsafe_pid')
        return child_identity(pid) == runtime['child_start_ticks']
    except (OSError, ValueError, KeyError):
        return False


def stop_api():
    if not PRIVATE.exists():
        return
    runtime = private_json()
    if process_matches(runtime):
        os.kill(int(runtime['child_pid']), signal.SIGTERM)
        for _ in range(50):
            if not process_matches(runtime):
                break
            time.sleep(.1)
        require(not process_matches(runtime), 'api_shutdown_requires_review')


def start(paid=False):
    os.chdir(ROOT)
    os.umask(0o077)
    OUT.mkdir(mode=0o700, parents=True, exist_ok=True)
    info = OUT.lstat()
    require(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid(), 'unsafe_evidence_directory')
    OUT.chmod(0o700)
    before = protected()
    plan = frozen()
    require(BINARY.is_file(), 'build_scoped_api_example_first')
    if PRIVATE.exists():
        require(not process_matches(private_json()), 'scoped_api_already_running')
    started = False
    try:
        if not (DATA / 'postmaster.pid').exists():
            pg('pg_ctl', '-D', DATA, '-l', DATA.parent / 'server.log', '-w', 'start')
            started = True
        checked = subprocess.run(['node', str(ROOT / 'scripts/live-work-preflight.mjs'), '--scope', str(PLAN), '--check-database', '--for-execution'], env={**BASE_ENV, 'PATH': os.environ['PATH']}, capture_output=True, text=True, timeout=12)
        require(checked.returncode == 0, 'disposable_scope_or_expiration_failed')
        require(json.loads(checked.stdout)['scope_sha256'] == SCOPE, 'scope_digest_mismatch')
        # Read-only target identity before role setup; no primary connection is ever constructed.
        require(json.loads(sql("SELECT json_build_array(current_database(),current_setting('data_directory'),system_identifier::text) FROM pg_control_system()")) == [DATABASE, str(DATA), SYSTEM_IDENTIFIER], 'cluster_identity_mismatch')
        remaining = float(sql(f"SELECT extract(epoch FROM (expires_at-clock_timestamp())) FROM economic_jobs WHERE id='{JOB_ID}'"))
        require(remaining > 300, 'job_expired_or_insufficient_validity_no_renewal')
        deadline = int(time.time()) + min(3500, int(remaining) - 30)
        valid_until = datetime.fromtimestamp(deadline, timezone.utc).isoformat()
        password = secrets.token_hex(32)
        exists = sql(f"SELECT count(*) FROM pg_roles WHERE rolname='{ROLE}'") == '1'
        operation = 'ALTER' if exists else 'CREATE'
        sql(f"BEGIN; {operation} ROLE {ROLE} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{password}' VALID UNTIL '{valid_until}'; GRANT CONNECT ON DATABASE {DATABASE} TO {ROLE}; GRANT pg_read_all_settings TO {ROLE}; GRANT EXECUTE ON FUNCTION pg_control_system() TO {ROLE}; GRANT USAGE ON SCHEMA public TO {ROLE}; GRANT SELECT ON ALL TABLES IN SCHEMA public TO {ROLE}; GRANT UPDATE(id) ON agents,economic_jobs TO {ROLE}; GRANT UPDATE(committed_microusd) ON live_budgets TO {ROLE}; GRANT INSERT,UPDATE ON live_sessions TO {ROLE}; GRANT INSERT ON live_session_audit TO {ROLE}; GRANT USAGE ON SEQUENCE live_session_audit_id_seq TO {ROLE}; COMMIT;")
        # Only this database accepts this disposable credential; peer admin remains local.
        hba = DATA / 'pg_hba.conf'
        lines = [line for line in hba.read_text().splitlines() if ROLE not in line]
        hba.write_text(f'local {DATABASE} {ROLE} scram-sha-256\nlocal all {ROLE} reject\n' + '\n'.join(lines) + '\n')
        pg('pg_ctl', '-D', DATA, '-w', 'reload')
        runtime = {'scope_sha256': SCOPE, 'owner_token': secrets.token_hex(32), 'service_token': secrets.token_hex(32), 'db_password': password, 'deadline': deadline, 'paid_dispatch_authorized': paid}
        env = {**BASE_ENV, **credentials(), 'SUPA_EVALUATION_SANDBOX': SCOPE, 'PGPASSWORD': password, 'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null', 'SUPA_WORK_OPERATOR_TOKEN': runtime['owner_token'], 'SUPA_EVALUATION_SERVICE_TOKEN': runtime['service_token'], 'SUPA_EVALUATION_CREDENTIAL_DEADLINE': str(runtime['deadline']), 'SUPA_ALLOW_OPENAI': 'true', 'SUPA_CONTROLLED_INFERENCE': 'true', 'SUPA_LIVE_BUDGET_ID': plan['budget_id'], 'SUPA_LIVE_BUDGET_MICROUSD': '5000', 'SUPA_MAX_OUTPUT_TOKENS': '1024'}
        # The actual API filesystem contains no home directory, repository, primary cluster or other Unix sockets.
        proof = subprocess.run(sandbox(['/usr/bin/test', '!', '-e', str(ROOT / 'postgres-data')]), env=BASE_ENV, capture_output=True, timeout=5)
        require(proof.returncode == 0, 'filesystem_isolation_failed')
        command = sandbox(['/app/api'] + (['--authorize-one-paid-scope', SCOPE] if paid else []))
        reader, writer = os.pipe()
        try:
            with (OUT / 'api.log').open('ab') as log:
                child = subprocess.Popen(command[:1] + ['--info-fd', str(writer)] + command[1:], env=env, stdout=log, stderr=log, start_new_session=True, pass_fds=(writer,))
            os.close(writer)
            writer = None
            # Bubblewrap's separate control pipe reports a host PID; it carries no environment or credentials.
            import select
            require(select.select([reader], [], [], 5)[0], 'sandbox_child_identity_timeout')
            info = json.loads(os.read(reader, 4096))
            runtime['pid'] = child.pid
            runtime['child_pid'] = int(info['child-pid'])
            for _ in range(50):
                try:
                    runtime['child_start_ticks'] = child_identity(runtime['child_pid'])
                    break
                except (OSError, ValueError):
                    require(child.poll() is None, 'scoped_api_startup_failed')
                    time.sleep(.02)
            else:
                raise ValueError('sandbox_child_identity_unverified')
        finally:
            os.close(reader)
            if writer is not None:
                os.close(writer)
        fd = os.open(PRIVATE, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'w') as stream:
            json.dump(runtime, stream)
            stream.flush()
            os.fsync(stream.fileno())
        time.sleep(.4)
        require(child.poll() is None, 'scoped_api_startup_failed')
        require(protected() == before, 'protected_primary_integrity_mismatch')
        print(json.dumps({'api_started': True, 'paid_dispatch_authorized': paid, 'scope_sha256': SCOPE, 'credential_expires_unix': runtime['deadline'], 'external_requests': 0}))
    except Exception:
        stop_api()
        if started:
            pg('pg_ctl', '-D', DATA, '-m', 'fast', '-w', 'stop')
        raise


def execute(scope, authorized):
    require(authorized and scope == SCOPE, 'fresh_exact_paid_authorization_required')
    frozen()
    journal = ROOT / 'artifacts/milestone-10/live-requests' / (SCOPE + '.jsonl')
    require(not journal.exists(), 'existing_attempt_requires_review_no_retry')
    stop_api()
    start(paid=True)
    runtime = private_json()
    try:
        command = ['python3', str(ROOT / 'scripts/live-work-evaluation.py'), '--plan', str(PLAN), '--run', '--authorize-paid', '--scope-sha256', SCOPE, '--model', 'gpt-6-luna', '--max-cost-microusd', '4000']
        result = subprocess.run(command, cwd=ROOT, env={**BASE_ENV, 'PATH': os.environ['PATH'], 'SUPA_WORK_OPERATOR_TOKEN': runtime['owner_token']}, timeout=90)
        require(result.returncode == 0, 'evaluation_failed_no_automatic_retry')
    finally:
        stop_api()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['start', 'stop', 'execute'])
    parser.add_argument('--scope-sha256')
    parser.add_argument('--authorize-paid', action='store_true')
    args = parser.parse_args()
    if args.action == 'execute':
        execute(args.scope_sha256, args.authorize_paid)
    else:
        require(not args.authorize_paid and args.scope_sha256 is None, 'paid_flags_forbidden_for_preparation')
        if args.action == 'start':
            start()
        else:
            stop_api()


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print(json.dumps({'error': str(error) if isinstance(error, ValueError) else 'scoped_api_operation_failed_no_private_details'}), file=sys.stderr)
        raise SystemExit(1)
