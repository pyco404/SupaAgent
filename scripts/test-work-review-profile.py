#!/usr/bin/env python3
"""Disposable restored-copy profile checks; no credentials, grant installer or provider.

Requires the separately started socket-only work-review fixture cluster. Never starts
PostgreSQL, connects to the retained evaluation, installs authentication or builds Rust.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest
import uuid

ROOT = Path(__file__).resolve().parent.parent
PG = '/usr/lib/postgresql/16/bin/'
AGENT = '82c254ff-4659-4404-94c4-c497565567e6'
SESSION = '41316d3e-40b8-4b7d-8605-83db7aab36b8'
ACTOR = 'acede749-746b-4541-b178-0085434dcd39'
REAL = '6fd863e1-745d-4a06-b68e-eea69e854a69'
REPORT = '01fcfc0576fb7d8cebfc09666c060eb1bff5c85a08b2682d2b17842cfe5abc42'


class ProfileTests(unittest.TestCase):
    @classmethod
    def sql(cls, query, database=None, success=True):
        env = {**cls.env, 'PGDATABASE': database or cls.database}
        result = subprocess.run([PG+'psql', '-XqAt', '--no-password', '-v', 'ON_ERROR_STOP=1', '-v', 'VERBOSITY=sqlstate'],
                                input=query, env=env, capture_output=True, text=True, timeout=20)
        if success and result.returncode:
            raise AssertionError('disposable_profile_sql_failed_without_private_details')
        if not success:
            if result.returncode == 0:
                raise AssertionError('unsafe_operation_was_not_rejected')
            if not any(code in result.stderr for code in ('42501', '23505')):
                raise AssertionError('unexpected_failure_is_not_a_permission_rejection')
            return ''
        return result.stdout.strip()

    @classmethod
    def setUpClass(cls):
        if (ROOT/'postgres-data/infrastructure-v1/data/postmaster.pid').exists():
            raise AssertionError('primary_must_remain_stopped')
        fixture = json.loads((ROOT/'artifacts/work-review-support/cluster.json').read_text())
        base = Path(fixture['base'])
        if (base.parent != Path('/tmp') or not base.name.startswith('supa-work-review-')
                or base.is_symlink() or (base/'data').is_symlink()
                or (base/'data').stat().st_uid != os.getuid()):
            raise AssertionError('wrong_disposable_fixture')
        cls.env = {'PATH': '/usr/bin:/bin', 'HOME': str(Path.home()), 'LANG': 'C.UTF-8',
                   'PGHOST': str(base/'s'), 'PGPORT': '55680', 'PGUSER': 'pikoo',
                   'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null',
                   'PGOPTIONS': '-c statement_timeout=10000 -c log_statement=none -c log_min_error_statement=panic'}
        cls.database = 'supa_review_profile_'+uuid.uuid4().hex[:12]+'_test'
        identity = cls.sql("SELECT current_setting('data_directory')||'|'||current_setting('listen_addresses')", 'supa_m10_test')
        if identity != str(base/'data')+'|':
            raise AssertionError('wrong_cluster_or_network_binding')
        cls.sql('CREATE DATABASE '+cls.database+' TEMPLATE supa_review_seed_test', 'supa_m10_test')
        if cls.sql('SELECT max(version) FROM _sqlx_migrations') != '12':
            raise AssertionError('restored_schema_twelve_required')
        # SQLx migration/checksum behavior is already covered by the compiled Rust
        # integration suite. This fixture executes identical DDL for profile tests.
        cls.sql((ROOT/'migrations/0013_work_evaluation_reviews.sql').read_text())
        cls.role = 'supa_review_profile_'+uuid.uuid4().hex[:12]
        cls.sql('CREATE ROLE '+cls.role+' NOLOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS')
        cls.sql(f'''GRANT CONNECT ON DATABASE {cls.database} TO {cls.role};
GRANT USAGE ON SCHEMA public TO {cls.role};
GRANT SELECT ON agents,agent_access,live_sessions,application_epoch,_sqlx_migrations,work_evaluation_reviews TO {cls.role};
GRANT INSERT ON work_evaluation_reviews TO {cls.role};
GRANT UPDATE(id) ON agents,live_sessions TO {cls.role};
GRANT UPDATE(granted_at) ON agent_access TO {cls.role};''')
        cls.before = cls.fingerprint()

    @classmethod
    def fingerprint(cls):
        tables = json.loads(cls.sql("SELECT json_agg(tablename ORDER BY tablename) FROM pg_tables WHERE schemaname='public' AND tablename<>'work_evaluation_reviews'"))
        digest = hashlib.sha256()
        for table in tables:
            assert table.replace('_', '').isalnum()
            rows = cls.sql(f'SELECT coalesce(jsonb_agg(v ORDER BY v::text),\'[]\'::jsonb) FROM (SELECT to_jsonb(t) v FROM "{table}" t) x')
            digest.update((table+'\n'+rows+'\n').encode())
        return digest.hexdigest()

    @classmethod
    def tearDownClass(cls):
        if hasattr(cls, 'before'):
            if cls.fingerprint() != cls.before:
                raise AssertionError('source_tables_changed')
            cls.sql('DROP OWNED BY '+cls.role+'; DROP ROLE '+cls.role)
            print(json.dumps({'disposable_database': cls.database, 'source_tables_unchanged': True,
                              'runtime_or_human_credentials_created': False, 'delegations_issued': 0,
                              'real_owner_grants_created': False, 'retained_database_contacted': False}))

    def insertion(self, actor=ACTOR):
        request = str(uuid.uuid4())
        return f'''INSERT INTO work_evaluation_reviews(request_id,session_id,agent_id,reviewer_principal_id,decision,report_sha256,confirmation)
VALUES('{request}','{SESSION}','{AGENT}','{actor}','accept_evaluation_only','{REPORT}',
jsonb_build_object('request_id','{request}','decision','accept_evaluation_only','report_sha256','{REPORT}',
'confirm_review',true,'consented_at',clock_timestamp(),'expires_at',clock_timestamp()+interval '1 minute'));'''

    def test_profile_locks_and_review_need_no_financial_update(self):
        response = self.sql(f'''BEGIN; SET LOCAL ROLE {self.role};
SELECT id FROM agents WHERE id='{AGENT}' FOR UPDATE;
SELECT role FROM agent_access WHERE agent_id='{AGENT}' AND principal_id='{ACTOR}' FOR UPDATE;
SELECT id FROM live_sessions WHERE id='{SESSION}' FOR UPDATE;
{self.insertion()}
SELECT decision FROM work_evaluation_reviews WHERE session_id='{SESSION}'; ROLLBACK;''')
        self.assertIn('accept_evaluation_only', response)
        self.assertEqual(self.fingerprint(), self.before)
        self.assertEqual(self.sql('SELECT count(*) FROM work_evaluation_reviews'), '0')

    def test_profile_cannot_change_money_job_session_or_ownership(self):
        for statement in [
            'UPDATE economic_jobs SET revision=revision WHERE false',
            'UPDATE economic_entries SET id=id WHERE false',
            'UPDATE live_budgets SET committed_microusd=committed_microusd WHERE false',
            'UPDATE live_sessions SET data=data WHERE false',
            "UPDATE agent_access SET role='owner' WHERE false",
            'INSERT INTO agent_access SELECT * FROM agent_access WHERE false',
            'DELETE FROM work_evaluation_reviews WHERE false',
            'TRUNCATE work_evaluation_reviews',
        ]:
            with self.subTest(operation=statement.split()[0:2]):
                self.sql(f'BEGIN; SET LOCAL ROLE {self.role}; '+statement+'; ROLLBACK;', success=False)
        dangerous = self.sql(f'''SELECT has_table_privilege('{self.role}','economic_jobs','UPDATE')
OR has_column_privilege('{self.role}','live_budgets','committed_microusd','UPDATE')
OR has_column_privilege('{self.role}','live_sessions','data','UPDATE')
OR has_column_privilege('{self.role}','agent_access','role','UPDATE')''')
        self.assertEqual(dangerous, 'f')

    def test_real_owner_cannot_claim_test_principal_or_replace_owner(self):
        self.assertEqual(self.sql(f"SELECT principal_id FROM agent_access WHERE agent_id='{AGENT}' AND role='owner'"), ACTOR)
        self.sql(f'BEGIN; {self.insertion(REAL)} ROLLBACK;', success=False)
        # Even a hypothetical ownership replacement in a rolled-back synthetic
        # transaction is insufficient: the original session approver remains bound.
        self.sql(f'''BEGIN; UPDATE agent_access SET principal_id='{REAL}' WHERE agent_id='{AGENT}' AND role='owner';
{self.insertion(REAL)} ROLLBACK;''', success=False)
        # An additional owner also violates the existing one-owner constraint.
        self.sql(f'''BEGIN; INSERT INTO agent_access(agent_id,principal_id,role,granted_by)
VALUES('{AGENT}','{REAL}','owner','{ACTOR}'); ROLLBACK;''', success=False)
        self.assertEqual(self.fingerprint(), self.before)

    def test_scope_and_original_generation_are_preserved(self):
        value = json.loads(self.sql(f"SELECT data FROM live_sessions WHERE id='{SESSION}'"))
        self.assertEqual(value['approved_by'], ACTOR)
        self.assertEqual(value['generation_requests'], 1)
        self.assertEqual(value['status'], 'awaiting_approval')
        digest = self.sql(f"SELECT encode(sha256(convert_to(work_review_canonical(data->'work'->'report'),'UTF8')),'hex') FROM live_sessions WHERE id='{SESSION}'")
        self.assertEqual(digest, REPORT)
        self.assertEqual(self.fingerprint(), self.before)


if __name__ == '__main__':
    unittest.main()
