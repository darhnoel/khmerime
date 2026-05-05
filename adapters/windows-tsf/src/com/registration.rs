//! COM and TSF profile registration helpers.

use windows::core::{w, Error, Result, GUID, PCWSTR};
use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS, HINSTANCE};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, REG_SZ,
};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr, ITfInputProcessorProfiles,
    GUID_TFCAT_TIP_KEYBOARD,
};

pub const CLSID_KHMERIME_TEXT_SERVICE: GUID = GUID::from_u128(0x79f0a9c7_fec5_4637_9d9d_4dfc54c8b5c2);
pub const GUID_KHMERIME_PROFILE: GUID = GUID::from_u128(0x40fa8742_c6ef_4c1b_9f9a_ae064a6ec66d);
pub const KHMER_LANGUAGE_ID: u16 = 0x0453;
pub const TEXT_SERVICE_DESCRIPTION: &str = "KhmerIME";

/// Registration operations implemented by the Windows adapter.
pub const PLANNED_REGISTRATION_STEPS: &[&str] = &[
    "register COM CLSID",
    "register TSF input processor profile",
    "register keyboard text-service category",
    "unregister TSF profile before removing COM CLSID",
];

pub unsafe fn register_server(module: HINSTANCE) -> Result<()> {
    let module_path = module_path(module)?;
    register_com_class(&module_path)?;
    unregister_tsf_profile();
    register_tsf_profile()?;
    Ok(())
}

pub unsafe fn unregister_server() -> Result<()> {
    unregister_tsf_profile();
    unregister_com_class()
}

unsafe fn register_com_class(module_path: &str) -> Result<()> {
    let clsid = guid_braced(&CLSID_KHMERIME_TEXT_SERVICE);
    set_default_value(&format!("CLSID\\{clsid}"), TEXT_SERVICE_DESCRIPTION)?;
    set_default_value(&format!("CLSID\\{clsid}\\InprocServer32"), module_path)?;
    set_named_value(
        &format!("CLSID\\{clsid}\\InprocServer32"),
        "ThreadingModel",
        "Apartment",
    )?;
    Ok(())
}

unsafe fn unregister_com_class() -> Result<()> {
    let clsid = guid_braced(&CLSID_KHMERIME_TEXT_SERVICE);
    let subkey = wide_null(&format!("CLSID\\{clsid}"));
    let status = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(subkey.as_ptr()));
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::from_win32())
    }
}

unsafe fn register_tsf_profile() -> Result<()> {
    let profiles: ITfInputProcessorProfiles =
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
    profiles.Register(&CLSID_KHMERIME_TEXT_SERVICE)?;
    let description = wide_null(TEXT_SERVICE_DESCRIPTION);
    let icon_file = wide_null("");
    profiles.AddLanguageProfile(
        &CLSID_KHMERIME_TEXT_SERVICE,
        KHMER_LANGUAGE_ID,
        &GUID_KHMERIME_PROFILE,
        &description,
        &icon_file,
        0,
    )?;
    profiles.EnableLanguageProfileByDefault(
        &CLSID_KHMERIME_TEXT_SERVICE,
        KHMER_LANGUAGE_ID,
        &GUID_KHMERIME_PROFILE,
        BOOL(1),
    )?;

    let categories: ITfCategoryMgr = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
    categories.RegisterCategory(
        &CLSID_KHMERIME_TEXT_SERVICE,
        &GUID_TFCAT_TIP_KEYBOARD,
        &CLSID_KHMERIME_TEXT_SERVICE,
    )?;
    Ok(())
}

unsafe fn unregister_tsf_profile() {
    if let Ok(profiles) =
        CoCreateInstance::<_, ITfInputProcessorProfiles>(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)
    {
        let _ = profiles.RemoveLanguageProfile(&CLSID_KHMERIME_TEXT_SERVICE, KHMER_LANGUAGE_ID, &GUID_KHMERIME_PROFILE);
        let _ = profiles.Unregister(&CLSID_KHMERIME_TEXT_SERVICE);
    }
}

unsafe fn set_default_value(subkey: &str, value: &str) -> Result<()> {
    set_registry_value(subkey, None, value)
}

unsafe fn set_named_value(subkey: &str, name: &str, value: &str) -> Result<()> {
    set_registry_value(subkey, Some(name), value)
}

unsafe fn set_registry_value(subkey: &str, name: Option<&str>, value: &str) -> Result<()> {
    let mut key = HKEY::default();
    let subkey = wide_null(subkey);
    let status = RegCreateKeyW(HKEY_CLASSES_ROOT, PCWSTR(subkey.as_ptr()), &mut key);
    if status != ERROR_SUCCESS {
        return Err(Error::from_win32());
    }

    let name_wide = name.map(wide_null);
    let value_wide = wide_null(value);
    let value_bytes = std::slice::from_raw_parts(value_wide.as_ptr() as *const u8, value_wide.len() * 2);
    let status = RegSetValueExW(
        key,
        name_wide
            .as_ref()
            .map(|wide| PCWSTR(wide.as_ptr()))
            .unwrap_or_else(PCWSTR::null),
        0,
        REG_SZ,
        Some(value_bytes),
    );
    let _ = RegCloseKey(key);
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::from_win32())
    }
}

fn module_path(module: HINSTANCE) -> Result<String> {
    let mut buffer = [0u16; 32768];
    let len = unsafe { GetModuleFileNameW(module, &mut buffer) } as usize;
    if len == 0 {
        return Err(Error::from_win32());
    }
    Ok(String::from_utf16_lossy(&buffer[..len]))
}

fn guid_braced(guid: &GUID) -> String {
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1,
        guid.data2,
        guid.data3,
        guid.data4[0],
        guid.data4[1],
        guid.data4[2],
        guid.data4[3],
        guid.data4[4],
        guid.data4[5],
        guid.data4[6],
        guid.data4[7]
    )
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[allow(dead_code)]
fn _static_description() -> PCWSTR {
    w!("KhmerIME")
}
