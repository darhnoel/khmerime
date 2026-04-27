//! Future Windows render-action boundary.
//!
//! Render code should translate `SessionSnapshot` and `SessionResult` into TSF
//! actions: update preedit, refresh candidates, commit once, or clear state.

pub mod render_state;
