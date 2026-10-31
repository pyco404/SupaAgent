#!/usr/bin/env python3
"""Offline, bounded OpenPGP preparation primitives; no production execution CLI.

Only explicitly supplied private 0700 GnuPG homes are used. Encryption/verifying
homes must contain public keys only. Fingerprint arguments are cryptographic pins,
NOT evidence of independent approval. Real export, trust installation, external
signing, decryption and upload require separately approved orchestration.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import tempfile
import time
import uuid

PROFILE = 'gnupg-openpgp-aes256-ocb-v1'
MAX_BYTES = 16 * 1024 * 1024
MANIFEST_FIELDS = {'version', 'snapshot_id', 'created_at', 'filename', 'bytes',
                   'sha256', 'profile', 'recipient_primary', 'recipient_subkey',
                   'signer_primary', 'signer_key'}


def require(ok, code):
    if not ok:
        raise ValueError(code)


def fingerprint(value):
    require(isinstance(value, str) and re.fullmatch(r'(?:[A-F0-9]{40}|[A-F0-9]{64})', value),
            'full_uppercase_fingerprint_required')
    return value


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True) + '\n').encode()


def home_path(home):
    home = Path(home).absolute()
    for path in [home, *home.parents]:
        require(not path.is_symlink(), 'key_home_symlink_rejected')
    info = home.stat()
    require(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid()
            and stat.S_IMODE(info.st_mode) == 0o700, 'private_key_home_required')
    return home


def gpg(home, arguments, data=None):
    home = home_path(home)
    command = ['/usr/bin/gpg', '--no-options', '--homedir', str(home), '--batch',
               '--no-tty', '--no-auto-key-retrieve', '--no-auto-key-import',
               '--auto-key-locate', 'clear', '--pinentry-mode', 'error', *arguments]
    # No inherited GPG/agent, proxy, cloud, DB or application settings.
    env = {'PATH': '/usr/bin:/bin', 'HOME': str(home), 'GNUPGHOME': str(home), 'LC_ALL': 'C'}
    try:
        return subprocess.run(command, input=data, capture_output=True, timeout=45, env=env)
    except (OSError, subprocess.TimeoutExpired):
        raise ValueError('openpgp_process_unavailable') from None


def public_certificate(home, primary):
    """Reject mixed certificates, secret material, expired or revoked keys."""
    primary = fingerprint(primary)
    secret = gpg(home, ['--with-colons', '--list-secret-keys'])
    require(secret.returncode == 0 and not any(line.startswith((b'sec:', b'ssb:'))
            for line in secret.stdout.splitlines()), 'public_only_home_required')
    result = gpg(home, ['--with-colons', '--fixed-list-mode', '--list-keys'])
    require(result.returncode == 0, 'public_key_listing_failed')
    keys = []
    for line in result.stdout.decode('utf-8', errors='strict').splitlines():
        row = line.split(':')
        if row[0] in {'pub', 'sub'}:
            require(len(row) > 11, 'invalid_key_listing')
            keys.append({'kind': row[0], 'validity': row[1], 'expires': row[6],
                         'capabilities': row[11], 'bits': row[2], 'algorithm': row[3],
                         'fingerprint': None})
        elif row[0] == 'fpr' and keys and keys[-1]['fingerprint'] is None:
            keys[-1]['fingerprint'] = fingerprint(row[9])
    primaries = [k for k in keys if k['kind'] == 'pub']
    require(len(primaries) == 1 and primaries[0]['fingerprint'] == primary,
            'certificate_pin_mismatch')
    now = int(time.time())
    root = primaries[0]
    require(root['validity'] not in {'r', 'e', 'i', 'd'}
            and 'D' not in root['capabilities']
            and (not root['expires'] or int(root['expires']) > now), 'primary_key_unusable')
    return keys


def usable_key(keys, pin, capability, subkey=False):
    pin = fingerprint(pin)
    matches = [k for k in keys if k['fingerprint'] == pin]
    require(len(matches) == 1, 'key_pin_mismatch')
    key = matches[0]
    require(not subkey or key['kind'] == 'sub', 'encryption_subkey_required')
    require(key['bits'] == '255' and key['algorithm'] == {'e': '18', 's': '22'}[capability],
            'dedicated_curve25519_profile_required')
    require(key['validity'] not in {'r', 'e', 'i', 'd'}
            and 'D' not in key['capabilities'] and capability in key['capabilities']
            and (not key['expires'] or int(key['expires']) > int(time.time())),
            'key_capability_or_validity_rejected')


def encrypt_bytes(home, plaintext, recipient_primary, recipient_subkey):
    """In-memory preparation helper, capped at 16 MiB. No file/source discovery."""
    require(isinstance(plaintext, bytes) and 0 < len(plaintext) <= MAX_BYTES,
            'plaintext_size_rejected')
    keys = public_certificate(home, recipient_primary)
    usable_key(keys, recipient_subkey, 'e', subkey=True)
    # A full independently approved fingerprint must precede use of this anchor.
    # No broad --trust-model always, ambient recipient or user configuration.
    result = gpg(home, ['--trusted-key', recipient_primary, '--no-encrypt-to',
                       '--recipient', recipient_subkey + '!', '--cipher-algo', 'AES256',
                       '--force-ocb', '--compress-algo', 'none', '--set-filename', '',
                       '--status-fd', '2', '--encrypt'], plaintext)
    beginnings = [line.split()[2:] for line in result.stderr.splitlines()
                  if line.startswith(b'[GNUPG:] BEGIN_ENCRYPTION ')]
    require(result.returncode == 0 and beginnings == [[b'0', b'9', b'2']]
            and b'[GNUPG:] END_ENCRYPTION' in result.stderr.splitlines()
            and 0 < len(result.stdout) <= MAX_BYTES + 65536, 'authenticated_encryption_failed')
    return result.stdout


def manifest(ciphertext, snapshot_id, recipient_primary, recipient_subkey,
             signer_primary, signer_key):
    value = {'version': 1, 'snapshot_id': snapshot_id, 'created_at': int(time.time()),
             'filename': snapshot_id + '.pgbundle.gpg', 'bytes': len(ciphertext),
             'sha256': hashlib.sha256(ciphertext).hexdigest(), 'profile': PROFILE,
             'recipient_primary': recipient_primary, 'recipient_subkey': recipient_subkey,
             'signer_primary': signer_primary, 'signer_key': signer_key}
    validate_manifest(value, snapshot_id, recipient_primary, recipient_subkey,
                      signer_primary, signer_key)
    return canonical(value)


def validate_manifest(value, snapshot_id, recipient_primary, recipient_subkey,
                      signer_primary, signer_key):
    require(type(value) is dict and set(value) == MANIFEST_FIELDS, 'manifest_fields_rejected')
    require(isinstance(snapshot_id, str) and str(uuid.UUID(snapshot_id)) == snapshot_id,
            'canonical_snapshot_uuid_required')
    pins = [fingerprint(x) for x in [recipient_primary, recipient_subkey, signer_primary, signer_key]]
    require(pins[0] != pins[1] and not set(pins[:2]).intersection(pins[2:]),
            'separate_encryption_and_signing_identities_required')
    require(type(value['version']) is int and value['version'] == 1
            and value['profile'] == PROFILE, 'manifest_profile_rejected')
    require(value['snapshot_id'] == snapshot_id
            and value['filename'] == snapshot_id + '.pgbundle.gpg', 'snapshot_context_rejected')
    require([value[k] for k in ['recipient_primary', 'recipient_subkey', 'signer_primary', 'signer_key']]
            == pins, 'manifest_key_binding_rejected')
    require(type(value['created_at']) is int and 0 < value['created_at'] <= int(time.time()) + 60,
            'manifest_time_rejected')
    require(type(value['bytes']) is int and 0 < value['bytes'] <= MAX_BYTES + 65536
            and isinstance(value['sha256'], str) and re.fullmatch('[0-9a-f]{64}', value['sha256']),
            'manifest_size_or_digest_rejected')


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, 'duplicate_json_field')
        result[key] = value
    return result


def verify_snapshot(home, document, signature, ciphertext, snapshot_id,
                    recipient_primary, recipient_subkey, signer_primary, signer_key):
    """Authenticate manifest and ciphertext BEFORE any external decryption/restore.

    Expected snapshot UUID and fingerprints must come from approved independent
    inventory, not from this manifest. Valid old snapshots are not automatically latest.
    """
    require(isinstance(document, bytes) and len(document) <= 32768
            and isinstance(signature, bytes) and 0 < len(signature) <= 16384,
            'manifest_or_signature_size_rejected')
    require(isinstance(ciphertext, bytes) and 0 < len(ciphertext) <= MAX_BYTES + 65536,
            'ciphertext_size_rejected')
    value = json.loads(document, object_pairs_hook=unique_object)
    require(canonical(value) == document, 'noncanonical_manifest_rejected')
    validate_manifest(value, snapshot_id, recipient_primary, recipient_subkey,
                      signer_primary, signer_key)
    keys = public_certificate(home, signer_primary)
    usable_key(keys, signer_key, 's')
    with tempfile.TemporaryDirectory(prefix='verify-', dir=home_path(home)) as temp:
        folder = Path(temp)
        for name, data in [('manifest.json', document), ('manifest.sig', signature)]:
            path = folder / name
            path.write_bytes(data)
            path.chmod(0o600)
        result = gpg(home, ['--status-fd', '1', '--verify', str(folder / 'manifest.sig'),
                           str(folder / 'manifest.json')])
    require(result.returncode == 0, 'manifest_signature_rejected')
    statuses = {line.split()[1] for line in result.stdout.decode().splitlines()
                if line.startswith('[GNUPG:] ') and len(line.split()) > 1}
    require(not statuses.intersection({'BADSIG', 'ERRSIG', 'EXPSIG', 'EXPKEYSIG',
            'REVKEYSIG', 'KEYEXPIRED', 'SIGEXPIRED', 'FAILURE', 'ERROR', 'NODATA',
            'NO_PUBKEY'}), 'unusable_signature_rejected')
    valid = [line.split()[2:] for line in result.stdout.decode().splitlines()
             if line.startswith('[GNUPG:] VALIDSIG ')]
    require(len(valid) == 1 and len(valid[0]) >= 10 and valid[0][0] == signer_key
            and valid[0][-1] == signer_primary and valid[0][7] in {'8', '9', '10'}
            and valid[0][8] == '00',
            'manifest_signer_or_digest_rejected')
    require(len(ciphertext) == value['bytes']
            and hashlib.sha256(ciphertext).hexdigest() == value['sha256'],
            'ciphertext_integrity_rejected')
    return {'verified': True, 'snapshot_id': snapshot_id, 'profile': PROFILE,
            'plaintext_released': False, 'execution_authorized': False}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    print(json.dumps({'mode': 'preparation_library_only', 'real_key_generation': False,
                      'real_backup_encryption': False, 'uploads': False,
                      'synthetic_validation': 'python3 scripts/test-backup-crypto.py'}))
