#!/usr/bin/env python3
"""Verify recorded demonstration artifacts without a database or model credential."""
from collections import Counter
import json
from pathlib import Path

base = Path('artifacts/demo')
load = lambda name: json.loads((base / name).read_text())
first, ten, final = (load(n) for n in ['before-restart.json', 'ten-cycles.json', 'final.json'])
paused = load('paused-after-restart.json')
assert len(first['cycles']) == 5 and len(ten['cycles']) == 10
assert {c['id'] for c in first['cycles']} <= {c['id'] for c in ten['cycles']}
assert {m['id'] for m in first['agent']['memory']} <= {m['id'] for m in ten['agent']['memory']}
assert Counter(c['status'] for c in final['cycles']) == {'completed': 15, 'interrupted': 1}
assert len(final['agent']['memory']) == 30
assert Counter(m['kind'] for m in final['agent']['memory']) == {'hypothesis': 15, 'verified_observation': 15}
assert len(final['actions']) == 49 and all(a['status'] == 'succeeded' and a['authorization_outcome'] == 'allowed' for a in final['actions'])
assert len(final['evaluations']) == 14 and all(e['verification'] == 'model_assessment' for e in final['evaluations'])
assert not any(g['completed'] for g in final['agent']['goals'])
assert len(final['drafts']) == 5 and len(load('generated-drafts.json')['drafts']) == 5
known = {e['id'] for e in final['events']}
for draft in final['drafts']:
    assert draft['status'] == 'draft'
    assert len(draft['data']['text']) <= 280 and 'simulat' in draft['data']['text'].lower()
    assert set(draft['data']['event_ids']) <= known
assert all(not call['policy']['live'] for call in final['inference'])
assert Counter(c['status'] for c in final['inference']) == {'succeeded': 20, 'unknown': 1}
assert final['financials']['balance_microusd'] == sum(e['amount_microusd'] for e in final['ledger']) == 990000
assert final['financials']['revenue_microusd'] == 12500
assert final['financials']['inference_microusd'] == 21000
assert final['financials']['operating_microusd'] == 1500
assert all(e['simulated'] for e in final['ledger'])
assert final['agent']['lifecycle'] == paused['agent']['lifecycle'] == 'paused'
assert len(paused['cycles']) == len(final['cycles'])
assert len(final['events']) == 229
for name in ['init.log', 'continue.log', 'drafts.log', 'http-process.log']:
    for line in (base / name).read_text().splitlines():
        json.loads(line)
smoke = load('http-process-result.json')
assert smoke['actual_process_restart'] and smoke['emergency_stop_persisted']
result = {'verified': True, 'completed_cycles': 15, 'interrupted_cycles': 1,
          'memory_entries': 30, 'successful_actions': 49, 'goal_assessments': 14,
          'drafts': 5, 'activity_events': 229, 'live_openai_calls': 0,
          'draft_prose': 'assistant-authored from inspected persisted events, deterministically replayed',
          'real_process_restart': True}
(base / 'verified-summary.json').write_text(json.dumps(result, indent=2))
print(json.dumps(result))
