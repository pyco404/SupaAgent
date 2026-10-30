#!/usr/bin/env python3
"""Offline preflight safety checks; never starts or connects to PostgreSQL."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('preflight',Path(__file__).with_name('preflight-development-epoch.py'))
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class StageOneSafety(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory()
        self.root=Path(self.temp.name)
        self.patch=patch.multiple(m,ROOT=self.root)
        self.pg_patch=patch.object(m.pg,'ROOT',self.root)
        self.patch.start();self.pg_patch.start()
    def tearDown(self):
        self.pg_patch.stop();self.patch.stop();self.temp.cleanup()
    def test_explicit_stage_one_flag_required_before_any_operation(self):
        script=Path(__file__).with_name('preflight-development-epoch.py')
        result=subprocess.run([sys.executable,str(script)],capture_output=True)
        self.assertEqual(result.returncode,2)
        self.assertIn(b'--approved-stage-one',result.stderr)
    def test_custodian_proposal_is_distinct_stable_private_and_ungranted(self):
        first=m.prepare_custodian();second=m.prepare_custodian()
        self.assertEqual(first,second)
        self.assertNotEqual(first['principal_id'],m.OWNER)
        self.assertFalse(first['authority_granted'])
        self.assertFalse(first['credentials_prepared'])
        self.assertEqual((self.root/'postgres-data/custodian-provisioning/identity.json').stat().st_mode&0o777,0o600)
    def test_claimed_independent_control_cannot_be_silently_accepted(self):
        record=m.prepare_custodian();record['independent_credential_control_verified']=True
        (self.root/'postgres-data/custodian-provisioning/identity.json').write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError,'custodian_proposal_mismatch'):m.prepare_custodian()
    def test_owner_reuse_is_rejected_without_replacement(self):
        record=m.prepare_custodian();record['principal_id']=m.OWNER
        file=self.root/'postgres-data/custodian-provisioning/identity.json'
        file.write_text(json.dumps(record));before=file.read_bytes()
        with self.assertRaisesRegex(ValueError,'custodian_must_be_distinct'):m.prepare_custodian()
        self.assertEqual(file.read_bytes(),before)
    def test_existing_symlink_is_rejected(self):
        m.prepare_custodian()
        file=self.root/'postgres-data/custodian-provisioning/identity.json'
        original=file.read_bytes();file.unlink()
        other=self.root/'preserve.json';other.write_bytes(original);file.symlink_to(other)
        with self.assertRaises(ValueError):m.prepare_custodian()
        self.assertEqual(other.read_bytes(),original)


if __name__=='__main__':unittest.main()
