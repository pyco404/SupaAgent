#!/usr/bin/env python3
"""Guard tests only: no PostgreSQL, credentials, network or runtime activation."""
import contextlib
import copy
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('onboarding', ROOT / 'scripts/validate-local-onboarding.py')
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)


class PreparationGuards(unittest.TestCase):
    def setUp(self):
        self.plan = json.loads((ROOT / 'config/first-agent-plan.json').read_text())

    def test_reviewed_proposal_is_inactive_without_assigned_agent_or_recipient(self):
        self.assertEqual(m.validate_plan(self.plan)['agent_id'], None)
        self.assertFalse(self.plan['agent_request']['config']['scheduler_enabled'])
        self.assertIsNone(self.plan['devnet_recipient'])

    def test_approval_identity_recipient_and_execution_drift_are_rejected(self):
        mutations = [('approved', True), ('agent_id', m.OWNER), ('epoch_id', m.OWNER),
                     ('proposed_owner_principal', m.CUSTODIAN), ('initial_lifecycle', 'active'),
                     ('live_spending_authorized', True), ('worker_launch_authorized', True),
                     ('devnet_recipient', '11111111111111111111111111111112')]
        for key, value in mutations:
            with self.subTest(key=key):
                proposal = copy.deepcopy(self.plan)
                proposal[key] = value
                with self.assertRaises(ValueError):
                    m.validate_plan(proposal)

    def test_broader_capabilities_owner_claims_and_unbounded_policy_are_rejected(self):
        for mutate in [lambda p: p['agent_request'].update(owner_id=m.OWNER),
                       lambda p: p['agent_request']['capabilities'].append('draft_tweet'),
                       lambda p: p['agent_request']['config'].update(scheduler_enabled=True),
                       lambda p: p['agent_request']['config'].update(max_provider_calls=9),
                       lambda p: p['future_offline_policy'].update(max_cycles=100),
                       lambda p: p['future_offline_policy'].update(valid_for_seconds=86400),
                       lambda p: p.update(goals=[])]:
            proposal = copy.deepcopy(self.plan)
            mutate(proposal)
            with self.assertRaises(ValueError):
                m.validate_plan(proposal)

    def test_crash_sequence_gaps_are_safe_but_rewinds_are_rejected(self):
        m.check_sequence_progress({'audit': 3, 'unused': None}, {'audit': 33, 'unused': None})
        m.check_sequence_progress({'audit': 3, 'unused': None}, {'audit': 3, 'unused': 1})
        for after in [{'audit': 2, 'unused': None}, {'audit': None, 'unused': None}, {'unused': None}]:
            with self.assertRaises(ValueError):
                m.check_sequence_progress({'audit': 3, 'unused': None}, after)

    def test_fingerprint_default_stays_strict_and_restart_mode_keeps_row_changes(self):
        pg = m.module('onboarding_fingerprint_contract', 'persistent-postgres.py')
        state = {'row': 1, 'counter': 3}
        def sql(query, database):
            if 'json_build_array(schemaname,tablename)' in query:
                return '[["public","audit"]]'
            return json.dumps({'row': state['row'], 'counter': state['counter'] if 'last_value' in query else None})
        with patch.object(pg, 'sql', side_effect=sql):
            strict = pg.fingerprint('fixture')
            self.assertEqual(strict, pg.fingerprint('fixture', include_sequence_values=True))
            restart = pg.fingerprint('fixture', include_sequence_values=False)
            state['counter'] = 33
            self.assertNotEqual(strict, pg.fingerprint('fixture'))
            self.assertEqual(restart, pg.fingerprint('fixture', include_sequence_values=False))
            state['row'] = 2
            self.assertNotEqual(restart, pg.fingerprint('fixture', include_sequence_values=False))

    def test_cli_has_no_arbitrary_target_or_primary_activation_switch(self):
        for option in ['--database-url', '--data-directory', '--activate', '--install-credentials']:
            with self.subTest(option=option), patch('sys.argv', ['validator', 'exercise-disposable', option, 'primary']), patch.object(m, 'exercise') as execute, contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as error:
                    m.main()
                self.assertEqual(error.exception.code, 2)
                execute.assert_not_called()

    def test_inspect_never_enters_disposable_execution(self):
        with patch('sys.argv', ['validator', 'inspect']), patch.object(m, 'baseline', return_value=(None, {})), patch.object(m, 'exercise') as execute, contextlib.redirect_stdout(io.StringIO()) as output:
            m.main()
            self.assertFalse(json.loads(output.getvalue())['owner_granted'])
            execute.assert_not_called()


if __name__ == '__main__':
    unittest.main()
