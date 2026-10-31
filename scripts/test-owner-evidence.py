#!/usr/bin/env python3
"""Synthetic identities only. Never loads the real proposal, trust path or credentials."""
import copy
import hashlib
import importlib.util
import subprocess
import tempfile
from pathlib import Path
import unittest
import uuid
spec=importlib.util.spec_from_file_location('review',Path(__file__).with_name('verify-owner-evidence.py'))
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class ProofTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.d=Path(self.tmp.name);self.keys={};self.signatures={}
        self.challenge={'version':1,'purpose':'owner-provisioning-review-only','epoch_id':m.EPOCH,'owner_principal':m.OWNER,'custodian_principal':m.CUSTODIAN,'nonce':str(uuid.uuid4()),'issued_at':1000,'expires_at':1100}
        self.trust={'version':1,'epoch_id':m.EPOCH,'expires_at':1200}
        (self.d/'message').write_bytes(m.canonical(self.challenge))
        for role,principal in [('owner',m.OWNER),('custodian',m.CUSTODIAN)]:
            private=self.d/(role+'.pem');public=self.d/(role+'.pub');signature=self.d/(role+'.sig')
            subprocess.run(['openssl','genpkey','-algorithm','ED25519','-out',str(private)],check=True,capture_output=True)
            subprocess.run(['openssl','pkey','-in',str(private),'-pubout','-out',str(public)],check=True,capture_output=True)
            der=subprocess.run(['openssl','pkey','-pubin','-in',str(public),'-outform','DER'],check=True,capture_output=True).stdout
            subprocess.run(['openssl','pkeyutl','-sign','-inkey',str(private),'-rawin','-in',str(self.d/'message'),'-out',str(signature)],check=True,capture_output=True)
            self.keys[role]=public.read_bytes();self.signatures[role]=signature.read_bytes()
            self.trust[role]={'principal_id':principal,'identity_verification_reference':'synthetic-independent-check','independent_control_verified':True,'public_key_sha256':hashlib.sha256(der).hexdigest()}
    def verify(self):return m.verify_document(self.trust,self.challenge,self.keys,self.signatures,1050)
    def test_valid_review_never_authorizes_execution_or_credentials(self):
        result=self.verify();self.assertTrue(result['evidence_valid']);self.assertFalse(result['execution_authorized']);self.assertEqual(result['grants_issued'],0)
        self.assertEqual(result,self.verify()) # Replay of a review is still no authorization.
    def test_wrong_epoch_principal_purpose_and_unknown_fields(self):
        original=copy.deepcopy(self.challenge)
        for key,value in [('epoch_id',str(uuid.uuid4())),('owner_principal',str(uuid.uuid4())),('purpose','activate'),('role','owner')]:
            self.challenge={**original,key:value}
            with self.assertRaises(ValueError):self.verify()
    def test_expired_future_and_overlong_challenges(self):
        for issued,expiry in [(900,1000),(1060,1100),(1000,3000)]:
            self.challenge.update(issued_at=issued,expires_at=expiry)
            with self.assertRaises(ValueError):self.verify()
    def test_unverified_name_or_self_submitted_key_is_insufficient(self):
        for role in ['owner','custodian']:
            original=copy.deepcopy(self.trust)
            self.trust[role]['independent_control_verified']=False
            with self.assertRaises(ValueError):self.verify()
            self.trust=copy.deepcopy(original);self.trust[role]['public_key_sha256']='0'*64
            with self.assertRaises(ValueError):self.verify()
            self.trust=original
    def test_tampered_signature_nonce_and_separate_keys(self):
        self.signatures['owner']=b'0'*64
        with self.assertRaises(subprocess.SubprocessError):self.verify()
        self.setUp();self.challenge['nonce']=str(uuid.uuid4())
        with self.assertRaises(subprocess.SubprocessError):self.verify()
    def test_missing_or_untrusted_anchor_fails_closed(self):
        with self.assertRaises((OSError,ValueError)):m.private_file(self.d/'absent',0)
        file=self.d/'anchor';file.write_text('{}');file.chmod(0o666)
        with self.assertRaises(ValueError):m.private_file(file,0)
if __name__=='__main__':unittest.main()
