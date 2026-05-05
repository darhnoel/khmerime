//! Render action vocabulary for the Windows TSF adapter.
//!
//! The public `derive_render_state` helper in `lib.rs` converts
//! `SessionSnapshot` + `SessionResult` into this coarse action set. Windows-only
//! TSF edit-session code then performs the native document mutations.

/// Coarse-grained native rendering responsibilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderAction {
    /// Update or start TSF composition text from the session preedit.
    UpdateComposition,
    /// Clear/end the active TSF composition.
    ClearComposition,
    /// Refresh the candidate UI from the session candidate list.
    UpdateCandidates,
    /// Hide the candidate UI.
    ClearCandidates,
    /// Commit one-shot text to the host document.
    CommitText,
}
