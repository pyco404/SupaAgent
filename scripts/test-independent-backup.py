#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import tempfile
import unittest
spec=importlib.util.spec_from_file_location('backup',Path(__file__).with_name('independent-backup-preflight.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class DiskTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
        (self.root/'dev/block').mkdir(parents=True)
    def disk(self,name,major):
        disk=self.root/'devices'/name;disk.mkdir(parents=True);(disk/'device').mkdir();(disk/'dev').write_text(major)
        (self.root/'dev/block'/major).symlink_to(disk)
        return disk
    def test_separate_disks_are_disjoint(self):
        self.disk('nvme0','259:0');self.disk('usb1','8:0')
        self.assertTrue(m.physical_disks('259:0',self.root).isdisjoint(m.physical_disks('8:0',self.root)))
    def test_different_partitions_are_same_disk(self):
        disk=self.disk('nvme0','259:0')
        for name,major in [('p1','259:1'),('p2','259:2')]:
            p=disk/name;p.mkdir();(p/'partition').write_text('1');(self.root/'dev/block'/major).symlink_to(p)
        self.assertEqual(m.physical_disks('259:1',self.root),m.physical_disks('259:2',self.root))
    def test_mapper_resolves_backing_disk(self):
        disk=self.disk('nvme0','259:0');mapper=self.root/'devices/virtual/dm0';(mapper/'slaves').mkdir(parents=True)
        (mapper/'slaves/disk').symlink_to(disk);(self.root/'dev/block/253:0').symlink_to(mapper)
        self.assertEqual(m.physical_disks('253:0',self.root),m.physical_disks('259:0',self.root))
    def test_unknown_and_virtual_storage_rejected(self):
        with self.assertRaises(ValueError):m.physical_disks('0:1',self.root)
        disk=self.root/'devices/virtual/loop0';disk.mkdir(parents=True);(self.root/'dev/block/7:0').symlink_to(disk)
        with self.assertRaises(ValueError):m.physical_disks('7:0',self.root)
    def test_symlink_and_shared_destination_rejected(self):
        target=self.root/'target';target.mkdir(mode=0o700);alias=self.root/'alias';alias.symlink_to(target)
        with self.assertRaises(ValueError):m.review(alias,source=target,sys_root=self.root)
        target.chmod(0o755)
        with self.assertRaises(ValueError):m.review(target,source=target,sys_root=self.root)
if __name__=='__main__':unittest.main()
