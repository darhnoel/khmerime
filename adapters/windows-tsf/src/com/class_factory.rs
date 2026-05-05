//! COM `IClassFactory` implementation for the TSF text service.

use core::ffi::c_void;

use windows::core::{implement, Error, IUnknown, Interface, Result, GUID};
use windows::Win32::Foundation::{BOOL, CLASS_E_NOAGGREGATION, E_POINTER};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::TextServices::ITfTextInputProcessor;

use crate::com::dll_module;
use crate::com::text_service::KhmerImeTextService;

/// Human-readable name for the COM class factory boundary.
pub const CLASS_FACTORY_BOUNDARY: &str = "KhmerIME TSF COM class factory";

#[implement(IClassFactory)]
pub struct KhmerImeClassFactory;

impl KhmerImeClassFactory {
    pub fn new() -> Self {
        dll_module::object_created();
        Self
    }
}

impl Drop for KhmerImeClassFactory {
    fn drop(&mut self) {
        dll_module::object_released();
    }
}

impl IClassFactory_Impl for KhmerImeClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Option<&IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        if ppvobject.is_null() {
            return Err(Error::from(E_POINTER));
        }
        unsafe {
            *ppvobject = std::ptr::null_mut();
        }
        if punkouter.is_some() {
            return Err(Error::from(CLASS_E_NOAGGREGATION));
        }

        let service: ITfTextInputProcessor = KhmerImeTextService::new().into();
        unsafe { service.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        dll_module::lock_server(flock.as_bool());
        Ok(())
    }
}
