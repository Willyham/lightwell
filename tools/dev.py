#!/usr/bin/env python3
"""Cross-platform development commands. Requires Python 3.10+; never installs tools."""
import argparse
import hashlib
import json
import os
import platform
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXE = '.exe' if os.name == 'nt' else ''

def run(*args, **kwargs):
    return subprocess.run([str(v) for v in args], cwd=ROOT, check=True, **kwargs)


def doctor():
    gaps=[]
    print(f'Host: {platform.system()} {platform.release()} {platform.machine()}')
    for program in ['rustup','cargo','rustc']:
        found=shutil.which(program)
        print(f'{program}: {found or "MISSING"}')
        if not found:gaps.append(program)
    if not gaps:
        installed=run('rustup','toolchain','list',capture_output=True,text=True).stdout
        if not any(line.startswith('1.94.0-') for line in installed.splitlines()):
            gaps.append('Rust 1.94.0 (see development.md; Doctor will not install it)')
        else:
            for args in [('rustc','--version'),('cargo','fmt','--version'),('cargo','clippy','--version')]:run(*args)
    if sys.platform=='darwin':
        if not shutil.which('clang'):gaps.append('Xcode Command Line Tools')
        print('Graphics: native Metal; unlocked desktop required for UI checks (not proven by Doctor).')
    elif sys.platform.startswith('linux'):
        for library in ['pkg-config','cc']:
            if not shutil.which(library):gaps.append(library)
        print('Display:',os.environ.get('WAYLAND_DISPLAY') or os.environ.get('DISPLAY') or 'MISSING')
        if not (os.environ.get('WAYLAND_DISPLAY') or os.environ.get('DISPLAY')):gaps.append('graphical desktop for smoke')
        print('Runtime: Vulkan driver, X11/Wayland libraries and desktop file chooser portal required.')
    else:print('Runtime: MSVC C++ Build Tools / Windows SDK and DX12 desktop required.')
    for gap in gaps:print('GAP:',gap)
    return 1 if gaps else 0


def cargo(operation,release=False):
    operations={
        'build':['build','--locked','--package','lightwell-app'],
        'test':['test','--locked','--workspace'],
        'fmt':['fmt','--all','--','--check'],
        'lint':['clippy','--locked','--workspace','--all-targets','--','-D','warnings'],
    }
    args=operations[operation]
    if release:args=args+['--release']
    run('cargo',*args)


def verify_smoke(evidence, scenario, source_count):
    app=json.loads((evidence/'result.json').read_text(encoding="utf-8"))
    events=[json.loads(line) for line in (evidence/'events.jsonl').read_text(encoding="utf-8").splitlines()]
    assert events[0]['event']=='startup' and events[-1]['event']=='shutdown','Missing lifecycle events'
    assert app['status']=='captured','Unsuccessful app result'
    assert app.get('run_id') and all(e.get('run_id')==app['run_id'] for e in events),'Wrong log run identity'
    assert len(app['frames'])==max(1,source_count),'Missing/stale frames'
    for index,frame in enumerate(app['frames']):
        state=frame['state'];generation=index+1 if source_count else 0
        orientation=1 if scenario.startswith('large') or (scenario=='alternating' and index%2) else 6
        assert state.get('run_id')==app['run_id'],'Wrong frame run identity'
        assert state['requested_generation']==generation,'Wrong requested generation'
        assert state['backend']['backend'] and state['backend']['adapter'],'Missing backend evidence'
        assert frame['capture_provenance']=='window-renderer-readback'
        try:
            from PIL import Image
            from check_probe_capture import check as pixel_check
        except ImportError as error:
            raise ValueError('Smoke pixel checks require Pillow; install tools/fixture-requirements.txt') from error
        path=evidence/frame['file']
        if scenario in ('empty','invalid'):
            assert state['phase']==('empty' if scenario=='empty' else 'error') and state['displayed_generation']==0
            if scenario=='invalid':assert state['error_code']=='invalid-input'
            with Image.open(path) as image:
                assert len(image.getcolors(image.width*image.height) or [])>10,'Blank empty UI'
        else:
            assert state['displayed_generation']==(generation if scenario in ('repeated','alternating') else 1),'Stale/wrong displayed image'
            assert state['source_dimensions']==({'large24':[6000,4000],'large60':[10000,6000]}.get(scenario,[480,320] if orientation==1 else [320,480])),'Wrong source dimensions'
            assert state['phase']==('error' if scenario=='replacement' and index==1 else 'ready'),'Wrong state'
            if scenario=='replacement' and index==1:assert state['error_code']=='invalid-input','Wrong replacement failure'
            pixel_check(path,orientation=orientation,source_aspect={'large24':3/2,'large60':5/3}.get(scenario))
            assert any(e['event']=='render_ready' and e['generation']==state['displayed_generation'] for e in events),'Missing upload readiness'
    return app


