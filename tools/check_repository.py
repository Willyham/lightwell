#!/usr/bin/env python3
"""Read-only repository contracts, using the checked-in task schema and stdlib."""
import json
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def require(ok, message):
    if not ok:
        raise ValueError(message)


def schema_check(value, schema, root, path):
    if '$ref' in schema:
        target = root
        for key in schema['$ref'].split('/')[1:]:
            target = target[key]
        return schema_check(value, target, root, path)
    kind = schema.get('type')
    types = {'object': dict, 'array': list, 'string': str, 'integer': int}
    if kind:
        require(type(value) is types[kind], f'{path}: expected {kind}')
    if 'const' in schema:
        require(value == schema['const'], f'{path}: invalid constant')
    if 'enum' in schema:
        require(value in schema['enum'], f'{path}: invalid value {value!r}')
    if kind == 'object':
        require(set(schema.get('required', [])) <= value.keys(), f'{path}: missing required keys')
        props = schema.get('properties', {})
        if schema.get('additionalProperties') is False:
            require(value.keys() <= props.keys(), f'{path}: unknown keys')
        for key, child in value.items():
            if key in props:
                schema_check(child, props[key], root, f'{path}.{key}')
    elif kind == 'array':
        require(len(value) >= schema.get('minItems', 0), f'{path}: too few items')
        if schema.get('uniqueItems'):
            require(len({json.dumps(x, sort_keys=True) for x in value}) == len(value), f'{path}: duplicates')
        for i, item in enumerate(value):
            schema_check(item, schema.get('items', {}), root, f'{path}[{i}]')
    elif kind == 'string':
        require(len(value) >= schema.get('minLength', 0), f'{path}: empty string')
        if 'pattern' in schema:
            require(re.search(schema['pattern'], value) is not None, f'{path}: pattern mismatch')
    elif kind == 'integer':
        require(value >= schema.get('minimum', value), f'{path}: below minimum')


def check_plan(plan, schema):
    schema_check(plan, schema, schema, plan['plan_id'])
    tasks = {task['id']: task for task in plan['tasks']}
    require(len(tasks) == len(plan['tasks']), 'Duplicate task IDs')
    for key, task in tasks.items():
        require(set(task['dependencies']) <= tasks.keys(), f'{key}: unknown dependency')
        require(key not in task['dependencies'], f'{key}: self dependency')
        if task['status'] in {'ready', 'in_progress', 'completed'}:
            require(all(tasks[d]['status'] == 'completed' for d in task['dependencies']), f'{key}: unmet prerequisite')
        if task['status'] in {'blocked', 'cancelled'}:
            require(bool(task['extra_context']), f'{key}: missing status explanation')
        for link in task['context_links']:
            if link['kind'] == 'file':
                require((ROOT / link['target'].split('#')[0]).is_file(), f'{key}: missing {link["target"]}')
            if link['kind'] == 'task':
                require(link['target'] in tasks, f'{key}: missing linked task')
    remaining, done, waves = set(tasks), set(), []
    while remaining:
        ready = sorted(k for k in remaining if set(tasks[k]['dependencies']) <= done)
        require(bool(ready), 'Task dependency cycle')
        waves.append({'wave': len(waves) + 1, 'task_ids': ready})
        done.update(ready)
        remaining.difference_update(ready)
    require(plan['execution_waves'] == waves, f'{plan["plan_id"]}: stale execution waves')
    return tasks


def main():
    schema = json.loads((ROOT / 'tools/task-plan.schema.json').read_text())
    plans = []
    for name in ['product-decisions', 'implementation']:
        plan = json.loads((ROOT / f'tasks/{name}.json').read_text())
        plans.append(check_plan(plan, schema))
        print(f'PASS {name}: {len(plan["tasks"])} tasks, {len(plan["execution_waves"])} waves')
    product, implementation = plans
    require(not product.keys() & implementation.keys(), 'Task IDs overlap across active plans')
    for gate, inputs in {'TASK-035': ['TASK-023', 'TASK-024', 'TASK-025'], 'TASK-064': [f'TASK-{n:03}' for n in range(26, 31)]}.items():
        if implementation[gate]['status'] == 'completed':
            require(all(product[t]['status'] == 'completed' for t in inputs), f'{gate}: unresolved product decision')
    files = [ROOT / 'README.md', ROOT / 'AGENTS.md', ROOT / 'CONTRIBUTING.md']
    files += list((ROOT / 'docs').rglob('*.md')) + list((ROOT / 'fixtures').rglob('*.md'))
    count = 0
    for file in files:
        text = re.sub(r'```.*?```', '', file.read_text(), flags=re.S)
        for raw in re.findall(r'\[[^\]\n]*\]\(([^)]+)\)', text):
            target = raw.split(' "', 1)[0].strip('<>')
            url = urlsplit(target)
            if url.scheme or not url.path:
                continue
            path = (file.parent / unquote(url.path)).resolve()
            require(path.exists(), f'{file.relative_to(ROOT)}: broken link {target}')
            count += 1
    print(f'PASS {count} local Markdown links and external completion gates')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, KeyError, OSError) as error:
        print(f'FAIL {error}', file=sys.stderr)
        sys.exit(1)
