# ADR-0008: Strip tap commits; long-press opens candidate panel

**Status:** Accepted

## Context

The keyboard had two separate commit surfaces that confused users:
- ⏎ committed the composition (Khmer text inserted, roman chars removed)
- Strip tap cycled through candidates in a blind round-robin

The ⏎ button's dual role (commit composition vs. insert newline) was implicit. The strip tap's cycling behaviour was invisible — the "›" hint did not tell users how many candidates existed or how to stop cycling.

## Decision

| Gesture | Before | After |
|---|---|---|
| Strip tap | Cycle candidates | **Commit full phrase** |
| Strip long-press | Nothing | **Open candidate panel** |
| ⏎ composing | Commit | **Commit + insert newline** |
| ⏎ idle | Nothing | **Insert newline** |
| Panel candidate tap | Select only | Select only (unchanged) |

Cycling is removed from the strip entirely. The panel is the only candidate-browsing surface. Tapping any candidate in the panel commits that variant immediately and closes the panel.

## Consequences

- Strip tap is now a deliberate "I accept this suggestion" gesture, not a blind cycle.
- The "›" hint is removed from the strip; the Khmer text alone is the tap target.
- Users who want an alternative candidate open the panel via long-press or 💡.
- ⏎ now inserts a newline when idle, which is the expected iOS keyboard contract.
