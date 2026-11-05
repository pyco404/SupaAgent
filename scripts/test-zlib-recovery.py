#!/usr/bin/env python3
"""Offline, isolated upstream zlib regression; never modifies application images."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parent.parent
ARCHIVE_SHA256 = '9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23'
FLAGS = '-O1 -g -fsanitize=address,undefined -fno-omit-frame-pointer'
ENV = {'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'TZ': 'UTC', 'CC': 'gcc', 'CFLAGS': FLAGS,
       'ASAN_OPTIONS': 'detect_leaks=0:abort_on_error=1:halt_on_error=1',
       'UBSAN_OPTIONS': 'halt_on_error=1:print_stacktrace=1'}


def execute(args, cwd, log, expected_success=True):
    result = subprocess.run(args, cwd=cwd, env=ENV, capture_output=True, timeout=120)
    log.write_bytes(result.stdout + result.stderr)
    if expected_success and result.returncode:
        raise RuntimeError('local_regression_command_failed_see_synthetic_log')
    return result


def run(archive, output):
    if archive.is_symlink() or not archive.is_file():
        raise ValueError('regular_upstream_archive_required')
    with archive.open('rb') as source:
        if hashlib.file_digest(source, 'sha256').hexdigest() != ARCHIVE_SHA256:
            raise ValueError('approved_official_upstream_archive_required')
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    report = {'upstream_archive_sha256': ARCHIVE_SHA256, 'cases': {},
              'application_images_changed': False, 'network_requests': 0, 'database_operations': 0,
              'scope': 'upstream-fix feasibility only; not Debian package or image validation'}
    symbols = {}
    for name in ('original', 'backport'):
        case = output / name
        case.mkdir(mode=0o700)
        with tarfile.open(archive) as source:
            members = source.getmembers()
            if (sum(m.size for m in members) > 32 * 1024**2
                    or any(not (m.name.startswith('zlib-1.3.1/') or (m.name == 'zlib-1.3.1' and m.isdir()))
                           or '..' in Path(m.name).parts or not (m.isfile() or m.isdir())
                           or m.issym() or m.islnk() for m in members)):
                raise ValueError('bounded_regular_upstream_tree_required')
            source.extractall(case, filter='data')
        tree = case / 'zlib-1.3.1'
        original = (tree / 'gzwrite.c').read_bytes()
        if name == 'backport':
            execute(['patch', '--batch', '--fuzz=0', '-p1', '-i', str(ROOT / 'deployment/patches/zlib-1.3.1-gzwrite-recovery.patch')],
                    tree, case / 'patch.log')
            patched = (tree / 'gzwrite.c').read_text()
            if patched.count('state->strm.avail_in = 0;\n                state->strm.next_in = state->in;') != 1:
                raise ValueError('exact_upstream_error_reset_required')
        execute(['./configure', '--static'], tree, case / 'configure.log')
        execute(['make', '-j2'], tree, case / 'build.log')
        execute(['make', 'check'], tree, case / 'upstream-check.log')
        execute(['gcc', *FLAGS.split(), '-I', str(tree), str(ROOT / 'scripts/fixtures/zlib-gzwrite-recovery.c'),
                 str(tree / 'libz.a'), '-o', str(case / 'probe')], tree, case / 'compile.log')
        outcome = execute([str(case / 'probe')], case, case / 'probe.log', expected_success=False)
        log = (case / 'probe.log').read_text(errors='replace')
        if name == 'original':
            if outcome.returncode == 0 or 'AddressSanitizer' not in log or 'gzwrite.c' not in log:
                raise RuntimeError('vulnerable_control_not_reproduced')
        elif outcome.returncode != 0 or 'gzwrite recovery and normal gzip roundtrip passed' not in log:
            raise RuntimeError('backport_recovery_regression_failed')
        defined = execute(['nm', '-g', '--defined-only', str(tree / 'libz.a')], tree, case / 'symbols.log')
        symbols[name] = {line.split()[-1] for line in defined.stdout.decode().splitlines() if len(line.split()) == 3}
        report['cases'][name] = {'exit_code': outcome.returncode, 'upstream_make_check_passed': True,
                                 'sanitizer_failure_expected': name == 'original',
                                 'gzwrite_source_sha256': hashlib.sha256((tree / 'gzwrite.c').read_bytes()).hexdigest()}
    if symbols['original'] != symbols['backport']:
        raise RuntimeError('public_static_symbol_set_changed')
    report['static_exported_symbol_names_unchanged'] = True
    report['passed'] = True
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    print(json.dumps(run(args.archive.resolve(strict=True), args.output.resolve())))
