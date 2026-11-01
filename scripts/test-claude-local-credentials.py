#!/usr/bin/env python3
"""Synthetic credentials and mocked metadata only; never reads a real API key."""
import importlib.util
import json
import os
from pathlib import Path
import pty
import select
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('credentials', Path(__file__).with_name('claude-local-credentials.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)
FAKE = 'sk-ant-' + 'synthetic-not-a-real-key'


class Credentials(unittest.TestCase):
    def test_atomic_private_updates_preserve_other_provider_and_workspace(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            other = root / '.env.eval'; other.write_text('OPENAI_MODEL=preserve-this\n')
            path = root / '.env.claude'
            m.write_private({'ANTHROPIC_API_KEY': FAKE, 'ANTHROPIC_WORKSPACE_ID': 'wrkspc_fixture'}, None, path)
            values, identity = m.read_private(path)
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            values['ANTHROPIC_MODEL'] = 'claude-haiku-5-5'
            m.write_private(values, identity, path)
            self.assertEqual(m.read_private(path)[0]['ANTHROPIC_WORKSPACE_ID'], 'wrkspc_fixture')
            self.assertEqual(other.read_text(), 'OPENAI_MODEL=preserve-this\n')
            with self.assertRaises(ValueError):
                m.write_private(values, None, path)

    def test_unsafe_permissions_links_duplicates_and_shell_syntax_fail_closed(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / '.env.claude'
            path.write_text('ANTHROPIC_MODEL=claude-haiku-5-5\n'); path.chmod(0o644)
            with self.assertRaises(ValueError): m.read_private(path)
            path.chmod(0o600)
            link = Path(tmp) / 'link'; link.symlink_to(path)
            with self.assertRaises(ValueError): m.read_private(link)
            for text in ['OPENAI_MODEL=forbidden\n', 'SUPA_ALLOW_ANTHROPIC=true\n', 'ANTHROPIC_MODEL=$(command)\n', 'ANTHROPIC_MODEL=a\nANTHROPIC_MODEL=b\n']:
                path.write_text(text)
                with self.assertRaises(ValueError): m.read_private(path)

    def test_presence_diagnostics_never_expose_secret_or_claim_access(self):
        with patch.dict(os.environ, {}, clear=True):
            result = m.check({'ANTHROPIC_API_KEY': FAKE}, True)
            self.assertTrue(result['key_present'])
            self.assertNotIn(FAKE, json.dumps(result))
            self.assertFalse(result['authentication_verified'])
            self.assertEqual(result['generation_requests'], 0)

    def test_probe_requires_approval_before_network_and_valid_scope(self):
        with patch.object(m.http.client, 'HTTPSConnection', side_effect=AssertionError('network forbidden')):
            with self.assertRaises(ValueError): m.probe({}, False)
            with patch.dict(os.environ, {}, clear=True):
                with self.assertRaises(ValueError): m.probe({'ANTHROPIC_API_KEY': FAKE}, True)

    def test_approved_probe_uses_one_get_no_redirects_and_only_safe_output(self):
        class Connection:
            def __init__(self, host, timeout):
                self.host = host; self.timeout = timeout; self.calls = []
            def request(self, method, path, headers): self.calls.append((method, path, headers))
            def getresponse(self): return self
            def read(self, limit):
                return json.dumps({'type': 'model', 'id': 'claude-haiku-5-5', 'private': FAKE}).encode()
            def close(self): pass
        for status in [200, 302, 401, 403, 404, 429]:
            conn = Connection('api.anthropic.com', 15); conn.status = status
            values = {'ANTHROPIC_API_KEY': FAKE, 'ANTHROPIC_WORKSPACE_ID': 'wrkspc_fixture', 'ANTHROPIC_MODEL': 'claude-haiku-5-5'}
            with patch.dict(os.environ, {}, clear=True), patch.object(m.http.client, 'HTTPSConnection', return_value=conn):
                result = m.probe(values, True)
            self.assertEqual(len(conn.calls), 1)
            self.assertEqual(conn.calls[0][0], 'GET')
            self.assertEqual(conn.calls[0][2]['anthropic-workspace-id'], 'wrkspc_fixture')
            self.assertEqual(result['authentication_verified'], status == 200)
            self.assertNotIn(FAKE, json.dumps(result))
            self.assertEqual(result['generation_requests'], 0)

    def test_interactive_terminal_never_echoes_key_and_non_tty_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / 'scripts').mkdir()
            script = root / 'scripts/claude-local-credentials.py'; shutil.copyfile(m.__file__, script)
            git = root / 'scripts/git-local'; git.write_text('#!/bin/sh\nexit 0\n'); git.chmod(0o700)
            other = root / '.env.eval'; other.write_text('OPENAI_MODEL=unchanged\n')
            plain = subprocess.run([sys.executable, str(script), 'set-key'], input=FAKE, text=True, capture_output=True)
            self.assertNotEqual(plain.returncode, 0)
            self.assertNotIn(FAKE, plain.stdout + plain.stderr)
            unknown = subprocess.run([sys.executable, str(script), 'check', '--key', FAKE], text=True, capture_output=True)
            self.assertNotEqual(unknown.returncode, 0)
            self.assertNotIn(FAKE, unknown.stdout + unknown.stderr)
            master, slave = pty.openpty()
            child = subprocess.Popen([sys.executable, str(script), 'set-key'], stdin=slave, stdout=slave, stderr=slave, start_new_session=True)
            os.close(slave)
            output = b''
            try:
                deadline = time.monotonic() + 8
                while b'(hidden):' not in output and time.monotonic() < deadline:
                    if select.select([master], [], [], .2)[0]: output += os.read(master, 4096)
                self.assertIn(b'(hidden):', output)
                os.write(master, (FAKE + '\n').encode())
                while child.poll() is None and time.monotonic() < deadline:
                    if select.select([master], [], [], .2)[0]:
                        try: output += os.read(master, 4096)
                        except OSError: break
                self.assertEqual(child.wait(timeout=3), 0)
                self.assertNotIn(FAKE.encode(), output)
                self.assertTrue(m.read_private(root / '.env.claude')[0]['ANTHROPIC_API_KEY'])
                self.assertEqual(other.read_text(), 'OPENAI_MODEL=unchanged\n')
            finally:
                if child.poll() is None: child.kill(); child.wait()
                os.close(master)


if __name__ == '__main__':
    unittest.main()
