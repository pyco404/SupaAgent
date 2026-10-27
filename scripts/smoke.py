#!/usr/bin/env python3
"""Actual localhost API-process restart smoke test. Always offline; needs TEST_DATABASE_URL."""
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parents[1]
log_path = root / "artifacts/demo/http-process.log"
token = secrets.token_hex(32)
with socket.socket() as sock:
    sock.bind(("127.0.0.1", 0))
    port = sock.getsockname()[1]
origin = f"http://127.0.0.1:{port}"
env = dict(os.environ, DATABASE_URL=os.environ["TEST_DATABASE_URL"],
           SUPA_API_TOKEN=token, SUPA_PROVIDER="offline", SUPA_ALLOW_OPENAI="false",
           SUPA_BIND=f"127.0.0.1:{port}")
private_goal = "Review confidential local experiment outcomes"


def request(path, data=None, authenticated=True):
    headers = {"Content-Type": "application/json"}
    if authenticated:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(origin + path, headers=headers,
                                 data=None if data is None else json.dumps(data).encode())
    with urllib.request.urlopen(req, timeout=5) as response:
        return json.load(response)


def start(log):
    process = subprocess.Popen([str(root / "target/debug/supa-api")], env=env,
                               stdout=log, stderr=log, cwd=root)
    for _ in range(100):
        if process.poll() is not None:
            raise RuntimeError("local server exited; inspect sanitized log")
        try:
            request("/health")
            return process
        except (urllib.error.URLError, TimeoutError):
            time.sleep(0.05)
    process.kill()
    process.wait()
    raise RuntimeError("local server startup timeout")


def wait_cycle(agent):
    for _ in range(100):
        cycles = request(f"/agents/{agent}/cycles")
        if any(c["status"] == "completed" for c in cycles):
            return
        time.sleep(0.05)
    raise RuntimeError("autonomous scheduler did not complete a cycle")


log_path.parent.mkdir(parents=True, exist_ok=True)
with log_path.open("w") as log:
    process = start(log)
    try:
        try:
            request("/agents", authenticated=False)
            raise AssertionError("unauthenticated request accepted")
        except urllib.error.HTTPError as error:
            assert error.code == 401
        agent = request("/agents", {"name": "Supa process smoke", "capabilities": ["remember"],
                                    "config": {"interval_seconds": 1}})["id"]
        base = f"/agents/{agent}"
        request(base + "/goals", {"description": private_goal})
        request(base + "/simulated-funding", {"kind": "seed", "amount_microusd": 100000,
                                              "idempotency_key": str(uuid.uuid4()), "simulated": True})
        request(base + "/resume", {"reason": "operator authorizes offline process test"})
        wait_cycle(agent)
        request(base + "/pause", {"reason": "verify pause control"})
        assert request(base)["agent"]["lifecycle"] == "paused"
        request(base + "/resume", {"reason": "verify explicit resume"})
        request(base + "/emergency-stop", {"reason": "verify emergency suspension"})
        before = len(request(base + "/cycles"))
        process.kill()  # Actual abrupt process termination, no graceful cleanup.
        process.wait(timeout=5)
        process = start(log)
        assert request(base)["agent"]["lifecycle"] == "suspended"
        time.sleep(1.2)
        assert len(request(base + "/cycles")) == before
        assert request(base + "/financials")["simulated"] is True
        assert request(base + "/memory")
        assert request(base + "/inference")
        request(base + "/lifecycle", {"state": "terminated", "reason": "offline test cleanup"})
        result = {"mode": "offline", "actual_process_restart": True, "autonomous_scheduler": True,
                  "authentication": True, "paused_and_resumed": True, "emergency_stop_persisted": True,
                  "memory_persisted": True, "public_bind": False}
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGINT)
            process.wait(timeout=10)
contents = log_path.read_text()
assert token not in contents and env["DATABASE_URL"] not in contents and private_goal not in contents
for line in contents.splitlines():
    json.loads(line)
(root / "artifacts/demo/http-process-result.json").write_text(json.dumps(result, indent=2))
print(json.dumps(result))
