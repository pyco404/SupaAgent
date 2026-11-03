#!/usr/bin/env python3
"""Network-free candidate boot-contract tests. Fake OpenSSL; no keys or DBs generated."""
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest

SOURCE = Path(__file__).resolve().parent.parent / 'deployment/postgres-entrypoint.sh'


class PostgresTLS(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.pgroot = self.root / 'volume'
        self.data = self.pgroot / 'pgdata'
        self.tls = self.pgroot / 'supa-tls'
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.log = self.root / 'calls.jsonl'
        self.argv = self.root / 'argv.json'
        self.env = {'PATH': str(self.bin) + ':/usr/bin:/bin', 'HOME': str(self.root),
                    'PGDATA': str(self.data), 'CALLS': str(self.log), 'ARGV': str(self.argv)}
        self.stub('id', "import os; print('0' if os.getenv('FAKE_ROOT') else '65532')")
        self.stub('chown', 'pass')
        self.stub('gosu', 'import os,sys,json; open(os.environ["ARGV"],"w").write(json.dumps(sys.argv[1:]))')
        self.stub('stat', '''import os,sys,stat
if sys.argv[2]=='%a': print(oct(stat.S_IMODE(os.stat(sys.argv[3]).st_mode))[2:])
else: print('0' if os.getenv('WRONG_UID') else '65532')''')
        self.stub('entrypoint', '''import os,sys,json
open(os.environ['ARGV'],'w').write(json.dumps(sys.argv[1:]))''')
        self.stub('openssl', '''import os,sys,json
from pathlib import Path
a=sys.argv[1:]
with open(os.environ['CALLS'],'a') as f:f.write(json.dumps(a)+chr(10))
if os.getenv('OPENSSL_FAIL'):sys.exit(1)
if '-pubin' in a:sys.stdin.read()
if a[0]=='req':
 if '-keyout' in a:Path(a[a.index('-keyout')+1]).write_text('SYNTHETIC_NOT_A_KEY')
 Path(a[a.index('-out')+1]).write_text('SYNTHETIC_RENEWED' if '-key' in a else 'SYNTHETIC_CERT')
elif a[0]=='x509' and '-checkend' in a:sys.exit(1 if os.getenv('EXPIRED') else 0)
else:print('mismatch' if os.getenv('MISMATCH') and '-pubin' in a else 'synthetic-public-material')''')
        text = SOURCE.read_text().replace('PGROOT=/var/lib/postgresql/data', 'PGROOT=' + str(self.pgroot))
        text = text.replace('/usr/libexec/postgresql18/docker-entrypoint.sh', str(self.bin / 'entrypoint'))
        self.script = self.root / 'entrypoint.sh'
        self.script.write_text(text)
        self.script.chmod(0o700)

    def stub(self, name, body):
        p = self.bin / name
        p.write_text('#!/usr/bin/python3' + chr(10) + body + chr(10))
        p.chmod(0o700)

    def run_boot(self, *, expected=0, args=('postgres',), **extra):
        r = subprocess.run([str(self.script), *args], env={**self.env, **extra},
                           capture_output=True, text=True, timeout=10)
        self.assertEqual(r.returncode, expected, r.stderr)
        self.assertNotIn('SYNTHETIC_NOT_A_KEY', r.stdout + r.stderr)
        return r

    def calls(self):
        return [json.loads(x) for x in self.log.read_text().splitlines()] if self.log.exists() else []

    def first_boot(self):
        self.run_boot()
        self.assertEqual(list(self.data.iterdir()), [])
        for name in ('server.key', 'server.crt'):
            self.assertEqual(stat.S_IMODE((self.tls / name).stat().st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(self.tls.stat().st_mode), 0o700)
        self.assertFalse(list(self.tls.glob('.pending-*')))

    def test_fresh_boot_keeps_initdb_empty_tls_private_and_execs_explicit_ssl(self):
        self.first_boot()
        args = json.loads(self.argv.read_text())
        self.assertEqual(args[0], 'postgres')
        self.assertIn('--port=5432', args)
        self.assertIn('ssl=on', args)
        self.assertIn('ssl_key_file=' + str(self.tls / 'server.key'), args)

    def test_restart_preserves_valid_identity_without_generating_a_new_key(self):
        self.first_boot()
        before = (self.tls / 'server.key').read_bytes()
        self.log.unlink()
        self.run_boot()
        self.assertEqual((self.tls / 'server.key').read_bytes(), before)
        self.assertFalse(any(a[0] == 'req' for a in self.calls()))

    def test_expiry_renews_certificate_using_the_existing_private_key(self):
        self.first_boot()
        before = (self.tls / 'server.key').read_bytes()
        self.log.unlink()
        self.run_boot(EXPIRED='1')
        request = [a for a in self.calls() if a[0] == 'req']
        self.assertEqual(len(request), 1)
        self.assertIn('-key', request[0])
        self.assertNotIn('-newkey', request[0])
        self.assertEqual((self.tls / 'server.key').read_bytes(), before)
        self.assertEqual((self.tls / 'server.crt').read_text(), 'SYNTHETIC_RENEWED')

    def test_wrong_major_is_rejected_without_enrolling_tls(self):
        self.data.mkdir(parents=True)
        (self.data / 'PG_VERSION').write_text('16')
        r = self.run_boot(expected=1)
        self.assertIn('postgres_major_mismatch', r.stderr)
        self.assertFalse(self.tls.exists())

    def test_unrecognized_nonempty_volume_is_not_initialized(self):
        self.data.mkdir(parents=True)
        (self.data / 'evidence').write_text('preserved')
        self.run_boot(expected=1)
        self.assertEqual((self.data / 'evidence').read_text(), 'preserved')
        self.assertFalse(self.tls.exists())

    def test_unexpected_pgdata_escape_is_rejected(self):
        self.run_boot(expected=1, PGDATA=str(self.pgroot / '../other'))
        self.assertFalse(self.pgroot.exists())

    def test_symlink_volume_is_rejected(self):
        other = self.root / 'other'
        other.mkdir()
        self.pgroot.symlink_to(other)
        self.run_boot(expected=1)
        self.assertEqual(list(other.iterdir()), [])

    def test_partial_tls_pair_requires_review_and_is_not_overwritten(self):
        self.first_boot()
        before = (self.tls / 'server.key').read_bytes()
        (self.tls / 'server.crt').unlink()
        self.run_boot(expected=1)
        self.assertEqual((self.tls / 'server.key').read_bytes(), before)

    def test_symlink_key_is_rejected_without_changing_target(self):
        self.first_boot()
        key = self.tls / 'server.key'
        key.unlink()
        other = self.root / 'other'
        other.write_text('preserved')
        key.symlink_to(other)
        self.run_boot(expected=1)
        self.assertEqual(other.read_text(), 'preserved')

    def test_key_permissions_and_owner_are_required(self):
        self.first_boot()
        (self.tls / 'server.key').chmod(0o644)
        self.run_boot(expected=1)
        (self.tls / 'server.key').chmod(0o600)
        self.run_boot(expected=1, WRONG_UID='1')

    def test_mismatched_key_and_certificate_fail_closed(self):
        self.first_boot()
        self.run_boot(expected=1, MISMATCH='1')
        self.assertFalse(list(self.tls.glob('.pending-*')))

    def test_openssl_failure_does_not_install_partial_identity(self):
        self.run_boot(expected=1, OPENSSL_FAIL='1')
        self.assertFalse((self.tls / 'server.key').exists())
        self.assertFalse((self.tls / 'server.crt').exists())
        self.assertFalse(list(self.tls.glob('.pending-*')))

    def test_root_drops_privilege_before_openssl_or_postgres(self):
        self.run_boot(FAKE_ROOT='1')
        args = json.loads(self.argv.read_text())
        self.assertEqual(args[:2], ['postgres', str(self.script)])
        self.assertFalse(self.log.exists())

    def test_nonserver_command_does_not_initialize_volume(self):
        self.run_boot(args=('/bin/true',))
        self.assertFalse(self.pgroot.exists())

    def test_server_options_keep_default_postgres_and_mandatory_tls(self):
        self.run_boot(args=('-c', 'max_connections=100'))
        args = json.loads(self.argv.read_text())
        self.assertEqual(args[0], 'postgres')
        self.assertIn('max_connections=100', args)
        self.assertIn('ssl=on', args)


if __name__ == '__main__':
    unittest.main(verbosity=2)
