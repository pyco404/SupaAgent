#!/usr/bin/env python3
"""Read-only local backup destination review. Never copies, uploads or starts PostgreSQL.

A different directory or partition is not an independent disk. Unknown topology,
network mounts, tmpfs and overlay storage fail closed and need separate review.
"""
import argparse
import json
import os
from pathlib import Path
import stat

ROOT=Path(__file__).resolve().parent.parent
SOURCE=ROOT/'postgres-data/infrastructure-v1/data'

def require(ok,code):
    if not ok:raise ValueError(code)

def physical_disks(device,sys_root=Path('/sys')):
    link=sys_root/'dev/block'/device
    require(link.exists(),'unverifiable_block_device')
    node=link.resolve()
    if (node/'partition').exists():node=node.parent
    slaves=list((node/'slaves').iterdir()) if (node/'slaves').exists() else []
    if slaves:
        devices=set()
        for child in slaves:
            devices|=physical_disks((child.resolve()/'dev').read_text().strip(),sys_root)
        return devices
    require('/virtual/' not in str(node) and (node/'device').exists(),'unverifiable_physical_disk')
    return {str(node)}

def review(destination,source=SOURCE,sys_root=Path('/sys')):
    source=Path(source);destination=Path(destination).absolute()
    require(destination.is_dir(),'destination_directory_required')
    for path in [destination,*destination.parents]:
        info=path.lstat()
        require(not stat.S_ISLNK(info.st_mode),'destination_symlink_rejected')
        require(not info.st_mode&0o022,'destination_ancestor_writable_by_others')
    info=destination.stat()
    require(info.st_uid==os.getuid() and stat.S_IMODE(info.st_mode)==0o700,'destination_requires_owner_only_0700')
    devices=[]
    for path in [source,destination]:
        dev=path.stat().st_dev
        devices.append(physical_disks(f'{os.major(dev)}:{os.minor(dev)}',sys_root))
    require(devices[0].isdisjoint(devices[1]),'same_physical_disk_rejected')
    return {'independent_physical_disk_verified':True,'destination':str(destination),'data_transferred':False,'upload_performed':False,'approval_required_before_copy':True,'encryption_and_restore_verification_required':True}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--destination',type=Path)
    args=parser.parse_args()
    if args.destination is None:
        print(json.dumps({'destination':None,'ready':False,'reason':'Independent destination remains unset; no files copied or uploaded.'}));return
    print(json.dumps(review(args.destination)))
if __name__=='__main__':
    try:main()
    except (ValueError,OSError):raise SystemExit('Independent backup destination rejected or unverifiable; no files copied.')
