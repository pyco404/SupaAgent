#!/usr/bin/env python3
"""Offline custom Debian-format zlib candidate. Never installs packages or changes images.

Requires separately downloaded, hash-pinned Debian inputs. This is not an official
Debian rebuild/signature: debhelper is deliberately not installed on the host.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import subprocess
import tarfile

ROOT = Path(__file__).resolve().parent.parent
INPUTS = {
    'zlib_1.3.dfsg+really1.3.1.orig.tar.gz': '60dd315c07f616887caa029408308a018ace66e3d142726a97db164b3b8f69fb',
    'zlib_1.3.dfsg+really1.3.1-1.debian.tar.xz': '9ed525955ce9fb0c1b39be8ff98f73450dbfc6305a9a27e6149c8972d38a0a9e',
    'debian-uploader-public.gpg': '75c35c20d648ff6727ea34d6964ba729e7baef7c759dd00ff8f8b3901ed462e1',
    'trixie-zlib.dsc': 'ede2791e29c1d3b422f9208bdd7edf040c20445ea1e7453a72037576e64fa197',
    'trixie-zlib1g.deb': '015be740d6236ad114582dea500c1d907f29e16d6db00566ca32fb68d71ac90d',
}
PATCH = ROOT / 'deployment/patches/zlib-1.3.1-gzwrite-recovery.patch'
PATCH_SHA = '07a3032e1ebdeffbf78ffb8c00fe607b7601b891632f13f1505d9f90d87085a4'
VERSION = '1:1.3.dfsg+really1.3.1-1+supa1'
EPOCH = 1793836800
ENV = {'PATH': '/usr/bin:/bin', 'LC_ALL': 'C', 'TZ': 'UTC',
       'SOURCE_DATE_EPOCH': str(EPOCH), 'CC': 'gcc', 'ARFLAGS': 'rcD'}


def sha(path):
    with path.open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()


def run(args, cwd, env=None):
    result = subprocess.run([str(a) for a in args], cwd=cwd, env=env or ENV,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
    with (cwd / 'commands.log').open('ab') as log:
        log.write(('COMMAND ' + str(args[0]) + '\n').encode() + result.stdout)
    if result.returncode:
        raise RuntimeError('candidate_command_failed_see_private_commands_log')
    return result.stdout.decode()


def extract(archive, destination):
    with tarfile.open(archive) as handle:
        members = handle.getmembers()
        if (sum(m.size for m in members) > 32 * 1024**2 or any(
                Path(m.name).is_absolute() or '..' in Path(m.name).parts
                or not (m.isfile() or m.isdir()) for m in members)):
            raise ValueError('regular_bounded_source_archive_required')
        handle.extractall(destination, filter='data')


def exports(lib, cwd):
    data = run(['nm', '-D', '--defined-only', lib], cwd)
    return sorted(line.split()[-1] for line in data.splitlines() if len(line.split()) >= 3)


def prepare(inputs, output):
    if sha(PATCH) != PATCH_SHA:
        raise ValueError('reviewed_patch_hash_mismatch')
    for name, expected in INPUTS.items():
        path = inputs / name
        if path.is_symlink() or sha(path) != expected:
            raise ValueError('pinned_input_required')
    output.mkdir(mode=0o700, parents=True, exist_ok=False)
    signature = run(['gpgv', '--status-fd', '1', '--keyring', inputs / 'debian-uploader-public.gpg',
                     inputs / 'trixie-zlib.dsc'], output)
    if '[GNUPG:] VALIDSIG ADE668AA675718B59FE29FEA24D68B725D5487D0 ' not in signature:
        raise ValueError('expected_debian_source_signature_required')
    descriptor = (inputs / 'trixie-zlib.dsc').read_text()
    for filename in list(INPUTS)[:2]:
        if INPUTS[filename] not in descriptor or filename not in descriptor:
            raise ValueError('signed_source_checksum_missing')
    run(['dpkg-deb', '-x', inputs / 'trixie-zlib1g.deb', output / 'baseline'], output)
    baseline = output / 'baseline/usr/lib/x86_64-linux-gnu/libz.so.1.3.1'
    baseline_exports = exports(baseline, output)
    packages = []
    report = {'version': VERSION, 'source_sha256': INPUTS, 'patch_sha256': PATCH_SHA,
              'signature_verified': True,
              'signing_subkey': 'ADE668AA675718B59FE29FEA24D68B725D5487D0',
              'key_source': 'https://keyring.debian.org',
              'trust_limit': 'HTTPS official Debian keyring; no separate out-of-band fingerprint ceremony', 'official_debian_package': False,
              'scope': 'local custom package feasibility; not image security approval'}
    provenance = json.dumps(report, sort_keys=True) + '\n'
    for name in ('first', 'second'):
        case = output / name
        case.mkdir()
        extract(inputs / next(iter(INPUTS)), case)
        tree = case / 'zlib-1.3.1'
        extract(inputs / 'zlib_1.3.dfsg+really1.3.1-1.debian.tar.xz', tree)
        if (tree / 'debian/patches/series').read_text().strip():
            raise ValueError('unexpected_debian_patches_require_review')
        run(['patch', '--batch', '--fuzz=0', '-p1', '-i', PATCH], tree)
        env = dict(ENV, CFLAGS=f'-O2 -fPIC -D_REENTRANT -DUNALIGNED_OK -fstack-protector-strong -D_FORTIFY_SOURCE=2 -ffile-prefix-map={tree}=. -fdebug-prefix-map={tree}=.',
                   LDFLAGS='-Wl,-z,relro,-z,now')
        run(['./configure', '--shared', '--prefix=/usr'], tree, env)
        run(['make', '-j2'], tree, env)
        run(['make', 'check'], tree, env)
        lib = tree / 'libz.so.1.3.1'
        if exports(lib, case) != baseline_exports:
            raise ValueError('debian_versioned_export_set_changed')
        dynamic = run(['readelf', '-d', lib], case)
        if 'Library soname: [libz.so.1]' not in dynamic:
            raise ValueError('soname_changed')
        needed = re.findall(r'Shared library: \[(.*?)\]', dynamic)
        if needed != ['libc.so.6']:
            raise ValueError('unexpected_dependencies')
        versions = run(['readelf', '--version-info', lib], case)
        glibc = {tuple(map(int, v.split('.'))) for v in re.findall(r'GLIBC_([0-9.]+)', versions)}
        if not glibc or max(glibc) > (2, 14):
            raise ValueError('dependency_floor_exceeds_original_debian_package')
        pkg = case / 'package'
        libdir = pkg / 'usr/lib/x86_64-linux-gnu'
        libdir.mkdir(parents=True)
        shutil.copyfile(lib, libdir / lib.name)
        (libdir / 'libz.so.1').symlink_to(lib.name)
        control = pkg / 'DEBIAN'
        control.mkdir()
        # Preserve original dependency/conflict declarations; never impersonate Debian.
        metadata = run(['dpkg-deb', '-f', inputs / 'trixie-zlib1g.deb'], case)
        metadata = re.sub(r'^Source:.*$', 'Source: zlib (' + VERSION + ')', metadata, flags=re.M)
        metadata = re.sub(r'^Version:.*$', 'Version: ' + VERSION, metadata, flags=re.M)
        metadata = re.sub(r'^Maintainer:.*$', 'Maintainer: Supa Agency local candidate <noreply@localhost>', metadata, flags=re.M)
        metadata = re.sub(r'^Installed-Size:.*$', 'Installed-Size: ' + str((lib.stat().st_size + 1023) // 1024), metadata, flags=re.M)
        (control / 'control').write_text(metadata)
        (control / 'shlibs').write_text('libz 1 zlib1g (>= 1:1.2.3.3.dfsg-1)\n')
        (control / 'triggers').write_text('activate-noawait ldconfig\n')
        doc = pkg / 'usr/share/doc/zlib1g'
        doc.mkdir(parents=True)
        shutil.copyfile(tree / 'debian/copyright', doc / 'copyright')
        (doc / 'supa-backport.json').write_text(provenance)
        for path in sorted(pkg.rglob('*')):
            if not path.is_symlink():
                path.chmod(0o755 if path.is_dir() else 0o644)
            os.utime(path, (EPOCH, EPOCH), follow_symlinks=False)
        os.utime(pkg, (EPOCH, EPOCH))
        deb = case / 'zlib1g-supa1-amd64.deb'
        run(['dpkg-deb', '--root-owner-group', '-Zxz', '--build', pkg, deb], case)
        extracted = case / 'extracted'
        run(['dpkg-deb', '-x', deb, extracted], case)
        actual = extracted / 'usr/lib/x86_64-linux-gnu'
        run(['gcc', '-I', tree, ROOT / 'scripts/fixtures/zlib-gzwrite-recovery.c',
             actual / 'libz.so.1.3.1', '-o', case / 'probe'], case)
        runtime_env = dict(ENV, LD_LIBRARY_PATH=str(actual), LD_DEBUG='libs')
        loaded = run([case / 'probe'], case, runtime_env)
        if f'calling init: {actual}/libz.so.1' not in loaded:
            raise ValueError('extracted_package_library_not_loaded')
        packages.append(sha(deb))
        report[name] = {'deb_sha256': sha(deb), 'library_sha256': sha(actual / 'libz.so.1.3.1'),
                        'glibc_max': '.'.join(map(str, max(glibc))), 'needed': needed,
                        'versioned_exports': len(baseline_exports), 'package_dynamic_recovery_passed': True,
                        'loaded_library_path': str(actual / 'libz.so.1'), 'make_check_passed': True}
    if len(set(packages)) != 1:
        raise ValueError('package_not_reproducible')
    report['two_path_byte_reproducible'] = True
    report['toolchain'] = {tool: run(args, output).splitlines()[0] for tool, args in {
        'gcc': ['gcc', '--version'], 'dpkg-deb': ['dpkg-deb', '--version'], 'ld': ['ld', '--version']}.items()}
    (output / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--inputs', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    print(json.dumps(prepare(args.inputs.resolve(strict=True), args.output.resolve()), indent=2))
