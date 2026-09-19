#!/usr/bin/env python3
"""Native macOS initial baseline. Launches only the supplied local package and synthetic fixtures."""
import argparse, hashlib, json, platform, statistics, subprocess, time
from pathlib import Path
from check_probe_capture import check as pixel_check
ROOT=Path(__file__).resolve().parents[1]

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--samples',type=int,default=30)
    args=p.parse_args()
    if args.output.exists():raise ValueError('Output must be new')
    args.output.mkdir(parents=True)
    binary=args.binary.resolve()
    report={'platform':platform.platform(),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'lockfile_sha256':hashlib.sha256((ROOT/'Cargo.lock').read_bytes()).hexdigest(),'method':'Application-cold launches; filesystem cache not purged. Launch-to-frame is an observation upper bound (50 ms polling, or process exit for fast runs), including rendering/readback but not display scanout. RSS sampled every ~50 ms; GPU memory not measured separately. All failures retained.','runs':[]}
    def save(): (args.output/'measurements.json').write_text(json.dumps(report,indent=2)+'\n')
    for name in ['empty','24mp','60mp','repeated60mp']:
        for i in range(1 if name=='repeated60mp' else args.samples):
            evidence=(args.output/f'{name}-{i:02}').resolve()
            command=[str(binary),'--evidence-dir',str(evidence)]
            if name!='empty':
                source=ROOT/'fixtures/generated'/('60mp.jpg' if name=='repeated60mp' else f'{name}.jpg')
                before=hashlib.sha256(source.read_bytes()).hexdigest()
                for _ in range(16 if name=='repeated60mp' else 1):command+=['--open',str(source)]
            started=time.monotonic();peak=0;rss=[];first_frame=None
            with (args.output/f'{name}-{i:02}.log').open('w') as log:
                process=subprocess.Popen(command,stdout=log,stderr=log)
                while process.poll() is None:
                    if time.monotonic()-started>35:
                        process.kill();process.wait();raise RuntimeError('Benchmark deadline exceeded')
                    state=subprocess.run(['ps','-o','rss=','-p',str(process.pid)],capture_output=True,text=True).stdout.strip()
                    if state:
                        value=int(state);peak=max(peak,value);rss.append([round(time.monotonic()-started,3),value])
                    events_path=evidence/'events.jsonl'
                    if first_frame is None and events_path.exists() and 'frame_captured' in events_path.read_text():first_frame=(time.monotonic()-started)*1000
                    time.sleep(.05)
            if first_frame is None and (evidence/'result.json').exists():first_frame=(time.monotonic()-started)*1000
            row={'workload':name,'index':i,'exit_code':process.returncode,'launch_to_observed_frame_ms':first_frame,'sampled_peak_rss_mib':peak/1024,'rss_samples':rss}
            report['runs'].append(row);save()
            if process.returncode:raise RuntimeError(f'{name} failed; see retained log')
            events=[json.loads(x) for x in events_path.read_text().splitlines()]
            row['upload_ms']=[e['detail']['upload_ms'] for e in events if e['event']=='render_ready']
            row['decode_ms']=[e['detail']['decode_ms'] for e in events if e['event']=='decoded']
            row['request_to_capture_ms']=[e['detail']['request_to_capture_ms'] for e in events if e['event']=='frame_captured']
            result=json.loads((evidence/'result.json').read_text())
            row['backend']=result['frames'][-1]['state']['backend']
            row['physical_size']=result['frames'][-1]['physical_size'];row['scale']=result['frames'][-1]['scale']
            assert len(result['frames'])==(16 if name=='repeated60mp' else 1)
            if name!='empty':
                row['pixel_checks']=[pixel_check(evidence/frame['file'],orientation=1,source_aspect=(5/3 if name in ('60mp','repeated60mp') else 3/2)) for frame in result['frames']]
                assert hashlib.sha256(source.read_bytes()).hexdigest()==before
                assert all(frame['state']['phase']=='ready' for frame in result['frames'])
            save()
        print(f'Finished {name}',flush=True)
    # Ordinary viewing has no timer/frame subscription after decoding settles.
    with (args.output/'idle.log').open('w') as log:
        data=(args.output/'idle-data').resolve()
        process=subprocess.Popen([str(binary),'--data-root',str(data),'--open',str(ROOT/'fixtures/generated/60mp.jpg')],stdout=log,stderr=log)
        try:
            deadline=time.monotonic()+10
            events=data/'logs/events.jsonl'
            while not events.exists() or '"event":"render_ready"' not in events.read_text():
                assert process.poll() is None and time.monotonic()<deadline
                time.sleep(.05)
            time.sleep(1)
            def usage():
                raw=subprocess.run(['ps','-o','time=,rss=','-p',str(process.pid)],capture_output=True,text=True,check=True).stdout.split()
                minutes,seconds=raw[0].split(':')
                return float(minutes)*60+float(seconds),int(raw[1])/1024
            before=usage();start=time.monotonic();samples=[]
            while time.monotonic()-start<30:
                samples.append(usage());time.sleep(.5)
            after=usage();elapsed=time.monotonic()-start
            report['idle']={'duration_s':elapsed,'cpu_percent_one_core':(after[0]-before[0])/elapsed*100,'rss_mib_start':before[1],'rss_mib_end':after[1],'rss_mib_peak':max(v[1] for v in samples),'method':'ps cumulative CPU delta over 30 seconds after render_ready plus 1 second settle; ordinary mode; child terminated after sample, not a clean-close check'}
            save()
        finally:
            if process.poll() is None:process.terminate()
            process.wait(timeout=5)
    report['summary']={}
    for name in ['empty','24mp','60mp']:
        rows=[r for r in report['runs'] if r['workload']==name]
        report['summary'][name]={}
        for key in ['launch_to_observed_frame_ms','sampled_peak_rss_mib']:
            values=sorted(r[key] for r in rows)
            report['summary'][name][key]={'median':statistics.median(values),'p95':values[min(len(values)-1,int(len(values)*.95))],'first':values[0] if key=='sampled_peak_rss_mib' else rows[0][key]}
        for key in ['decode_ms','upload_ms','request_to_capture_ms']:
            values=sorted(v for r in rows for v in r[key])
            if values:report['summary'][name][key]={'median':statistics.median(values),'p95':values[min(len(values)-1,int(len(values)*.95))]}
    save();print(json.dumps(report['summary'],indent=2))

if __name__=='__main__':main()
