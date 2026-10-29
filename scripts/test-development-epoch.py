#!/usr/bin/env python3
import importlib.util
import unittest
from pathlib import Path
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('epoch',Path(__file__).with_name('development-epoch.py'))
m=importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

class EpochSafety(unittest.TestCase):
    def test_independent_destination_is_unset_and_transfers_disabled(self):
        p=m.backup_policy()
        self.assertIsNone(p['destination'])
        self.assertFalse(p['upload_enabled'])
        self.assertFalse(p['transfer_authorized'])

    def test_destination_rejects_urls_temporary_storage_and_repository(self):
        for path in ['https://backup.invalid','/tmp/backups',str(m.ROOT/'backups'),'relative','/mnt/../tmp/backup']:
            with self.assertRaises(ValueError):m.backup_policy(path)
        p=m.backup_policy('/mnt/independent-supa-backups')
        self.assertTrue(p['approval_required_before_copy'])
        self.assertFalse(p['transfer_authorized'])

    def test_restore_rejects_arbitrary_paths_before_connection(self):
        with patch.object(m,'load') as load:
            with self.assertRaises(ValueError):m.verify('../historical.dump')
            load.assert_not_called()

    def test_socket_url_does_not_target_historical_or_test_database(self):
        u=m.url('supa_epoch_dev_fixture')
        self.assertNotIn('55432',u)
        self.assertNotIn('supa_eval',u)
        self.assertNotIn('supa_infrastructure_test',u)
        self.assertIn('host=',u)

if __name__=='__main__':unittest.main()
