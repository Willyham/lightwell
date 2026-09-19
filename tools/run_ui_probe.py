#!/usr/bin/env python3
"""Bounded native trial launch; a captured frame is not complete S0 acceptance."""
import argparse
import hashlib
import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('candidate', choices=['iced', 'egui'])
parser.add_argument('--fixture', type=Path, default=ROOT/'fixtures/s0/orientation-6.jpg')
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
output = args.output.resolve()
capture = output / 'window.png'
if capture.exists():
    raise SystemExit('Use a fresh output directory; refusing stale capture evidence')
fixture = args.fixture.resolve()
source_hash = hashlib.sha256(fixture.read_bytes()).hexdigest()
command = [str(ROOT/f'probes/s0/target/release/{args.candidate}-viewer'), str(fixture), str(capture)]
start = time.monotonic()
with (output/'process.log').open('w') as log:
    try:
        result = subprocess.run(command, stdout=log, stderr=log, timeout=30)
        status = 'captured' if result.returncode == 0 and capture.exists() else 'failed'
        code = result.returncode
    except subprocess.TimeoutExpired:
        status, code = 'timeout', None
unchanged = hashlib.sha256(fixture.read_bytes()).hexdigest() == source_hash
if not unchanged:
    status = 'failed'
result = {'candidate':args.candidate, 'status':status, 'exit_code':code, 'elapsed_seconds':time.monotonic()-start, 'fixture':fixture.name, 'fixture_sha256':source_hash, 'source_unchanged':unchanged, 'capture_provenance':'window-renderer-readback' if capture.exists() else None, 'pixel_verification':'pending' if capture.exists() else 'unavailable', 'native_dialog_resize_verification':'not-performed', 'command':command}
(output/'result.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
raise SystemExit(0 if status=='captured' else 1)
