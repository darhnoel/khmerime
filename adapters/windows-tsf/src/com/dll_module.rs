//! Placeholder for future COM DLL exports.
//!
//! A runnable TSF adapter will eventually export:
//! - `DllGetClassObject`
//! - `DllCanUnloadNow`
//! - `DllRegisterServer`
//! - `DllUnregisterServer`
//!
//! This file deliberately does not define those exports yet. Adding real exports
//! also requires Windows API bindings, registration behavior, and manual Windows
//! smoke testing.

/// Documents the future COM DLL export surface without implementing it.
pub const PLANNED_DLL_EXPORTS: &[&str] = &[
    "DllGetClassObject",
    "DllCanUnloadNow",
    "DllRegisterServer",
    "DllUnregisterServer",
];
