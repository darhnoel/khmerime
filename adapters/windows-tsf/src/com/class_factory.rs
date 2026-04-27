//! Placeholder for the future COM `IClassFactory` implementation.
//!
//! The class factory will create the KhmerIME TSF text service object after COM
//! loads the DLL. It must not own IME composition, candidate ranking, or history
//! learning behavior.

/// Human-readable name for the planned COM class factory boundary.
pub const CLASS_FACTORY_BOUNDARY: &str = "KhmerIME TSF COM class factory";
