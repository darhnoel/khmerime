//! Placeholder for future COM and TSF registration.
//!
//! A real Windows IME needs both normal COM in-process server registration and
//! TSF profile registration. Keep registry writes and TSF registration isolated
//! here so install/uninstall behavior does not leak into typing behavior.
//!
//! Packaging remains out of scope until a manually registered TSF text service
//! works in a Windows text field.

/// Registration operations expected once the Windows adapter becomes runnable.
pub const PLANNED_REGISTRATION_STEPS: &[&str] = &[
    "register COM CLSID",
    "register TSF input processor profile",
    "register keyboard text-service category",
    "unregister TSF profile before removing COM CLSID",
];
