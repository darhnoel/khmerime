# Decoder Golden Tests

These golden files lock the externally visible decoder behavior for selected high-value inputs.

Protected surface:
- `Transliterator::suggest(...)`
- `Wfst` decoder mode
- exact ordered suggestion output for the checked-in `top_n` per case

Snapshot policy:
- smaller stable cases freeze the full ranked prefix
- long beam-search phrase cases freeze the exact top-1 best path
- this keeps important phrase recovery locked while avoiding low-signal tail churn

Why this exists:
- to catch silent ranking and beam-search regressions
- to make decoder behavior changes reviewable in plain text diffs
- to ensure `cargo test` fails when outputs change

Verification:
```bash
cargo test
cargo test --features wfst-decoder
cargo test --test decoder_golden
```

Important workflow rule:
- normal test runs only verify
- there is no auto-bless or silent snapshot rewrite

Intentional updates:
1. make the decoder change
2. run the golden test and inspect the diff carefully
3. edit `tests/golden/decoder_wfst_suggest.txt` intentionally
4. rerun the tests

If behavior changes without a deliberate snapshot edit, CI should fail.
