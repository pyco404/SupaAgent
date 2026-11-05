#!/usr/bin/env python3
"""Test opt-in permanent token SQL in a NEW socket-only PostgreSQL 18 cluster.

No arbitrary URL, data directory, dotenv, network endpoint or Railway input is
accepted. Recovery mode resumes only a stopped socket-only cluster created here. All fixtures are synthetic public identifiers, never generated keys.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import uuid

ROOT = Path(__file__).resolve().parents[1]


def uid(n):
    return str(uuid.UUID(int=n))


def literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def resume_recovery(args):
    """Finish only the interrupted recovery phase; never reset existing fixtures."""
    run = args.resume_recovery
    assert len(run) == 32 and all(c in '0123456789abcdef' for c in run)
    work = Path('/tmp') / ('supa-token-test-' + run)
    out = ROOT / 'artifacts/agent-token-registry' / run
    data, socket = work / 'data', work / 'socket'
    assert work.resolve(strict=True) == work and data.resolve(strict=True) == data
    assert out.resolve(strict=True) == out and socket.resolve(strict=True) == socket
    assert work.stat().st_uid == os.getuid() and work.stat().st_mode & 0o077 == 0
    assert (data / 'PG_VERSION').read_text().strip() == '18'
    assert not (data / 'postmaster.pid').exists(), 'resume_requires_stopped_disposable'
    assert shutil.disk_usage(ROOT).free >= 16 * 1024 * 1024, 'recovery_requires_16MiB_free'
    # The prior run retained exact snapshots immediately before its failed restart.
    lines = (out / 'sql.log').read_text().splitlines()
    snapshots, finances = [], []
    for line in lines:
        try:
            value = json.loads(line)
        except (ValueError, TypeError):
            continue
        if isinstance(value, list) and value and all(isinstance(r, dict) and 'record_sha256' in r for r in value):
            snapshots.append(value)
        if isinstance(value, dict) and set(value) == {'balance','ledger','entries','holds','sessions','cycles'}:
            finances.append(value)
    assert len(snapshots) >= 2 and snapshots[-1] == snapshots[-2]
    assert finances and all(v == 0 for v in finances[0].values())
    expected, financial_before = snapshots[-1], finances[0]
    original_logs = {name: hashlib.sha256((out / name).read_bytes()).hexdigest()
                     for name in ('sql.log', 'commands.log', 'server.log')}
    pg = args.bin_dir.resolve(strict=True)
    env = {k: os.environ[k] for k in ('PATH','HOME')}
    env.update(LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
               LD_LIBRARY_PATH=str(args.library_dir.resolve(strict=True)),
               PGHOST=str(socket), PGPORT='55481', PGUSER='supa_token_test',
               PGDATABASE='supa_token_test', PGSERVICEFILE='/dev/null')
    primary = ROOT / 'postgres-data/infrastructure-v1/data'

    def protected():
        assert not (primary / 'postmaster.pid').exists()
        return {n: hashlib.sha256((primary / n).read_bytes()).hexdigest()
                for n in ('global/pg_control','postgresql.conf','pg_hba.conf')}

    before = protected()
    registered = json.loads(subprocess.check_output(['pg_lsclusters','--json']))
    checks = []

    def command(argv, statement=None, failure=None):
        result = subprocess.run([str(a) for a in argv], input=statement, env=env,
                                capture_output=True, text=True, timeout=90)
        with (out / 'recovery.log').open('a') as f:
            f.write(result.stdout + result.stderr)
        if failure:
            assert result.returncode != 0 and failure in result.stderr, result.stderr
        else:
            assert result.returncode == 0, result.stderr
        return result.stdout.strip()

    def sql(statement, failure=None):
        return command([pg/'psql','-X','-qAt','-v','ON_ERROR_STOP=1',
                        '--set=VERBOSITY=verbose'], statement, failure)

    def start():
        # Override networking explicitly, even during recovery of this test cluster.
        command([pg/'pg_ctl','-D',data,'-l',out/'recovery-server.log','-w','start',
                 '-o',f"-c listen_addresses='' -c unix_socket_directories='{socket}' -c port=55481"])
        identity = json.loads(sql("SELECT json_build_object('db',current_database(),'user',current_user,"
                                  "'data',current_setting('data_directory'),'listen',current_setting('listen_addresses'),"
                                  "'address',inet_server_addr());"))
        assert identity == dict(db='supa_token_test',user='supa_token_test',data=str(data),listen='',address=None)

    def snapshot():
        return json.loads(sql("SELECT coalesce(jsonb_agg(to_jsonb(b) ORDER BY id),'[]') FROM agent_tokens.bindings b;"))

    assert '(PostgreSQL) 18.' in command([pg/'postgres','--version'])
    try:
        start()
        assert snapshot() == expected
        checks.append('restart_preserves_exact_original_successor_and_concurrent_winner')
        command([pg/'pg_ctl','-D',data,'-m','immediate','-w','stop'])
        start()
        assert snapshot() == expected
        checks.append('crash_recovery_preserves_exact_committed_history')
        original = next(r for r in expected if r['id'] == uid(1000))
        expired = sql("SELECT expires_at<=clock_timestamp() FROM agent_tokens.receipts WHERE id=" + literal(original['id'])) == 't'
        sql("SET ROLE token_runtime; SELECT agent_tokens.confirm(" + literal(original['creator_id']) + ',' +
            literal(original['id']) + ',' + literal(json.dumps(original['record'])) + ');',
            'token_verification_receipt_invalid' if expired else '23505')
        checks.append('consumed_or_expired_receipt_denied_after_recovery')
        sql("SET ROLE token_runtime; UPDATE agent_tokens.bindings SET mint=mint;", '42501')
        checks.append('unauthorized_writes_denied_after_recovery')
        sql("UPDATE agent_tokens.bindings SET mint=mint;", 'token_history_immutable')
        checks.append('privileged_ordinary_writes_cannot_replace_history_after_recovery')
        after = json.loads(sql("SELECT jsonb_build_object('balance',(SELECT sum(simulated_balance_microusd) FROM agents),"
                              "'ledger',(SELECT count(*) FROM simulated_ledger),'entries',(SELECT count(*) FROM economic_entries),"
                              "'holds',(SELECT count(*) FROM economic_holds),'sessions',(SELECT count(*) FROM live_sessions),"
                              "'cycles',(SELECT count(*) FROM cycles));"))
        assert after == financial_before and snapshot() == expected
        checks.append('financial_baseline_and_original_snapshot_unchanged')
    finally:
        if (data / 'postmaster.pid').exists():
            command([pg/'pg_ctl','-D',data,'-m','fast','-w','stop'])
        assert protected() == before
        assert json.loads(subprocess.check_output(['pg_lsclusters','--json'])) == registered
    report = dict(passed=True, mode='resume_recovery_only', checks=checks,
                  original_logs_sha256=original_logs, disposable_data=str(data), stopped=True,
                  protected_primary_stopped_unchanged=True, registered_clusters_unchanged=True,
                  financial_records_unchanged=True, source_sha256={p:hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
                      for p in ('scripts/validate-agent-token-registry.py','deployment/agent-token-registry.sql')})
    (out/'recovery-validation.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(passed=True, checks=len(checks), evidence=str((out/'recovery-validation.json').relative_to(ROOT)))))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True, type=Path)
    parser.add_argument('--library-dir', required=True, type=Path)
    parser.add_argument('--resume-recovery', help="Finish recovery only for this runner's stopped 32-hex test run")
    args = parser.parse_args()
    if args.resume_recovery:
        return resume_recovery(args)
    assert shutil.disk_usage(ROOT).free >= 96 * 1024 * 1024, 'new_cluster_requires_96MiB_free'
    pg = args.bin_dir.resolve(strict=True)
    os.umask(0o077)
    run = uuid.uuid4().hex
    work = Path('/tmp') / ('supa-token-test-' + run)
    work.mkdir(mode=0o700)
    socket = work / 'socket'
    socket.mkdir(mode=0o700)
    out = ROOT / 'artifacts/agent-token-registry' / run
    out.mkdir(mode=0o700, parents=True)
    env = {k: os.environ[k] for k in ('PATH', 'HOME')}
    env.update(LANG='C.UTF-8', LC_ALL='C.UTF-8', TZ='UTC',
               LD_LIBRARY_PATH=str(args.library_dir.resolve(strict=True)),
               PGHOST=str(socket), PGPORT='55481', PGUSER='supa_token_test',
               PGDATABASE='supa_token_test', PGPASSFILE='/dev/null', PGSERVICEFILE='/dev/null')
    primary = ROOT / 'postgres-data/infrastructure-v1/data'

    def protected():
        assert not (primary / 'postmaster.pid').exists(), 'primary_must_remain_stopped'
        return {name: hashlib.sha256((primary / name).read_bytes()).hexdigest()
                for name in ('global/pg_control', 'postgresql.conf', 'pg_hba.conf')}

    before = protected()
    registered = json.loads(subprocess.check_output(['pg_lsclusters', '--json']))
    migration_hashes = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in sorted((ROOT / 'migrations').glob('*.sql'))}
    assert len(migration_hashes) == 13
    checks = []

    def command(argv):
        result = subprocess.run([str(a) for a in argv], env=env, cwd=ROOT,
                                capture_output=True, text=True, timeout=120)
        with (out / 'commands.log').open('a') as f:
            f.write(result.stdout + result.stderr)
        assert result.returncode == 0, result.stderr[-1200:]
        return result.stdout.strip()

    def sql(statement, role=None, fail=None):
        if role:
            statement = 'SET ROLE ' + role + ';\n' + statement
        result = subprocess.run([str(pg / 'psql'), '-X', '-qAt', '-v', 'ON_ERROR_STOP=1',
                                 '--set=VERBOSITY=verbose'], input=statement, env=env,
                                capture_output=True, text=True, timeout=45)
        with (out / 'sql.log').open('a') as f:
            f.write(result.stdout + result.stderr)
        if fail:
            assert result.returncode != 0 and fail in result.stderr, result.stderr
        else:
            assert result.returncode == 0, result.stderr
        return result.stdout.strip()

    def check(name, statement, role=None, fail=None):
        result = sql(statement, role, fail)
        checks.append(name)
        (out / 'checks-progress.json').write_text(json.dumps(dict(checks=checks), indent=2)+'\n')
        return result

    version = command([pg / 'postgres', '--version'])
    assert '(PostgreSQL) 18.' in version
    started = False
    now = datetime.now(timezone.utc) - timedelta(seconds=2)
    past = (now - timedelta(seconds=2)).isoformat()
    current = now.isoformat()
    future = (now + timedelta(minutes=10)).isoformat()
    owners = {i: uid(100 + i) for i in range(1, 5)}
    purposes, qualifications = {}, {}

    def fixture(i):
        agent, owner, purpose, review, policy = map(uid, (i, 100+i, 200+i, 300+i, 400+i))
        body = dict(id=purpose, agent_id=agent, creator_id=owner, revision=1,
                    created_at=past, objective='synthetic purpose', task_classes=['research'],
                    success_criteria=['synthetic'], exclusions=[], public_summary=None)
        q = dict(id=review, purpose=body, policy_id=policy, policy_version=1,
                 reviewer_id=owner, decision='qualified', evidence_ids=[uid(500+i)],
                 reviewed_at=past, expires_at=future)
        purposes[i], qualifications[i] = purpose, review
        sql(f"INSERT INTO agents(id,data) VALUES({literal(agent)}, '{{}}');"
            f"INSERT INTO agent_access(agent_id,principal_id,role,granted_by) VALUES"
            f"({literal(agent)},{literal(owner)},'owner',{literal(owner)});"
            f"INSERT INTO ecosystem_social.purposes VALUES({literal(purpose)},{literal(agent)},"
            f"{literal(owner)},1,{literal(json.dumps(body))});"
            f"INSERT INTO ecosystem_social.qualification_policies VALUES({literal(policy)},1,true,"
            f"ARRAY[{literal(owner)}::uuid],3600);"
            f"INSERT INTO ecosystem_social.owner_reviews(id,agent_id,creator_id,kind,target_id,body) VALUES"
            f"({literal(review)},{literal(agent)},{literal(owner)},'qualification',"
            f"{literal(purpose)},{literal(json.dumps(q))});")

    records = {}

    def receipt(n, i, mint, original=None, expiry=future, evidence=None):
        ident = uid(n)
        a = dict(id=ident, agent_id=uid(i), creator_id=owners[i],
                 purpose_revision_id=purposes[i], qualification_review_id=qualifications[i],
                 network='devnet', mint=mint, transaction='1'*64, evidence_id=evidence or uid(n+10000))
        record = dict(association=a, confirmed_at=current, recorded_at=current)
        if original:
            record = dict(candidate=dict(original=records[original]['association'], successor=a,
                                         authorization_id=uid(n+20000), disclosure_id=uid(n+30000)),
                          policy_id=uid(900), policy_version=1, approved_by=owners[i],
                          approved_at=past, authorization_expires_at=future,
                          disclosure_source_id=uid(n+40000), disclosure_verified_by=uid(901),
                          disclosed_at=current, disclosure_verified_at=current,
                          confirmed_at=current, recorded_at=current)
        body = literal(json.dumps(record))
        sql(f"INSERT INTO agent_tokens.receipts VALUES({literal(ident)},{literal(uid(i))},"
            f"{literal(owners[i])},{literal('successor' if original else 'original')},"
            f"{literal(uid(original)) if original else 'NULL'},{literal(purposes[i])},"
            f"{literal(qualifications[i])},'devnet',{literal(mint)},{literal('1'*64)},"
            f"{literal(a['evidence_id'])},{body},"
            f"encode(sha256(convert_to(({body}::jsonb)::text,'UTF8')),'hex'),"
            f"{literal(current)},{literal(expiry)});")
        records[n] = record
        return record

    def confirmation(n, i, record=None):
        return f"SELECT agent_tokens.confirm({literal(owners[i])},{literal(uid(n))}," + literal(json.dumps(record or records[n])) + ');'

    def snapshot():
        return sql("SELECT coalesce(jsonb_agg(to_jsonb(b) ORDER BY id),'[]') FROM agent_tokens.bindings b;")

    def financial():
        return sql("SELECT jsonb_build_object('balance',(SELECT sum(simulated_balance_microusd) FROM agents),"
                   "'ledger',(SELECT count(*) FROM simulated_ledger),'entries',(SELECT count(*) FROM economic_entries),"
                   "'holds',(SELECT count(*) FROM economic_holds),'sessions',(SELECT count(*) FROM live_sessions),"
                   "'cycles',(SELECT count(*) FROM cycles));")

    try:
        command([pg/'initdb', '-D', work/'data', '-U', 'supa_token_test', '--auth-local=trust',
                 '--auth-host=reject', '--wal-segsize=1', '--encoding=UTF8', '--locale=C.UTF-8'])
        with (work/'data/postgresql.conf').open('a') as f:
            f.write(f"\nlisten_addresses=''\nunix_socket_directories='{socket}'\nport=55481\nmax_wal_size='32MB'\nmin_wal_size='2MB'\n")
        command([pg/'pg_ctl', '-D', work/'data', '-l', out/'server.log', '-w', 'start'])
        started = True
        command([pg/'createdb', 'supa_token_test'])
        identity = sql("SELECT json_build_object('db',current_database(),'user',current_user,"
                       "'data',current_setting('data_directory'),'listen',current_setting('listen_addresses'),"
                       "'address',inet_server_addr());")
        metadata = json.loads(identity)
        assert metadata == dict(db='supa_token_test', user='supa_token_test', data=str(work/'data'), listen='', address=None)
        checks.append('verified_new_socket_only_database_identity')
        for p in sorted((ROOT/'migrations').glob('*.sql')):
            sql(p.read_text())
        sql((ROOT/'deployment/ecosystem-social.sql').read_text())
        sql((ROOT/'deployment/agent-token-registry.sql').read_text())
        sql("CREATE ROLE token_runtime NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE;"
            "CREATE ROLE token_viewer NOLOGIN NOINHERIT NOSUPERUSER;"
            "GRANT USAGE ON SCHEMA agent_tokens TO token_runtime,token_viewer;"
            "GRANT EXECUTE ON FUNCTION agent_tokens.confirm(uuid,uuid,jsonb) TO token_runtime;")
        for i in owners:
            fixture(i)
        baseline = financial()
        receipt(1000, 1, '1'*32)
        check('viewer_cannot_confirm', confirmation(1000, 1), 'token_viewer', '42501')
        check('non_owner_cannot_confirm', confirmation(1000, 2), 'token_runtime', 'token_creator_authorization_required')
        sql(f"INSERT INTO agent_access VALUES({literal(uid(1))},{literal(uid(800))},'operator',{literal(owners[1])},now());")
        check('operator_cannot_confirm', confirmation(1000, 1).replace(owners[1], uid(800)), 'token_runtime', '42501')
        altered = dict(records[1000], confirmed_at=future)
        check('tampered_evidence_denied', confirmation(1000, 1, altered), 'token_runtime', 'token_verification_receipt_invalid')
        check('original_confirmed', confirmation(1000, 1), 'token_runtime')
        check('duplicate_confirmation_denied', confirmation(1000, 1), 'token_runtime', '23505')
        receipt(1001, 1, '2'*32)
        check('replacement_denied', confirmation(1001, 1), 'token_runtime', '23505')
        receipt(1002, 2, '1'*32)
        check('cross_agent_mint_reuse_denied', confirmation(1002, 2), 'token_runtime', '23505')
        for table in ('bindings', 'receipts', 'revocations'):
            check('runtime_no_direct_insert_'+table, f"INSERT INTO agent_tokens.{table} DEFAULT VALUES;", 'token_runtime', '42501')
        check('runtime_no_financial_write', 'UPDATE agents SET simulated_balance_microusd=1;', 'token_runtime', '42501')
        check('runtime_no_acl_write', "UPDATE agent_access SET role='owner';", 'token_runtime', '42501')
        check('runtime_no_role_escalation', 'CREATE ROLE unintended;', 'token_runtime', '42501')
        for operation in ('UPDATE agent_tokens.bindings SET mint=mint;', 'DELETE FROM agent_tokens.bindings;', 'TRUNCATE agent_tokens.bindings CASCADE;'):
            check('immutable_history_'+operation.split()[0], operation, fail='42501')
        receipt(1010, 1, '3'*32, original=1000)
        check('successor_recorded_without_replacement', confirmation(1010, 1), 'token_runtime')
        assert sql("SELECT count(*) FROM agent_tokens.bindings WHERE kind='original' AND mint="+literal('1'*32)) == '1'
        receipt(1020, 3, '4'*32)
        receipt(1021, 3, '5'*32)
        with ThreadPoolExecutor(2) as pool:
            def race(n):
                try:
                    sql(confirmation(n, 3), 'token_runtime')
                    return True
                except AssertionError as error:
                    assert '23505' in str(error), error
                    return False
            assert sorted(pool.map(race, (1020, 1021))) == [False, True]
        checks.append('concurrent_original_confirmations_exactly_one_winner')
        receipt(1030, 4, '6'*32)
        sql(f"INSERT INTO agent_tokens.revocations(receipt_id) VALUES({literal(uid(1030))});")
        check('revoked_receipt_denied', confirmation(1030, 4), 'token_runtime', 'token_verification_receipt_invalid')
        receipt(1031, 4, '7'*32, expiry=(now+timedelta(microseconds=1)).isoformat())
        check('expired_receipt_denied', confirmation(1031, 4), 'token_runtime', 'token_verification_receipt_invalid')
        receipt(1032, 4, '8'*32)
        sql(f"INSERT INTO ecosystem_social.qualification_policies VALUES({literal(uid(404))},2,false,ARRAY[{literal(owners[4])}::uuid],3600);")
        check('revoked_qualification_denied', confirmation(1032, 4), 'token_runtime', 'token_current_qualification_required')
        stable = snapshot()
        check('rollback_is_atomic', 'BEGIN; '+confirmation(1001, 1)+' COMMIT;', 'token_runtime', '23505')
        assert stable == snapshot()
        command([pg/'pg_ctl', '-D', work/'data', '-m', 'fast', '-w', 'stop'])
        started = False
        command([pg/'pg_ctl', '-D', work/'data', '-l', out/'server.log', '-w', 'start'])
        started = True
        assert stable == snapshot()
        checks.append('restart_original_and_successor_persistence')
        command([pg/'pg_ctl', '-D', work/'data', '-m', 'immediate', '-w', 'stop'])
        started = False
        command([pg/'pg_ctl', '-D', work/'data', '-l', out/'server.log', '-w', 'start'])
        started = True
        assert stable == snapshot()
        checks.append('crash_recovery_preserves_exact_history')
        check('duplicate_after_recovery_denied', confirmation(1000, 1), 'token_runtime', '23505')
        assert financial() == baseline
        checks.append('financial_baseline_unchanged')
    finally:
        if started or (work/'data/postmaster.pid').exists():
            command([pg/'pg_ctl', '-D', work/'data', '-m', 'fast', '-w', 'stop'])
        assert protected() == before
        assert json.loads(subprocess.check_output(['pg_lsclusters', '--json'])) == registered
        assert migration_hashes == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT/'migrations').glob('*.sql')}
    report = dict(passed=True, postgres_version=version, checks=checks, checks_passed=len(checks),
                  source_sha256={p: hashlib.sha256((ROOT/p).read_bytes()).hexdigest()
                                 for p in ('deployment/agent-token-registry.sql', 'scripts/validate-agent-token-registry.py')},
                  disposable_data=str(work/'data'), stopped=True, migrations_unchanged=migration_hashes,
                  primary_stopped_unchanged=True, registered_clusters_unchanged=True,
                  financial_records_unchanged=True, railway_operations=0, network_requests=0)
    (out/'validation.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(passed=True, checks=len(checks), evidence=str(out.relative_to(ROOT)))))


if __name__ == '__main__':
    main()
