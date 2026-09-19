#!/usr/bin/env python3
"""Native process failure checks for the supplied local scaffold package."""
import argparse, hashlib, json, os, subprocess, sys, time
from pathlib import Path
from dev import smoke

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();binary=args.binary.resolve();root=args.output.resolve()
    root.mkdir(parents=True,exist_ok=False);results=[]
    fixture=Path(__file__).resolve().parents[1]/'fixtures/s0/orientation-6.jpg'
    source=fixture.read_bytes()
    # Actual hung child, not merely a mocked TimeoutExpired. exec prevents orphans.
    if os.name!='nt':
        helper=root/'hung-child';helper.write_text('#!/bin/sh\nexec sleep 10\n');helper.chmod(0o755)
        assert smoke(root/'hung','load',helper,timeout=.2)==1
        assert 'timed out' in json.loads((root/'hung/result.json').read_text())['error']
        results.append('Actual hung child killed and reaped; no success frame fabricated')
    obstacle=root/'not-a-directory';obstacle.write_text('preserve')
    failed=subprocess.run([str(binary),'--evidence-dir',str(obstacle/'evidence')],capture_output=True,text=True,timeout=5)
    assert failed.returncode==2 and 'Cannot create evidence directory' in failed.stderr
    assert obstacle.read_text()=='preserve'
    results.append('Evidence initialization failure is explicit and preserves existing files')
    # Normal viewing must survive a diagnostics initialization failure.
    with (root/'diagnostics-unavailable.log').open('w') as log:
        process=subprocess.Popen([str(binary),'--data-root',str(obstacle),'--open',str(fixture)],stdout=log,stderr=log)
        deadline=time.monotonic()+10
        try:
            while True:
                content=(root/'diagnostics-unavailable.log').read_text()
                if '"event":"decoded"' in content:break
                assert process.poll() is None,content
                assert time.monotonic()<deadline,'No decoded state'
                time.sleep(.01)
            assert 'viewing continues' in content
        finally:
            if process.poll() is None:process.terminate()
            process.wait(timeout=5)
    results.append('Normal viewing decoded successfully with unwritable diagnostics; test then terminated child')
    # Kill a live process after its first incremental event; prior records survive.
    isolated=root/'abrupt'
    with (root/'abrupt.log').open('w') as log:
        process=subprocess.Popen([str(binary),'--data-root',str(isolated),'--open',str(fixture)],stdout=log,stderr=log)
        deadline=time.monotonic()+10;events=isolated/'logs/events.jsonl'
        try:
            while not events.exists() or not events.read_text():
                assert process.poll() is None
                assert time.monotonic()<deadline
                time.sleep(.005)
        finally:
            if process.poll() is None:process.kill()
            process.wait(timeout=5)
    retained=[json.loads(line) for line in events.read_text().splitlines()]
    assert retained[0]['event']=='startup'
    assert not (isolated/'config').exists() and not (isolated/'cache').exists()
    assert fixture.read_bytes()==source
    results.append('Abrupt process termination retains incremental startup; no configuration/cache/source mutation')
    (root/'result.json').write_text(json.dumps({'status':'passed','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'checks':results},indent=2)+'\n')
    print('\n'.join(results))

if __name__=='__main__':main()
