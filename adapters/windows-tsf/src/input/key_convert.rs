//! Placeholder for Windows key conversion.
//!
//! Planned behavior:
//! - printable text becomes a Unicode scalar `NativeKeyEvent.keyval`.
//! - Windows virtual-key code is preserved in `NativeKeyEvent.keycode`.
//! - Ctrl/Alt/Windows shortcuts pass through to the host application.
//! - Enter, Backspace, Escape, Space, and arrows map to the session key contract.

/// Documents that key conversion is not implemented in this skeleton phase.
pub const KEY_CONVERSION_IMPLEMENTED: bool = false;
