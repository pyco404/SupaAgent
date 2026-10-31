#!/usr/bin/env python3
"""Read-only harness admission checks, using synthetic directories and mocked config reads."""
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('status_validation',Path(__file__).with_name('validate-operator-status.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ClusterSafetyTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name);self.work=self.root/'postgres-data/stage-two/m8a_0123456789ab/workspace'
        for path in [self.work.parent,self.work,self.work/'data',self.work/'s']:path.mkdir(parents=True,mode=0o700,exist_ok=True)
        self.values={'data_directory':str(self.work/'data'),'listen_addresses':'','port':'55445','unix_socket_directories':str(self.work/'s'),'external_pid_file':''}
        self.patch=patch.object(m,'ROOT',self.root);self.patch.start();self.addCleanup(self.patch.stop)
    def check(self):
        def query(args,**kwargs):
            self.assertEqual(args[0],'/usr/lib/postgresql/16/bin/postgres');self.assertEqual(args[-2],'-C');return self.values[args[-1]]+'\n'
        with patch.object(m.subprocess,'check_output',side_effect=query):m.inspect_disposable_cluster(self.work)
    def test_valid_private_configuration_is_read_without_starting(self):self.check()
    def test_wrong_directory_and_running_cluster_rejected(self):
        with self.assertRaises(ValueError):m.inspect_disposable_cluster(self.root/'postgres-data/infrastructure-v1')
        (self.work/'data/postmaster.pid').write_text('123')
        with self.assertRaises(ValueError):self.check()
    def test_symlink_and_shared_permissions_rejected(self):
        (self.work/'data').rmdir();(self.work/'data').symlink_to(self.work/'s')
        with self.assertRaises(ValueError):self.check()
        (self.work/'data').unlink();(self.work/'data').mkdir(mode=0o755)
        with self.assertRaises(ValueError):self.check()
    def test_configuration_cannot_repoint_the_data_directory_or_network(self):
        for setting in self.values:
            original=self.values.copy();self.values[setting]='unexpected'
            with self.assertRaises(ValueError):self.check()
            self.values=original
    def test_config_and_log_symlinks_rejected(self):
        for path in [self.work/'data/postgresql.conf',self.work/'server.log']:
            path.symlink_to(self.work/'s')
            with self.assertRaises(ValueError):self.check()
            path.unlink()
if __name__=='__main__':unittest.main()
