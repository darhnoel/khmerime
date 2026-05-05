//! COM DLL exports for the KhmerIME TSF text service.

use core::ffi::c_void;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use windows::core::{Interface, GUID, HRESULT};
use windows::Win32::Foundation::{BOOL, CLASS_E_CLASSNOTAVAILABLE, HINSTANCE, S_FALSE, S_OK};
use windows::Win32::System::Com::IClassFactory;

use crate::com::class_factory::KhmerImeClassFactory;
use crate::com::registration::{register_server, unregister_server, CLSID_KHMERIME_TEXT_SERVICE};

static DLL_MODULE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static OBJECT_COUNT: AtomicU32 = AtomicU32::new(0);
static LOCK_COUNT: AtomicU32 = AtomicU32::new(0);

/// Documents the COM DLL export surface implemented by this crate.
pub const PLANNED_DLL_EXPORTS: &[&str] = &[
    "DllGetClassObject",
    "DllCanUnloadNow",
    "DllRegisterServer",
    "DllUnregisterServer",
    "DllMain",
];

pub fn module_instance() -> HINSTANCE {
    HINSTANCE(DLL_MODULE.load(Ordering::SeqCst))
}

pub fn object_created() {
    OBJECT_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn object_released() {
    OBJECT_COUNT.fetch_sub(1, Ordering::SeqCst);
}

pub fn lock_server(lock: bool) {
    if lock {
        LOCK_COUNT.fetch_add(1, Ordering::SeqCst);
    } else {
        LOCK_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

#[no_mangle]
pub extern "system" fn DllMain(instance: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == 1 {
        DLL_MODULE.store(instance.0, Ordering::SeqCst);
    }
    BOOL(1)
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    clsid: *const GUID,
    iid: *const GUID,
    object: *mut *mut c_void,
) -> HRESULT {
    if clsid.is_null() || iid.is_null() || object.is_null() {
        return windows::Win32::Foundation::E_POINTER;
    }

    *object = std::ptr::null_mut();
    if *clsid != CLSID_KHMERIME_TEXT_SERVICE {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory: IClassFactory = KhmerImeClassFactory::new().into();
    factory.query(iid, object)
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECT_COUNT.load(Ordering::SeqCst) == 0 && LOCK_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    match register_server(module_instance()) {
        Ok(()) => S_OK,
        Err(err) => err.code(),
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    match unregister_server() {
        Ok(()) => S_OK,
        Err(err) => err.code(),
    }
}
