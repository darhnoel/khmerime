# Async full warmup with all three engines instead of Phase A / Phase B

On startup the macOS IMK process loads all three engines (live `shadow_interactive`,
visible refiner, commit refiner) in a single background thread. If a key event arrives
before the thread finishes, the session blocks briefly on a condvar (500 ms timeout).
This differs from the Linux IBus adapter, which uses Phase A (~100 ms, legacy decoder)
followed by Phase B (~1300 ms, full engines) with an in-process engine swap.

Phase A / Phase B was designed for IBus because the bridge process restarts on every
login and must accept keystrokes within milliseconds. The macOS IMK process is
long-lived (it starts once and stays resident for the login session), and on Apple
Silicon the full load completes in roughly 400 ms — fast enough that a first-keystroke
block is almost never observed. Carrying the Phase A/B complexity (two session
configurations, idle-composition detection, engine swap logic) would add ~200 lines of
machinery for a race that rarely materialises.

This ADR originally claimed the single-phase approach was consistent with how Windows TSF
does warmup (`spawn_default_driver_warmup`). That was not accurate and is corrected here:
the two adapters shared a warmup *strategy* but not a *fallback*. macOS waits for the
engine and still composes the key; Windows returned `FALSE` and let the roman reach the
document. The apparent parity delayed discovery of that defect. See
`adapters/windows-tsf/docs/adr/0001-warmup-must-not-leak-roman-into-the-document.md`,
which also notes that the long-lived-process reasoning above does not transfer to TSF —
Windows loads the text-service DLL into every host application process.

The visible refiner and commit refiner are included in the background load (not deferred
to a third phase) because the macOS process lifetime amortises the cost and full refiner
coverage is required to match the 23 IBus protocol tests that cover refinement behaviour.
