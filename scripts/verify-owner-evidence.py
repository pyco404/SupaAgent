#!/usr/bin/env python3
"""Read-only proof review. This program cannot issue credentials, grants or approvals.

A key proves control, not identity. Its SHA-256 fingerprint and identity-check reference
must first be anchored by an independent verifier in a root-controlled trust record.
No trust-on-first-use or proposal-file fallback is supported.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time
import uuid

EPOCH='fd2f52e0-33fc-4c0d-893a-7b08b26ab748'
OWNER='6fd863e1-745d-4a06-b68e-eea69e854a69'
CUSTODIAN='e8c6ade3-917f-4fe4-a092-f8a58eccb487'
TRUST=Path('/etc/supa-agent/onboarding-authority.json')

def require(ok, code):
    if not ok: raise ValueError(code)

def canonical(value):
    return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=True).encode()

def private_file(path, uid):
    # Resolve no symlinks, including ancestors. Root and the containing directory
    # must not be writable by the requesting process or other users.
    path=Path(path).absolute()
    for parent in [path,*path.parents]:
        s=parent.lstat()
        require(not stat.S_ISLNK(s.st_mode),'symlink_not_permitted')
        require(s.st_uid==uid and not s.st_mode&0o022,'untrusted_path_permissions')
    require(path.is_file() and path.stat().st_size<=32768,'invalid_trust_record')

def verify_document(trust, challenge, keys, signatures, now=None):
    """Pure verification; callers cannot turn this result into an execution permit."""
    now=int(time.time()) if now is None else now
    require(set(challenge)=={'version','purpose','epoch_id','owner_principal','custodian_principal','nonce','issued_at','expires_at'},'challenge_fields')
    require(challenge['version']==1 and challenge['purpose']=='owner-provisioning-review-only','wrong_purpose')
    require(challenge['epoch_id']==EPOCH and challenge['owner_principal']==OWNER and challenge['custodian_principal']==CUSTODIAN,'identity_or_epoch_mismatch')
    require(str(uuid.UUID(challenge['nonce']))==challenge['nonce'],'invalid_nonce')
    issued,expiry=challenge['issued_at'],challenge['expires_at']
    require(type(issued) is int and type(expiry) is int and issued<=now<expiry and 0<expiry-issued<=900,'challenge_expired_or_invalid')
    require(trust['version']==1 and trust['epoch_id']==EPOCH and trust['expires_at']>now,'trust_expired_or_mismatched')
    fingerprints=[]
    for role,principal in [('owner',OWNER),('custodian',CUSTODIAN)]:
        anchor=trust[role]
        require(anchor['principal_id']==principal and anchor['identity_verification_reference'] and anchor['independent_control_verified'] is True,'identity_not_independently_verified')
        key=keys[role]
        require(len(key)<=8192 and len(signatures[role])==64,'invalid_ed25519_material')
        with tempfile.TemporaryDirectory(prefix='supa-proof-') as directory:
            d=Path(directory);os.chmod(d,0o700)
            for name,value in [('public.pem',key),('message',canonical(challenge)),('signature',signatures[role])]:
                (d/name).write_bytes(value);os.chmod(d/name,0o600)
            der=subprocess.run(['openssl','pkey','-pubin','-in',str(d/'public.pem'),'-outform','DER'],capture_output=True,check=True).stdout
            # RFC 8410 Ed25519 SubjectPublicKeyInfo: exact algorithm identifier + 32-byte public key.
            require(len(der)==44 and der.startswith(bytes.fromhex('302a300506032b6570032100')),'ed25519_required')
            fingerprint=hashlib.sha256(der).hexdigest()
            require(fingerprint==anchor['public_key_sha256'],'unanchored_key')
            subprocess.run(['openssl','pkeyutl','-verify','-pubin','-inkey',str(d/'public.pem'),'-rawin','-in',str(d/'message'),'-sigfile',str(d/'signature')],capture_output=True,check=True)
            fingerprints.append(fingerprint)
    require(fingerprints[0]!=fingerprints[1],'independent_keys_required')
    return {'evidence_valid':True,'challenge_sha256':hashlib.sha256(canonical(challenge)).hexdigest(),'nonce':challenge['nonce'],'expires_at':expiry,'grants_issued':0,'credentials_created':False,'execution_authorized':False,'next':'Independent operator must approve provisioning separately; record this nonce as consumed in the immutable provisioning audit. Revalidation never grants access.'}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--challenge',type=Path,required=True)
    for role in ['owner','custodian']:
        parser.add_argument('--'+role+'-public-key',type=Path,required=True)
        parser.add_argument('--'+role+'-signature',type=Path,required=True)
    args=parser.parse_args()
    require(os.geteuid()!=0,'run_as_unprivileged_reviewer')
    private_file(TRUST,0)
    trust=json.loads(TRUST.read_text())
    challenge=json.loads(args.challenge.read_text())
    print(json.dumps(verify_document(trust,challenge,{r:getattr(args,r+'_public_key').read_bytes() for r in ['owner','custodian']},{r:getattr(args,r+'_signature').read_bytes() for r in ['owner','custodian']})))

if __name__=='__main__':
    try:main()
    except (ValueError,KeyError,OSError,subprocess.SubprocessError):
        raise SystemExit('Owner verification unavailable or rejected; no access granted.')