def smoke(output,scenario,binary=None,timeout=35):
    # All inputs are repository synthetic fixtures; no private image upload/logging.
    output=output.resolve()
    if output.exists():raise ValueError('Smoke output must be a new directory')
    output.mkdir(parents=True)
    evidence=output/'app'
    fixture=ROOT/'fixtures/s0/orientation-6.jpg'
    cases={'load':[fixture], 'replacement':[fixture,ROOT/'fixtures/s0/invalid.jpg'], 'empty':[], 'invalid':[ROOT/'fixtures/s0/invalid.jpg'], 'repeated':[fixture]*8, 'alternating':[fixture,ROOT/'fixtures/s0/orientation-1.jpg']*4, 'large24':[ROOT/'fixtures/generated/24mp.jpg'], 'large60':[ROOT/'fixtures/generated/60mp.jpg']}
    sources=cases[scenario]
    hashes={path.name:hashlib.sha256(path.read_bytes()).hexdigest() for path in sources}
    binary=Path(binary).resolve() if binary else ROOT/f'target/release/lightwell{EXE}'
    command=[str(binary),'--evidence-dir',str(evidence)]
    for path in sources:command+=['--open',str(path)]
    result={'scenario':scenario,'status':'failed','fixture_hashes':hashes,'command':command,'platform':platform.platform()}
    try:
        result['binary_sha256']=hashlib.sha256(binary.read_bytes()).hexdigest()
        result['lockfile_sha256']=hashlib.sha256((ROOT/'Cargo.lock').read_bytes()).hexdigest()
        with (output/'subprocess.log').open('w') as log:
            process=subprocess.run(command,cwd=ROOT,stdout=log,stderr=log,timeout=timeout)
        result['exit_code']=process.returncode
        if process.returncode:raise ValueError(f'Application exit {process.returncode}')
        app=verify_smoke(evidence,scenario,len(sources))
        for path in sources:assert hashlib.sha256(path.read_bytes()).hexdigest()==hashes[path.name],'Source changed'
        result['status']='passed'
        result['backend']=app['frames'][-1]['state']['backend']
    except (OSError,ValueError,KeyError,IndexError,TypeError,AssertionError,subprocess.TimeoutExpired) as error:
        result['error']=str(error)
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n', encoding="utf-8")
    (output/'reproduce.md').write_text(f'# Smoke run\n\nScenario: {scenario}. Status: {result["status"]}.\n\nInvocation (argument array):\n\n```json\n{json.dumps(command,indent=2)}\n```\n\nRenderer readback; native dialog/focus is a separate test. Sources are synthetic.\n', encoding="utf-8")
    print(json.dumps(result,indent=2))
    return 0 if result['status']=='passed' else 1


def inventory(output):
    host=run('rustc','-vV',capture_output=True,text=True).stdout.split('host: ')[1].splitlines()[0]
    data=json.loads(run('cargo','metadata','--locked','--format-version','1','--filter-platform',host,capture_output=True,text=True).stdout)
    active={node['id'] for node in data['resolve']['nodes']}
    packages=[{'name':p['name'],'version':p['version'],'license':p['license'],'manifest':p['manifest_path']} for p in data['packages'] if p['id'] in active]
    output.mkdir(parents=True,exist_ok=True)
    (output/'dependencies.json').write_text(json.dumps({'target':host,'packages':packages,'review_status':'inventory only; compatibility/advisory audit pending'},indent=2)+'\n', encoding="utf-8")
    notices=output/'licenses';notices.mkdir(exist_ok=True)
    for package in packages:
        source=Path(package['manifest']).parent
        dest=notices/f'{package["name"]}-{package["version"]}'
        for path in source.iterdir():
            if path.is_file() and path.name.lower().startswith(('license','copying','notice')):
                dest.mkdir(exist_ok=True);shutil.copy2(path,dest/path.name)
    print(f'Inventory: {len(packages)} packages. Compatibility/advisory and embedded-asset review remains pending.')


