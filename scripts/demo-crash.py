#!/usr/bin/env python3
"""Kill only the child offline demo worker after it durably reserves a cycle."""
import json
from pathlib import Path
import subprocess
import time

marker = Path('artifacts/demo/crash-marker.json')
if marker.exists():
    raise SystemExit('Crash marker already exists; preserve artifacts and use a new demo directory for another run.')
with Path('artifacts/demo/crash-process.log').open('w') as log:
    worker = subprocess.Popen(['target/debug/examples/demo', 'crash'], stdout=log, stderr=log)
    try:
        for _ in range(100):
            if worker.poll() is not None:
                raise RuntimeError('Demo worker exited before reservation')
            if marker.exists():
                json.loads(marker.read_text())
                worker.kill()
                worker.wait(timeout=5)
                print('Offline demo worker killed after durable cycle and inference reservation.')
                break
            time.sleep(0.05)
        else:
            raise RuntimeError('Timed out waiting for durable reservation')
    finally:
        if worker.poll() is None:
            worker.kill()
            worker.wait(timeout=5)
