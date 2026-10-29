#!/usr/bin/env python3
"""Safety tests: no server, historical input, or credentials required."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('persistent', Path(__file__).with_name('persistent-postgres.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class InfrastructureSafety(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.base = self.root / 'private'
        self.context = patch.multiple(m, ROOT=self.root, BASE=self.base)
        self.context.start()

    def tearDown(self):
        self.context.stop()
        self.temp.cleanup()

    def test_rejects_symlink_and_shared_directory(self):
        self.base.symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(ValueError):
            m.private_directory(self.base)
        self.base.unlink()
        self.base.mkdir(mode=0o755)
        with self.assertRaises(ValueError):
            m.private_directory(self.base)

    def test_manifest_cannot_be_overwritten(self):
        m.private_directory(self.base)
        path = self.base / 'identity.json'
        m.exclusive_json(path, m.IDENTITY)
        with self.assertRaises(FileExistsError):
            m.exclusive_json(path, {})
        self.assertEqual(json.loads(path.read_text()), m.IDENTITY)
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)

    def test_no_inherited_secrets_or_network_host(self):
        with patch.dict(os.environ, {'OPENAI_API_KEY': 'fixture-not-secret', 'PGPASSWORD': 'fixture'}):
            env = m.environment()
        self.assertNotIn('OPENAI_API_KEY', env)
        self.assertNotIn('PGPASSWORD', env)
        self.assertEqual(env['PGHOST'], str(self.base / 'socket'))
        self.assertEqual(env['PGPASSFILE'], '/dev/null')

    def test_existing_data_is_never_reinitialized(self):
        m.private_directory(self.base)
        (self.base / 'sentinel').write_text('preserve')
        with patch.object(m.subprocess, 'run') as run:
            run.return_value.returncode = 0
            with self.assertRaises(ValueError):
                m.prepare()
        self.assertEqual((self.base / 'sentinel').read_text(), 'preserve')
        self.assertEqual(run.call_count, 1)

    def test_restore_rejects_path_traversal(self):
        with patch.object(m, 'identity') as identity:
            with self.assertRaises(ValueError):
                m.verify('../../original')
            identity.assert_not_called()

    def test_tampered_backup_rejected_before_restore(self):
        bid = '84961021-a102-4790-8878-6d8a5e23ffaf'
        folder = self.base / 'backups' / bid
        m.private_directory(folder)
        m.exclusive_json(folder / 'manifest.json', {'version': 2, 'identity': m.IDENTITY, 'source_database': m.DATABASE,
                         'database_sha256': 'wrong', 'roles_sha256': 'wrong'})
        (folder / 'database.dump').touch(mode=0o600)
        with patch.object(m, 'identity'), patch.object(m, 'command') as command:
            with self.assertRaisesRegex(ValueError, 'backup_checksum_mismatch'):
                m.verify(bid)
            command.assert_not_called()


if __name__ == '__main__':
    unittest.main()
