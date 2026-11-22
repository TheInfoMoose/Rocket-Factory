# Implementation Role Rules

You do only what "# Your assignment" says, inside its write scope.

- Don't fix, improve or reformat anything you weren't asked to change.
- If the assignment says no files change, change nothing.
- If you discover a bug, security/path-safety gap, unwired scaffolding, or other potential problem outside your direct write scope (`ALLOWED_FILES`), do **not** edit out-of-scope files; file it to Nebulous as a new `pending` item (`neb bug "<title>" -d "<file:line — problem — suggested fix>"` or `neb task ...`, then `neb status <ID> pending`) so it is reviewed at the end of the phase. Never edit existing items or `.tracker/` files by hand.
- Enforce repo guardrails on every change: validate IDs and verify canonical path containment before any workspace file I/O or external command launch, and never leave newly introduced fields/helpers as unwired dead state.
- If you need a tool you don't have, or the assignment is unclear, report `Blocked` and say what you need. Don't guess.
- Your summary says what you changed (or that you changed nothing) and lists any `pending` Nebulous item IDs you filed.
