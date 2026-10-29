#!/usr/bin/env python3
import importlib.util
import tempfile
import unittest
import uuid
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('review',Path(__file__).with_name('cutover-review.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class RollbackSafety(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.base=Path(self.temp.name)
  self.patches=[patch.object(m,'ROOT',self.base),patch.object(m.epoch.pg,'ROOT',self.base)]
  for p in self.patches:p.start()
  self.root=m.fixture_root(self.base/'postgres-data/cutover-config-test'/str(uuid.uuid4()))
  m.private_write(self.root/'api.env',b'FIXTURE=original\n')
 def tearDown(self):
  for p in reversed(self.patches):p.stop()
  self.temp.cleanup()
 def test_real_configuration_path_rejected(self):
  with self.assertRaises(ValueError):m.fixture_root(self.base/'web')
 def test_exact_configuration_round_trip_keeps_snapshot(self):
  old=(self.root/'api.env').read_bytes();record=m.install_fixture(self.root,'api.env',b'FIXTURE=active\n',m.digest(old))
  self.assertTrue(m.rollback_fixture(self.root,record));self.assertEqual((self.root/'api.env').read_bytes(),old)
  self.assertEqual((self.root/record['before']).read_bytes(),old)
 def test_changed_configuration_is_never_overwritten(self):
  old=(self.root/'api.env').read_bytes();record=m.install_fixture(self.root,'api.env',b'FIXTURE=active\n',m.digest(old))
  (self.root/'api.env').write_bytes(b'FIXTURE=operator_edit\n')
  with self.assertRaises(ValueError):m.rollback_fixture(self.root,record)
  self.assertEqual((self.root/'api.env').read_bytes(),b'FIXTURE=operator_edit\n')
 def test_tampered_snapshot_is_rejected(self):
  old=(self.root/'api.env').read_bytes();record=m.install_fixture(self.root,'api.env',b'FIXTURE=active\n',m.digest(old))
  (self.root/record['before']).write_bytes(b'tampered')
  with self.assertRaises(ValueError):m.rollback_fixture(self.root,record)
if __name__=='__main__':unittest.main()
