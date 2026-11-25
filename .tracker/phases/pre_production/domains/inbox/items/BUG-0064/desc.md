## Problem Description
scripts/phased-step.sh:236-263 — fingerprint() uses git status --porcelain=v1 --untracked-files=all and changed_since_snapshot() diffs pre-step and post-step snapshots with comm -3. When a step adds an ignore rule (e.g. .agents/ in STEP 20F) matching pre-existing untracked files, those files disappear from fingerprint() and are reported as out-of-scope changes. Suggested fix: filter out paths present only in SNAP/tree that still exist unchanged on disk and are now git-check-ignore'd, or pass --ignored when comparing against SNAP/tree.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