def package(output):
    if output.exists():raise ValueError('Package output must be new')
    cargo('build',True)
    output.mkdir(parents=True)
    target=output/'Lightwell'
    target.mkdir()
    binary=ROOT/f'target/release/lightwell{EXE}'
    if sys.platform=='darwin':
        app=target/'Lightwell.app/Contents';(app/'MacOS').mkdir(parents=True)
        shutil.copy2(binary,app/'MacOS/lightwell')
        (app/'Info.plist').write_bytes(plistlib.dumps({'CFBundleIdentifier':'org.lightwell.app','CFBundleName':'Lightwell','CFBundleExecutable':'lightwell','CFBundlePackageType':'APPL','CFBundleShortVersionString':'0.0.0','LSMinimumSystemVersion':'14.0','NSHighResolutionCapable':True}))
    else:shutil.copy2(binary,target/binary.name)
    revision=run('git','rev-parse','HEAD',capture_output=True,text=True).stdout.strip()
    dirty=bool(run('git','status','--porcelain',capture_output=True,text=True).stdout.strip())
    host=run('rustc','-vV',capture_output=True,text=True).stdout.split('host: ')[1].splitlines()[0]
    (target/'build.json').write_text(json.dumps({'revision':revision,'working_tree_dirty':dirty,'target':host,'profile':'release','binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'lock_sha256':hashlib.sha256((ROOT/'Cargo.lock').read_bytes()).hexdigest()},indent=2)+'\n',encoding='utf-8')
    shutil.copy2(ROOT/'LICENSE',target/'LICENSE')
    inventory(target/'notices')
    (target/'README.txt').write_text('Unsigned local development artifact. License/advisory audit and cross-platform verification are not complete. See project docs/engineering/platforms.md for runtime requirements.\n', encoding="utf-8")
    if sys.platform.startswith('linux'):
        archive=shutil.make_archive(str(output/'lightwell-development'),'gztar',output,'Lightwell')
    else:
        archive=output/'lightwell-development.zip'
        with zipfile.ZipFile(archive,'w',compression=zipfile.ZIP_DEFLATED,strict_timestamps=False) as bundle:
            for path in sorted(target.rglob('*')):
                bundle.write(path,path.relative_to(output))
    digest=hashlib.sha256(Path(archive).read_bytes()).hexdigest()
    (output/'checksums.txt').write_text(f'{digest}  {Path(archive).name}\n', encoding="utf-8")
    print(archive)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='op',required=True)
    for name in ['doctor','fmt','lint','test','check','audit','fixtures']:sub.add_parser(name)
    p=sub.add_parser('build');p.add_argument('--release',action='store_true')
    p=sub.add_parser('develop');p.add_argument('--debug',action='store_true');p.add_argument('arguments',nargs=argparse.REMAINDER)
    p=sub.add_parser('smoke');p.add_argument('--output',type=Path,required=True);p.add_argument('--scenario',choices=['empty','load','replacement','invalid','repeated','alternating','large24','large60'],default='load');p.add_argument('--binary')
    for name in ['package','inventory']:
        p=sub.add_parser(name);p.add_argument('--output',type=Path,required=True)
    if len(sys.argv)>1 and sys.argv[1]=="develop":
        arguments=sys.argv[2:]
        debug=bool(arguments and arguments[0]=="--debug")
        if debug:arguments=arguments[1:]
        run("cargo","run","--locked",*([] if debug else ["--release"]),"--package","lightwell-app","--",*arguments);return 0
    args=parser.parse_args()
    if args.op=='doctor':return doctor()
    if args.op=='fixtures':
        run(sys.executable,'tools/check_fixtures.py');return 0
    if args.op=='audit':
        checker=ROOT/f'.tools/cargo-deny/bin/cargo-deny{EXE}'
        if not checker.is_file():raise ValueError('Install cargo-deny 0.20.2 into .tools/cargo-deny; see scaffold-commands.md')
        version=run(checker,'--version',capture_output=True,text=True).stdout.strip()
        if version!='cargo-deny 0.20.2':raise ValueError(f'Unexpected audit tool: {version}')
        from advisory_policy import audit
        audit(checker);return 0
    if args.op=='check':
        run(sys.executable,'tools/check_repository.py')
        run(sys.executable,'-m','unittest','discover','-s','tools','-p','test_*.py')
        from advisory_policy import check_policy
        check_policy()
        for op in ['fmt','lint','test']:cargo(op)
        print('Headless checks passed. GUI, license/advisory audit and platform acceptance are separate.')
    elif args.op in ['build','fmt','lint','test']:cargo(args.op,getattr(args,'release',False))
    elif args.op=='smoke':return smoke(args.output,args.scenario,args.binary)
    elif args.op=='inventory':inventory(args.output)
    elif args.op=='package':package(args.output)
    return 0


if __name__=='__main__':
    try:sys.exit(main())
    except (OSError,ValueError,subprocess.CalledProcessError) as error:
        print(f'FAIL: {error}',file=sys.stderr);sys.exit(1)
