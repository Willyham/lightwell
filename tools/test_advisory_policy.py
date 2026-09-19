"""Policy regressions: an exception must fail closed when its scope changes."""
import copy
import datetime
import unittest
from advisory_policy import EXCEPTIONS, validate


class AdvisoryPolicyTests(unittest.TestCase):
    def setUp(self):
        self.exceptions = copy.deepcopy(EXCEPTIONS)
        self.packages = [dict(name=e['package'], version=e['version'], source='registry+https://github.com/rust-lang/crates.io-index') for e in self.exceptions]
        self.tasks = {e['task']: dict(status='ready') for e in self.exceptions}
        self.base = '[licenses]\nallow = ["MIT"]\n'
        self.today = datetime.date(2026, 9, 19)

    def policy(self):
        return validate(self.exceptions, self.packages, self.tasks, self.base, self.today)

    def test_valid_policy_only_ignores_reviewed_ids(self):
        config = self.policy()
        self.assertTrue(config.startswith(self.base))
        self.assertEqual(config.count('{ id = '), 2)
        self.assertIn('expires 2026-10-19', config)
        self.assertNotIn('unmaintained =', config)

    def test_expiry_is_exclusive(self):
        self.today = datetime.date(2026, 10, 19)
        with self.assertRaisesRegex(ValueError, 'expired'): self.policy()

    def test_future_review_and_excessive_duration_rejected(self):
        self.today = datetime.date(2026, 9, 18)
        with self.assertRaisesRegex(ValueError, 'window'): self.policy()
        self.today = datetime.date(2026, 9, 19)
        self.exceptions[0]['expires'] = '2027-01-01'
        with self.assertRaisesRegex(ValueError, 'window'): self.policy()

    def test_version_source_and_removal_require_review(self):
        original = copy.deepcopy(self.packages)
        for replacement in [[], [dict(original[0], version='1.0.16')], [dict(original[0], source='git+https://example.invalid')]]:
            self.packages = replacement + original[1:]
            with self.assertRaisesRegex(ValueError, 'changed or removed'): self.policy()

    def test_missing_or_retired_task_rejected(self):
        for value in [None, dict(status='completed'), dict(status='cancelled')]:
            self.tasks[self.exceptions[0]['task']] = value
            with self.assertRaisesRegex(ValueError, 'missing or retired'): self.policy()

    def test_static_ignores_and_duplicate_ids_rejected(self):
        self.base += '\n[advisories]\nignore = ["RUSTSEC-2024-0436"]\n'
        with self.assertRaisesRegex(ValueError, 'Static'): self.policy()
        self.base = ''
        self.exceptions[1]['id'] = self.exceptions[0]['id']
        with self.assertRaisesRegex(ValueError, 'duplicate'): self.policy()


if __name__ == '__main__':
    unittest.main()
