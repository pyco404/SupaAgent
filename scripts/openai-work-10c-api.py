#!/usr/bin/env python3
"""Exact Milestone 10C launcher. Start is locked; paid execution needs fresh exact consent."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('reviewed_openai_launcher', ROOT / 'scripts/openai-work-api.py')
api = importlib.util.module_from_spec(spec)
spec.loader.exec_module(api)
api.PLAN = ROOT / 'config/openai-work-evaluation-10c.json'
api.SCOPE = '3c3e5a2f689f6af3347b78c9c19f639842ae6ec1472aea61ace5c853287471ba'
api.DATA = Path('/tmp/supa-m10a-pg-8c6356653dc2/data')
api.SOCKET = api.DATA.parent / 's'
api.DATABASE = 'supa_m10a_8c6356653dc2_test'
api.SYSTEM_IDENTIFIER = '7703596362529484555'
api.JOB_ID = '7bf3067c-ea30-4557-8c70-c06aeb1c429b'
api.ROLE = 'supa_openai_10c_eval'
api.OUT = ROOT / 'artifacts/milestone-10c/runtime'
api.PRIVATE = api.OUT / 'runtime.json'
api.BINARY = ROOT / 'target/claude-validation/debug/examples/openai_work_10c_api'
BINARY_SHA256 = '0fafa6390359009af238772d41f9dfed8bb2704f064823b5c3f65ff8252ce24d'
_original_frozen = api.frozen


def frozen():
    plan = _original_frozen()
    api.require(not api.BINARY.is_symlink() and api.BINARY.is_file(), 'reviewed_binary_required')
    api.require(hashlib.sha256(api.BINARY.read_bytes()).hexdigest() == BINARY_SHA256, 'reviewed_binary_changed')
    for field, path in [('provider_source_sha256', 'crates/providers/src/work.rs'), ('validator_source_sha256', 'crates/domain/src/live_work.rs')]:
        api.require(hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == plan['implementation'][field], 'reviewed_implementation_changed')
    capacity = os.statvfs(api.ROOT)
    api.require(capacity.f_bavail * capacity.f_frsize >= 1024 ** 3, 'minimum_one_gib_free_required')
    return plan


api.frozen = frozen
if __name__ == '__main__':
    try:
        api.main()
    except Exception as error:
        code = str(error) if isinstance(error, ValueError) else 'scoped_10c_api_failed_without_private_details'
        print(json.dumps({'error': code}), file=sys.stderr)
        raise SystemExit(1)
