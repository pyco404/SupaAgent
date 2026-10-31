#!/usr/bin/env python3
"""Network-free real GnuPG tests. ONLY generated synthetic data and disposable keys.

Never reads the primary database, backup dumps, ~/.gnupg or Railway credentials.
Private test keys are unprotected disposable fixtures, never approved recovery keys.
"""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest
import uuid

spec = importlib.util.spec_from_file_location('crypto', Path(__file__).with_name('backup-crypto.py'))
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class BackupCryptoTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix='supa-synthetic-openpgp-')
        cls.root = Path(cls.tmp.name)
        cls.homes = []
        cls.addClassCleanup(cls.cleanup)
        cls.recipient = cls.home('recipient')
        cls.signer = cls.home('signer')
        cls.public = cls.home('recipient-public')
        cls.verifier_home = cls.home('signer-public')
        cls.wrong = cls.home('wrong-signer')
        cls.recipient_pin = cls.generate(cls.recipient, 'Supa Synthetic Encryption', 'cert')
        cls.cmd(cls.recipient, ['--pinentry-mode', 'loopback', '--passphrase', '',
                               '--quick-add-key', cls.recipient_pin, 'cv25519', 'encr', '1d'])
        cls.subkey = cls.key_fingerprints(cls.recipient)[1]
        cls.signer_pin = cls.generate(cls.signer, 'Supa Synthetic Manifest', 'cert')
        cls.cmd(cls.signer, ['--pinentry-mode', 'loopback', '--passphrase', '',
                            '--quick-add-key', cls.signer_pin, 'ed25519', 'sign', '1d'])
        cls.signer_key = cls.key_fingerprints(cls.signer)[1]
        cls.wrong_pin = cls.generate(cls.wrong, 'Supa Synthetic Untrusted', 'sign')
        cls.recipient_export = cls.cmd(cls.recipient, ['--export', cls.recipient_pin]).stdout
        cls.signer_export = cls.cmd(cls.signer, ['--export', cls.signer_pin]).stdout
        cls.cmd(cls.public, ['--import'], cls.recipient_export)
        cls.cmd(cls.verifier_home, ['--import'], cls.signer_export)
        cls.plain = (b'SYNTHETIC ONLY: no actual database records.\n' * 4096)
        cls.cipher = m.encrypt_bytes(cls.public, cls.plain, cls.recipient_pin, cls.subkey)
        cls.snapshot = str(uuid.uuid4())
        cls.document = m.manifest(cls.cipher, cls.snapshot, cls.recipient_pin, cls.subkey,
                                 cls.signer_pin, cls.signer_key)
        cls.signature = cls.sign(cls.document)

    @classmethod
    def home(cls, name):
        home = cls.root / name
        home.mkdir(mode=0o700)
        cls.homes.append(home)
        return home

    @classmethod
    def cmd(cls, home, args, data=None, check=True):
        result = m.gpg(home, args, data)
        if check and result.returncode:
            raise AssertionError('Synthetic GnuPG operation failed; private output suppressed.')
        return result

    @classmethod
    def key_fingerprints(cls, home):
        out = cls.cmd(home, ['--with-colons', '--list-keys']).stdout.decode()
        return [line.split(':')[9] for line in out.splitlines() if line.startswith('fpr:')]

    @classmethod
    def generate(cls, home, label, usage):
        cls.cmd(home, ['--pinentry-mode', 'loopback', '--passphrase', '',
                       '--quick-generate-key', label, 'ed25519', usage, '1d'])
        return cls.key_fingerprints(home)[0]

    @classmethod
    def sign(cls, data, home=None, pin=None):
        return cls.cmd(home or cls.signer, ['--local-user', (pin or cls.signer_key) + '!',
                       '--digest-algo', 'SHA256', '--detach-sign'], data).stdout

    @classmethod
    def cleanup(cls):
        # Terminate only fixture-specific agents; never touch personal GnuPG state.
        for home in cls.homes:
            subprocess.run(['/usr/bin/gpgconf', '--homedir', str(home), '--kill', 'all'],
                           capture_output=True, timeout=10,
                           env={'PATH': '/usr/bin:/bin', 'HOME': str(cls.root)})
        cls.tmp.cleanup()

    def test_signed_ciphertext_and_real_aead_round_trip(self):
        result = self.verify_snapshot()
        self.assertTrue(result['verified'])
        self.assertFalse(result['plaintext_released'])
        self.assertFalse(result['execution_authorized'])
        recovered = self.cmd(self.recipient, ['--status-fd', '2', '--decrypt'], self.cipher)
        self.assertEqual(recovered.stdout, self.plain)
        # 0 = no MDC; 9 = AES256; 2 = OCB. Require successful process exit as well.
        self.assertIn(b'[GNUPG:] DECRYPTION_INFO 0 9 2', recovered.stderr)
        self.assertIn(b'[GNUPG:] DECRYPTION_OKAY', recovered.stderr)

    def verify_snapshot(self, **changes):
        values = {'home': type(self).verifier_home, 'document': self.document,
                  'signature': self.signature, 'ciphertext': self.cipher,
                  'snapshot_id': self.snapshot, 'recipient_primary': self.recipient_pin,
                  'recipient_subkey': self.subkey, 'signer_primary': self.signer_pin,
                  'signer_key': self.signer_key}
        values.update(changes)
        return m.verify_snapshot(**values)

    def test_ciphertext_tampering_and_truncation_rejected_before_decryption(self):
        corrupt = bytearray(self.cipher)
        corrupt[-20] ^= 1
        for data in [bytes(corrupt), self.cipher[:-16]]:
            with self.assertRaises(ValueError):
                self.verify_snapshot(ciphertext=data)
            decrypted = self.cmd(self.recipient, ['--decrypt'], data, check=False)
            self.assertNotEqual(decrypted.returncode, 0)
            # GPG can stream provisional plaintext on failure. It is never a usable restore.

    def test_changed_manifest_rejected(self):
        value = json.loads(self.document)
        value['created_at'] -= 1
        with self.assertRaisesRegex(ValueError, 'manifest_signature_rejected'):
            self.verify_snapshot(document=m.canonical(value))

    def test_wrong_signer_and_bad_signature_rejected(self):
        wrong = self.sign(self.document, self.wrong, self.wrong_pin)
        for signature in [wrong, self.signature[:-8], b'not a signature']:
            with self.assertRaises(ValueError):
                self.verify_snapshot(signature=signature)

    def test_short_wrong_and_unapproved_fingerprints_rejected(self):
        for pin in ['01234567', '0' * 40]:
            with self.assertRaises(ValueError):
                m.encrypt_bytes(self.public, b'synthetic', pin, self.subkey)
        with self.assertRaises(ValueError):
            self.verify_snapshot(signer_primary=self.wrong_pin, signer_key=self.wrong_pin)

    def test_wrong_recipient_cannot_decrypt(self):
        result = self.cmd(self.wrong, ['--decrypt'], self.cipher, check=False)
        self.assertNotEqual(result.returncode, 0)

    def test_actual_expired_certificate_rejected(self):
        home = self.home('expired-synthetic-key')
        self.cmd(home, ['--faked-system-time', str(int(time.time()) - 3 * 86400),
                        '--pinentry-mode', 'loopback', '--passphrase', '',
                        '--quick-generate-key', 'Supa Synthetic Expired', 'ed25519', 'sign', '1d'])
        pin = self.key_fingerprints(home)[0]
        exported = self.cmd(home, ['--export', pin]).stdout
        public = self.home('expired-public')
        self.cmd(public, ['--import'], exported)
        with self.assertRaisesRegex(ValueError, 'primary_key_unusable'):
            m.public_certificate(public, pin)

    def test_user_configuration_cannot_add_ambient_recipients(self):
        (self.public / 'gpg.conf').write_text('invalid-synthetic-option\nencrypt-to ' + self.wrong_pin + '\n')
        encrypted = m.encrypt_bytes(self.public, b'synthetic-no-ambient-config',
                                    self.recipient_pin, self.subkey)
        self.assertEqual(self.cmd(self.recipient, ['--decrypt'], encrypted).stdout,
                         b'synthetic-no-ambient-config')
        self.assertNotEqual(self.cmd(self.wrong, ['--decrypt'], encrypted, check=False).returncode, 0)

    def test_snapshot_replay_and_path_substitution_rejected(self):
        with self.assertRaises(ValueError):
            self.verify_snapshot(snapshot_id=str(uuid.uuid4()))
        value = json.loads(self.document)
        value['filename'] = '../../database.dump'
        document = m.canonical(value)
        with self.assertRaises(ValueError):
            self.verify_snapshot(document=document, signature=self.sign(document))

    def test_duplicate_unknown_and_noncanonical_manifest_rejected(self):
        cases = [self.document.replace(b'{', b'{"version":1,', 1),
                 json.dumps(json.loads(self.document), indent=2).encode(),
                 m.canonical({**json.loads(self.document), 'extra': 'rejected'})]
        for document in cases:
            with self.assertRaises(ValueError):
                self.verify_snapshot(document=document, signature=self.sign(document))

    def test_downgrade_and_malformed_limits_rejected_even_if_signed(self):
        for key, value in [('profile', 'cfb-no-integrity'), ('bytes', True),
                           ('bytes', m.MAX_BYTES * 2), ('sha256', 'x' * 64),
                           ('version', True), ('created_at', int(time.time()) + 3600)]:
            doc = m.canonical({**json.loads(self.document), key: value})
            with self.assertRaises(ValueError):
                self.verify_snapshot(document=doc, signature=self.sign(doc))

    def test_role_separation_and_key_capabilities(self):
        with self.assertRaises(ValueError):
            m.manifest(self.cipher, self.snapshot, self.recipient_pin, self.subkey,
                       self.recipient_pin, self.recipient_pin)
        with self.assertRaises(ValueError):
            m.encrypt_bytes(type(self).verifier_home, b'synthetic', self.signer_pin, self.signer_pin)
        keys = m.public_certificate(self.public, self.recipient_pin)
        for validity in ['r', 'e', 'i', 'd']:
            changed = copy.deepcopy(keys)
            next(k for k in changed if k['fingerprint'] == self.subkey)['validity'] = validity
            with self.assertRaises(ValueError):
                m.usable_key(changed, self.subkey, 'e', True)
        changed = copy.deepcopy(keys)
        next(k for k in changed if k['fingerprint'] == self.subkey)['expires'] = '1'
        with self.assertRaises(ValueError):
            m.usable_key(changed, self.subkey, 'e', True)

    def test_secret_material_and_insecure_or_symlink_homes_rejected(self):
        with self.assertRaisesRegex(ValueError, 'public_only_home_required'):
            m.encrypt_bytes(self.recipient, b'synthetic', self.recipient_pin, self.subkey)
        bad = self.home('bad-mode')
        bad.chmod(0o755)
        with self.assertRaises(ValueError):
            m.public_certificate(bad, self.recipient_pin)
        alias = self.root / 'home-alias'
        alias.symlink_to(self.public)
        with self.assertRaises(ValueError):
            m.public_certificate(alias, self.recipient_pin)

    def test_encryption_is_randomized_and_preserves_payload(self):
        other = m.encrypt_bytes(self.public, self.plain, self.recipient_pin, self.subkey)
        self.assertNotEqual(hashlib.sha256(other).digest(), hashlib.sha256(self.cipher).digest())
        self.assertEqual(self.cmd(self.recipient, ['--decrypt'], other).stdout, self.plain)

    def test_loss_of_original_key_homes_disposable_recovery(self):
        recovery = self.home('recovery-after-loss')
        verifier = self.home('fresh-public-verifier')
        self.cmd(verifier, ['--import'], self.signer_export)
        # Disposable synthetic recovery export only: never exports an operator's keys.
        secret = self.cmd(self.recipient, ['--export-secret-keys', self.recipient_pin]).stdout
        self.cmd(recovery, ['--import'], secret)
        self.assertTrue(self.verify_snapshot(home=verifier)['verified'])
        self.assertEqual(self.cmd(recovery, ['--decrypt'], self.cipher).stdout, self.plain)


if __name__ == '__main__':
    unittest.main()
