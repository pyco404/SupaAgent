#!/usr/bin/env python3
"""Offline comparison tests; optional schema-only disposable local restore rehearsal."""
import importlib.util
import json
import os
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import uuid

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('metadata', ROOT/'scripts/postgres-metadata.py')
metadata = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metadata)


class MetadataTests(unittest.TestCase):
    def test_object_insertion_order_does_not_change_semantic_comparison(self):
        a = {'triggers': {'public.a.same': {'enabled': 'O'}, 'public.b.same': {'enabled': 'D'}}}
        b = {'triggers': dict(reversed(list(a['triggers'].items())))}
        self.assertEqual(metadata.compare(a, b), [])

    def test_each_object_change_missing_or_extra_is_reported(self):
        for category in ['database','schemas','functions','triggers','relations','constraints','indexes',
                         'table_privileges','column_privileges','extensions','extension_members']:
            a = {category: {'object-a': {'definition': 'before', 'owner': 'owner-a'}}}
            for b in [{category: {}}, {category: {'object-a': {'definition': 'after', 'owner': 'owner-a'}}},
                      {category: {'object-a': {'definition': 'before', 'owner': 'owner-b'}}}]:
                changes = metadata.compare(a, b)
                self.assertEqual(len(changes), 1)
                self.assertEqual(changes[0]['category'], category)
                self.assertEqual(changes[0]['object'], 'object-a')
                self.assertEqual(len(metadata.compare(b, a)), 1)

    def test_mismatch_fails_and_persists_complete_values(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp)
            with self.assertRaisesRegex(AssertionError, 'restored_schema_metadata_differs'):
                metadata.require_equal({'functions': {'f()': 'before'}}, {'functions': {'f()': 'after'}}, path)
            self.assertEqual(json.loads((path/'schema-comparison.json').read_text())['differences'][0]['source'], 'before')

    def test_constraint_expression_and_enforcement_changes_are_not_normalized_away(self):
        original = {'definition': "CHECK ((historical_database = 'unrecovered'::text))",
                    'validated': True, 'enforced': True, 'type': 'c',
                    'deferrable': False, 'deferred': False, 'no_inherit': False}
        key = 'public.application_epoch.application_epoch_historical_database_check'
        for field, value in [('definition', "CHECK ((historical_database = 'other'::text))"),
                             ('validated', False), ('enforced', False), ('no_inherit', True),
                             ('deferrable', True), ('deferred', True), ('type', 'n')]:
            with self.subTest(field=field):
                self.assertEqual(len(metadata.compare({'constraints': {key: original}},
                    {'constraints': {key: {**original, field: value}}})), 1)


