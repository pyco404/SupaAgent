#!/usr/bin/env python3
"""No database, network or real identity writes; synthetic preparation contracts."""
import concurrent.futures
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import uuid

spec = importlib.util.spec_from_file_location('owner', Path(__file__).with_name('owner-provisioning.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class OwnerPreparation(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.base = self.root / 'postgres-data/owner-provisioning'
        self.patches = [patch.object(m, 'ROOT', self.root), patch.object(m, 'BASE', self.base),
                        patch.object(m.review.epoch.pg, 'ROOT', self.root), patch.object(m, 'ignored')]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in reversed(self.patches):
            p.stop()
        self.temp.cleanup()

    def test_id_and_credentials_stable_without_grant(self):
        first = m.prepare()
        secret = (self.base / 'credentials.pending.json').read_bytes()
        self.assertEqual(first, m.prepare())
        self.assertEqual(secret, (self.base / 'credentials.pending.json').read_bytes())
        self.assertFalse(first['permissions_granted'])
        self.assertFalse(first['authentication_enabled'])
        for value in m.credentials().values():
            if len(value) > 24:
                self.assertNotIn(value, json.dumps(first))
        self.assertEqual(self.base.stat().st_mode & 0o077, 0)
        self.assertEqual((self.base / 'identity.json').stat().st_mode & 0o077, 0)

    def test_concurrent_preparation_has_single_identity(self):
        m.prepare()  # Establish owned directory before exercising concurrent locks.
        with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
            results = list(pool.map(lambda _: m.prepare(), range(8)))
        self.assertEqual(len({r['principal_id'] for r in results}), 1)

    def test_permission_change_fails_without_replacement(self):
        m.prepare()
        target = self.base / 'credentials.pending.json'
        before = target.read_bytes()
        target.chmod(0o644)
        with self.assertRaises(ValueError):
            m.prepare()
        self.assertEqual(target.read_bytes(), before)

    def test_symlink_and_hardlink_rejected(self):
        m.prepare()
        target = self.base / 'identity.json'
        target.rename(self.base / 'original.json')
        target.symlink_to(self.base / 'original.json')
        with self.assertRaises(ValueError):
            m.load()
        target.unlink()
        os.link(self.base / 'original.json', target)
        with self.assertRaises(ValueError):
            m.load()

    def test_corrupt_or_changed_binding_never_regenerates(self):
        original = m.prepare()
        path = self.base / 'identity.json'
        value = json.loads(path.read_text())
        value['authentication_enabled'] = True
        path.write_text(json.dumps(value))
        with self.assertRaises(ValueError):
            m.prepare()
        self.assertEqual(json.loads(path.read_text())['principal_id'], original['principal_id'])
        path.write_text('{}')
        with self.assertRaises(KeyError):
            m.prepare()

    def test_missing_identity_cannot_remap_existing_credentials(self):
        m.prepare()
        credentials = (self.base / 'credentials.pending.json').read_bytes()
        (self.base / 'identity.json').unlink()
        with self.assertRaises(ValueError):
            m.prepare()
        self.assertFalse((self.base / 'identity.json').exists())
        self.assertEqual(credentials, (self.base / 'credentials.pending.json').read_bytes())

    def test_credential_collision_rejected(self):
        m.prepare()
        path = self.base / 'credentials.pending.json'
        value = json.loads(path.read_text())
        value['service_token'] = value['operator_token']
        path.write_text(json.dumps(value))
        with self.assertRaises(ValueError):
            m.credentials()

    def test_json_rosters_use_literal_dotenv_quoting(self):
        roster = '[{"role":"owner","id":"synthetic"}]'
        self.assertEqual(m.dotenv({'SUPA_OPERATOR_CREDENTIALS': roster}),
                         ("SUPA_OPERATOR_CREDENTIALS='" + roster + "'\n").encode())
        for bad in ["quote'break", "line\nbreak", "null\0break"]:
            with self.assertRaises(ValueError):
                m.dotenv({'SUPA_API_TOKEN': bad})

    def test_render_is_guarded_review_only(self):
        principal = str(uuid.uuid4())
        database = 'supa_epoch_dev_' + uuid.UUID(m.EPOCH).hex
        sql = m.render_transition(database, principal, 'activate_read_only')
        self.assertIn('epoch_guard.execute_transition', sql)
        self.assertNotIn('UPDATE application_epoch', sql)
        self.assertIn('plan_text', sql)
        self.assertIn('expected_revision', sql)
        self.assertIn('BEGIN READ WRITE', sql)
        self.assertIn(database, sql)
        self.assertIn(principal, sql)
        for bad in ['production', "x'; DELETE FROM agents; --"]:
            with self.assertRaises(ValueError):
                m.render_transition(bad, principal, 'rollback')
        for bad in [m.EPOCH, m.review.epoch.pg.ARCHIVED, str(uuid.UUID(int=0))]:
            with self.assertRaises(ValueError):
                m.render_transition(database, bad, 'rollback')


if __name__ == '__main__':
    unittest.main()
