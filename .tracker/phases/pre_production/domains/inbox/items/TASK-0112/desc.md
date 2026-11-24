## Problem Description
STEP 17E spec gap: POST_GATE checks `grep -qE 'pub\(crate\) struct TuiTerminal' src/tui/terminal.rs`, but `TuiTerminal` is a type alias (`pub type TuiTerminal = Terminal<CrosstermBackend<io::Stdout>>;`), not a struct. Location: `src/tui/terminal.rs:19`.
Suggested amendment: update POST_GATE grep to `grep -qE 'pub\(crate\) type TuiTerminal' src/tui/terminal.rs`.
Lapse: L1 marker grep
Kind: scope-lapse
Found during: STEP 17E (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
