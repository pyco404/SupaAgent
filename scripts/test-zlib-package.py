#!/usr/bin/env python3
"""Fail-closed input/path tests for isolated zlib candidate preparation."""
import importlib.util
from pathlib import Path
import tempfile
import tarfile
import io
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('zlib_package', Path(__file__).with_name('prepare-zlib-package.py'))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class PackageGuards(unittest.TestCase):
    def test_changed_reviewed_patch_rejected_before_output_creation(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(MODULE, 'PATCH_SHA', '0' * 64):
            out = Path(tmp) / 'out'
            with self.assertRaisesRegex(ValueError, 'reviewed_patch_hash_mismatch'):
                MODULE.prepare(Path(tmp), out)
            self.assertFalse(out.exists())

    def test_unapproved_source_bytes_rejected_before_output_creation(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / next(iter(MODULE.INPUTS))).write_bytes(b'unapproved synthetic source')
            with self.assertRaisesRegex(ValueError, 'pinned_input_required'):
                MODULE.prepare(root, root / 'out')
            self.assertFalse((root / 'out').exists())

    def test_symlink_input_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'actual').write_bytes(b'synthetic')
            (root / next(iter(MODULE.INPUTS))).symlink_to('actual')
            with self.assertRaisesRegex(ValueError, 'pinned_input_required'):
                MODULE.prepare(root, root / 'out')

    def malformed_archive(self, name, member_type, size=0):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            archive = root / 'source.tar'
            with tarfile.open(archive, 'w') as handle:
                member = tarfile.TarInfo(name)
                member.type = member_type
                member.linkname = '/etc/passwd' if member_type == tarfile.SYMTYPE else ''
                member.size = size
                handle.addfile(member, io.BytesIO(b'x' * size) if size else None)
            with self.assertRaisesRegex(ValueError, 'regular_bounded_source_archive_required'):
                MODULE.extract(archive, root / 'extract')
            self.assertFalse((root / 'extract').exists())

    def test_parent_traversal_rejected(self):
        self.malformed_archive('../outside', tarfile.REGTYPE)

    def test_absolute_path_rejected(self):
        self.malformed_archive('/outside', tarfile.REGTYPE)

    def test_archive_symlinks_rejected(self):
        self.malformed_archive('source/link', tarfile.SYMTYPE)

    def test_device_nodes_rejected(self):
        self.malformed_archive('source/device', tarfile.CHRTYPE)

    def test_normal_source_file_extracts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with tarfile.open(root / 'source.tar', 'w') as handle:
                member = tarfile.TarInfo('source/ok'); member.size = 2
                handle.addfile(member, io.BytesIO(b'ok'))
            MODULE.extract(root / 'source.tar', root / 'extract')
            self.assertEqual((root / 'extract/source/ok').read_bytes(), b'ok')


if __name__ == '__main__':
    unittest.main()
