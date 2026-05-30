# Commit Refiner keeps a latency budget

The **Commit Refiner** runs on Enter to produce the final **Commit Text**. We
give it a wall-clock latency budget (150 ms in the IBus bridge) rather than
letting it run unbounded, even though it fires after the user has stopped
typing. Enter must feel instant; an unbounded decode could stall the commit on a
long phrase or a busy machine.

The trade-off is completeness. When the budget trips, the decode is marked as a
`Timeout` failure and the **Hidden Commit Fallback** yields nothing, so the
commit degrades to the **Visible Candidate Commit** (or the raw roman floor) —
whatever was already on screen. This is acceptable because the Commit Refiner
only acts as a fallback when the visible candidate is not useful Khmer; a good
visible candidate already wins regardless. So a budget trip never commits wrong
or empty text — at worst it commits the less-refined result the user could
already see.

## Consequences

- The budget is wall-clock, not decode-cost. Under heavy CPU contention the
  stopwatch can trip on a decode that is actually cheap, because the decode
  thread spent its time descheduled. In production this is rare and degrades
  gracefully; in the parallel test runner it caused spurious failures, so
  correctness-focused test helpers disable the budget (`wfst_max_latency_ms =
  u64::MAX`) to decouple result assertions from scheduling.
- An earlier glossary entry described the Commit Refiner as having "no budget."
  That was never true in the shipping adapters; the glossary now matches the
  code.
