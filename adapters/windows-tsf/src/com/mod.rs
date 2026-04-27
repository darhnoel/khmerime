//! Future COM server boundary for the Windows TSF text service.
//!
//! This module is intentionally a skeleton. It documents where COM-specific
//! code will live once the adapter grows into a runnable Windows DLL.
//!
//! Planned responsibilities:
//! - DLL entry points such as `DllGetClassObject` and `DllCanUnloadNow`.
//! - COM class factory construction for the KhmerIME text service.
//! - COM registration and unregistration helpers.
//! - TSF text-service profile registration.
//!
//! Do not add Khmer transliteration, ranking, or session behavior here. COM code
//! should create or activate the TSF shell, then delegate IME behavior to the
//! shared `khmerime_session` boundary.

pub mod class_factory;
pub mod dll_module;
pub mod registration;
pub mod text_service;
