# Test runs

The `pvc-test` mod (or `python3 pilot/owner-test/run.py`) writes every test run here:

- `<UTC time>_<test id>_<name>.md`: one file per test run with every detail of that test.
- `<same name>/`: its screenshots, every file each agent left, and the raw Claude Code transcripts.
- `INDEX.md`: one row per test run, made at the first run.

How the test works: [`pilot/owner-test/README.md`](../pilot/owner-test/README.md).