def rehearsal():
    """Use only the identified running launch fixture; copy no application rows."""
    assert not (ROOT/'postgres-data/infrastructure-v1/data/postmaster.pid').exists()
    config_path = ROOT/'artifacts/local-launch/runtime.json'
    info = config_path.lstat()
    assert not config_path.is_symlink() and info.st_uid == os.getuid() and not info.st_mode & 0o077
    fixture = json.loads(config_path.read_text())
    base = Path(fixture['base'])
    assert fixture['purpose'] == 'disposable_local_agent_launch'
    assert base.parent == Path('/tmp') and base.name.startswith('supa-launch-')
    assert fixture['socket'] == str(base/'socket')
    assert fixture['database'] == 'supa_launch_'+base.name.removeprefix('supa-launch-')+'_test'
    env = {'PATH': os.environ['PATH'], 'HOME': str(Path.home()), 'LANG': 'C.UTF-8',
           'PGHOST': fixture['socket'], 'PGPORT': '55690', 'PGUSER': 'pikoo',
           'PGPASSFILE': '/dev/null', 'PGSERVICEFILE': '/dev/null'}
    suffix = uuid.uuid4().hex[:12]
    source, restored = 'supa_meta_'+suffix+'_test', 'supa_metar_'+suffix+'_test'
    out = ROOT/'artifacts/milestone-11-metadata'/suffix
    out.mkdir(parents=True, mode=0o700)
    created = []
    def pg(name, *args, db='postgres', data=None):
        result = subprocess.run(['/usr/lib/postgresql/16/bin/'+name, *args],
            env={**env,'PGDATABASE':db}, input=data, capture_output=True, text=True, timeout=60)
        if result.returncode: raise RuntimeError('disposable_metadata_'+name+'_failed')
        return result.stdout
    def sql(query, db='postgres'):
        return pg('psql','-XqAt','-v','ON_ERROR_STOP=1','-c',query,db=db).strip()
    assert sql('SHOW data_directory') == str(base/'data')
    assert sql('SELECT system_identifier::text FROM pg_control_system()') == fixture['system_identifier']
    assert shutil.disk_usage(ROOT).free >= 96 * 1024**2, 'disposable_metadata_headroom_required'
    try:
        for database in [source,restored]:
            pg('createdb',database); created.append(database)
        # Preserve migration creation order: restoring a schema dump first would
        # already sort objects and conceal the original trigger-order defect.
        sql('CREATE TABLE public._sqlx_migrations(version bigint PRIMARY KEY,description text NOT NULL,'
            'installed_on timestamptz NOT NULL DEFAULT now(),success boolean NOT NULL,'
            'checksum bytea NOT NULL,execution_time bigint NOT NULL)',source)
        for migration in sorted((ROOT/'migrations').glob('*.sql')):
            pg('psql','-Xq','--single-transaction','-v','ON_ERROR_STOP=1',
               db=source,data=migration.read_text())
        # PG18 adds named NOT NULL constraints alongside CHECK constraints.
        # Reproduce their qualified-identity collision even on local PG16.
        sql("CREATE TABLE public.metadata_identity_fixture(historical_database text, "
            "CONSTRAINT application_epoch_historical_database_check CHECK(historical_database='unrecovered'), "
            "CONSTRAINT application_epoch_historical_database_not_null CHECK(historical_database IS NOT NULL))",source)
        type_probe = sql("WITH objects AS (SELECT 'public'::name AS identity UNION ALL SELECT repeat('x',100)) "
                         "SELECT pg_typeof(identity)::text||':'||length(identity::text) FROM objects ORDER BY length(identity::text)",source)
        assert type_probe == 'name:6\nname:63'
        fixed_probe = sql("WITH objects AS (SELECT 'public'::text AS identity UNION ALL SELECT repeat('x',100)) "
                          "SELECT pg_typeof(identity)::text||':'||length(identity::text) FROM objects ORDER BY length(identity::text)",source)
        assert fixed_probe == 'text:6\ntext:100'
        old_query = """SELECT jsonb_build_object(
          'functions',(SELECT jsonb_agg(pg_get_functiondef(p.oid) ORDER BY n.nspname,p.proname,p.oid::regprocedure::text)
            FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname IN ('public','epoch_guard')),
          'triggers',(SELECT jsonb_agg(pg_get_triggerdef(t.oid) ORDER BY t.tgname) FROM pg_trigger t WHERE NOT t.tgisinternal),
          'privileges',(SELECT coalesce(jsonb_agg(to_jsonb(p)-'table_catalog' ORDER BY table_schema,table_name,grantee,privilege_type),'[]')
            FROM information_schema.table_privileges p WHERE table_schema IN ('public','epoch_guard')))"""
        before = json.loads(sql(metadata.SNAPSHOT_SQL,source))
        prefix = 'public.metadata_identity_fixture.'
        fixture_constraints = {k:v for k,v in before['constraints'].items() if k.startswith(prefix)}
        assert len(fixture_constraints)==2 and all(len(k)>63 for k in fixture_constraints)
        broken_query = metadata.SNAPSHOT_SQL.replace('n.nspname::text AS identity','n.nspname AS identity')
        broken = json.loads(sql(broken_query,source))
        assert len([k for k in broken['constraints'] if k.startswith(prefix)]) == 1
        old_before = json.loads(sql(old_query,source))
        dump = out/'schema-only-fixture.dump'
        pg('pg_dump','--format=custom','--file',str(dump),db=source)
        dump.chmod(0o600)
        pg('pg_restore','--single-transaction','--exit-on-error','--dbname',restored,str(dump))
        after = json.loads(sql(metadata.SNAPSHOT_SQL,restored))
        old_after = json.loads(sql(old_query,restored))
        from collections import Counter
        changed = [k for k in old_before if old_before[k] != old_after[k]]
        report = {'old_comparison_equal':old_before==old_after,'old_changed_categories':changed,
            'old_trigger_multisets_equal':Counter(old_before['triggers'])==Counter(old_after['triggers']),
            'old_trigger_order_differences':[
                {'index':i,'source':a,'restored':b} for i,(a,b) in enumerate(zip(old_before['triggers'],old_after['triggers'])) if a!=b],
            'new_comparison':metadata.require_equal(before,after,out),
            'identity_type_truncation_reproduced':True,'long_constraints_preserved':fixture_constraints,
            'postgres_major':16,'historical_application_rows_copied':0,'primary_accessed':False}
        (out/'rehearsal.json').write_text(json.dumps(report,indent=2)+'\n')
        # Confirm the same comparison detects a real change, without mutating source.
        sql("ALTER TABLE public.metadata_identity_fixture DROP CONSTRAINT application_epoch_historical_database_check, "
            "ADD CONSTRAINT application_epoch_historical_database_check CHECK(historical_database='changed')",restored)
        constraint_changes = metadata.compare(before,json.loads(sql(metadata.SNAPSHOT_SQL,restored)))
        assert any(d['category']=='constraints' and d['object']==prefix+'application_epoch_historical_database_check'
                   for d in constraint_changes)
        (out/'constraint-change.json').write_text(json.dumps(constraint_changes,indent=2)+'\n')
        sql('ALTER TABLE public.agents DISABLE TRIGGER operational_write',restored)
        changed_schema = json.loads(sql(metadata.SNAPSHOT_SQL,restored))
        changes = metadata.compare(before,changed_schema)
        assert any(d['category']=='triggers' and d['object']=='public.agents.operational_write' for d in changes)
        sql('REVOKE EXECUTE ON FUNCTION epoch_guard.verify_active(uuid,uuid) FROM PUBLIC',restored)
        sql('REVOKE USAGE ON SCHEMA public FROM PUBLIC',restored)
        sql('REVOKE CONNECT ON DATABASE '+restored+' FROM PUBLIC',restored)
        changed_schema = json.loads(sql(metadata.SNAPSHOT_SQL,restored))
        categories = {d['category'] for d in metadata.compare(before,changed_schema)}
        assert {'triggers','functions','schemas','database'} <= categories
        print(json.dumps({k:v for k,v in report.items() if k!='old_trigger_order_differences'}))
        print(json.dumps({'real_trigger_and_acl_tampering_detected':sorted(categories),'evidence':str(out),'old_order_difference_count':len(report['old_trigger_order_differences'])}))
    finally:
        for database in reversed(created): pg('dropdb',database)


if __name__ == '__main__':
    if sys.argv[1:] == ['--disposable-rehearsal']: rehearsal()
    else: unittest.main(verbosity=2)
