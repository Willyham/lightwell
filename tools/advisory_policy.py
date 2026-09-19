#!/usr/bin/env python3
"""Exact, expiring maintenance exceptions for the maintained audit command."""
import datetime
import json
import re
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXCEPTIONS = (
    dict(id='RUSTSEC-2024-0436', package='paste', version='1.0.15',
         reviewed='2026-09-19', expires='2026-12-18', task='TASK-065',
         reason='Build-time macro in pinned Metal dependency; no supported Iced upgrade removes it. Local S0 development only.'),
    dict(id='RUSTSEC-2026-0192', package='ttf-parser', version='0.25.1',
         reviewed='2026-09-19', expires='2026-10-19', task='TASK-066',
         reason='Pinned Iced system/bundled-font stack; no application font import. Undisclosed upstream report requires short review window; no distribution approval.'),
)


def validate(exceptions, packages, tasks, base, today=None):
    today = today or datetime.datetime.now(datetime.timezone.utc).date()
    if re.search(r'^\s*(?:\[advisories\]|ignore\s*=)', base, re.M):
        raise ValueError('Static advisory overrides are forbidden; use reviewed expiring policy')
    ids = set()
    for item in exceptions:
        advisory = item['id']
        if not re.fullmatch(r'RUSTSEC-\d{4}-\d{4}', advisory) or advisory in ids:
            raise ValueError('Invalid or duplicate advisory exception')
        ids.add(advisory)
        reviewed = datetime.date.fromisoformat(item['reviewed'])
        expires = datetime.date.fromisoformat(item['expires'])
        if not reviewed <= today < expires or not 0 < (expires-reviewed).days <= 90:
            raise ValueError(f'{advisory}: expired/invalid review window; resolve {item["task"]}')
        matches = [p for p in packages if p['name'] == item['package']]
        if not matches or any(p['version'] != item['version'] or not str(p.get('source', '')).startswith('registry+') for p in matches):
            raise ValueError(f'{advisory}: dependency version/source changed or removed; review exception')
        task = tasks.get(item['task'])
        if not task or task['status'] in {'completed', 'cancelled'}:
            raise ValueError(f'{advisory}: follow-up task missing or retired')
        if not item['reason'].strip():
            raise ValueError(f'{advisory}: missing rationale')
    return base + '\n[advisories]\nignore = [\n' + ''.join(
        '  { id = ' + json.dumps(e['id']) + ', reason = ' + json.dumps(
            f'{e["package"]} {e["version"]}; expires {e["expires"]}; {e["task"]}. {e["reason"]}') + ' },\n'
        for e in exceptions) + ']\n'


def checked_config():
    # Reuse the exact locked metadata for both validation and cargo-deny.
    metadata = subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--format-version', '1'], cwd=ROOT,
        text=True, encoding='utf-8')
    data = json.loads(metadata)
    tasks = {t['id']: t for t in json.loads((ROOT/'tasks/implementation.json').read_text(encoding='utf-8'))['tasks']}
    config = validate(EXCEPTIONS, data['packages'], tasks, (ROOT/'deny.toml').read_text(encoding='utf-8'))
    return metadata, config


def check_policy():
    checked_config()
    print("PASS exact advisory exception versions, tasks and UTC expiry")


def audit(checker):
    metadata, config = checked_config()
    for item in EXCEPTIONS:
        print(f'Reviewed exception: {item["id"]}, {item["package"]} {item["version"]}, expires {item["expires"]}, {item["task"]}', flush=True)
    with tempfile.TemporaryDirectory(prefix='lightwell-audit-') as directory:
        config_path = Path(directory)/'deny.toml'
        metadata_path = Path(directory)/'metadata.json'
        config_path.write_text(config, encoding='utf-8')
        metadata_path.write_text(metadata, encoding='utf-8')
        subprocess.run([str(checker), '--manifest-path', str(ROOT/'Cargo.toml'),
                        '--metadata-path', str(metadata_path), '--config', str(config_path),
                        'check', 'licenses', 'sources', 'advisories'], cwd=ROOT, check=True)
