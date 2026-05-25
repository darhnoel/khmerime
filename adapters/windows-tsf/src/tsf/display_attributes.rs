//! TSF display attributes for inline roman preedit styling.

use std::sync::Mutex;

use windows::core::{implement, Error, BSTR, GUID, HRESULT};
use windows::Win32::Foundation::{COLORREF, E_INVALIDARG, FALSE, S_FALSE};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfCategoryMgr,
    ITfDisplayAttributeInfo, ITfDisplayAttributeInfo_Impl, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, TF_ATTR_TARGET_NOTCONVERTED, TF_CT_COLORREF, TF_CT_NONE, TF_CT_SYSCOLOR,
    TF_DA_COLOR, TF_DA_COLORTYPE, TF_DA_COLOR_0, TF_DISPLAYATTRIBUTE, TF_LS_SOLID,
};

use khmerime_session::SegmentPreviewEntry;

pub const GUID_KHMERIME_SEGMENT_FOCUS_ATTR: GUID = GUID::from_u128(0x7f3a4b1d_fbd4_4d5e_9cb3_8b76f6f4fd5a);
pub const GUID_KHMERIME_SEGMENT_EDIT_ATTR: GUID = GUID::from_u128(0x22f2a736_9b3c_4f62_bf1b_3d94cbf09fb3);

const SEGMENT_EDIT_HIGHLIGHT_BG: COLORREF = COLORREF(0x00A3E8FF);
const SEGMENT_EDIT_HIGHLIGHT_FG: COLORREF = COLORREF(0x00271811);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreeditDisplayAttribute {
    SegmentFocus,
    SegmentEdit,
}

impl PreeditDisplayAttribute {
    pub fn guid(self) -> GUID {
        match self {
            Self::SegmentFocus => GUID_KHMERIME_SEGMENT_FOCUS_ATTR,
            Self::SegmentEdit => GUID_KHMERIME_SEGMENT_EDIT_ATTR,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreeditDisplaySpan {
    pub start: usize,
    pub end: usize,
    pub attribute: PreeditDisplayAttribute,
}

impl PreeditDisplaySpan {
    pub fn start_utf16(self, text: &str) -> i32 {
        utf16_len_prefix(text, self.start) as i32
    }

    pub fn end_utf16(self, text: &str) -> i32 {
        utf16_len_prefix(text, self.end) as i32
    }
}

pub fn focused_raw_input_display_span(
    raw_preedit: &str,
    entries: &[SegmentPreviewEntry],
    focused_index: Option<usize>,
    segment_edit_active: bool,
) -> Option<PreeditDisplaySpan> {
    if raw_preedit.is_empty() {
        return None;
    }
    let focused_index = focused_index?;
    if focused_index >= entries.len() {
        return None;
    }

    let inputs = entries.iter().map(|entry| entry.input.trim()).collect::<Vec<_>>();
    if inputs.concat() != raw_preedit {
        return None;
    }

    let focused_input = inputs[focused_index];
    if focused_input.is_empty() {
        return None;
    }
    let start = inputs[..focused_index].iter().map(|input| input.len()).sum::<usize>();
    let end = start + focused_input.len();
    if start == end {
        return None;
    }

    Some(PreeditDisplaySpan {
        start,
        end,
        attribute: if segment_edit_active {
            PreeditDisplayAttribute::SegmentEdit
        } else {
            PreeditDisplayAttribute::SegmentFocus
        },
    })
}

pub fn register_display_attribute_guids() {
    if let Ok(categories) =
        unsafe { CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) }
    {
        let _ = unsafe { categories.RegisterGUID(&GUID_KHMERIME_SEGMENT_FOCUS_ATTR) };
        let _ = unsafe { categories.RegisterGUID(&GUID_KHMERIME_SEGMENT_EDIT_ATTR) };
    }
}

pub fn enum_display_attribute_info() -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
    Ok(KhmerImeDisplayAttributeInfoEnum::new().into())
}

pub fn get_display_attribute_info(guid: *const GUID) -> windows::core::Result<ITfDisplayAttributeInfo> {
    let Some(attribute) = unsafe { guid.as_ref() }.and_then(attribute_from_guid) else {
        return Err(Error::from(E_INVALIDARG));
    };
    Ok(KhmerImeDisplayAttributeInfo::new(attribute).into())
}

#[implement(ITfDisplayAttributeProvider)]
pub struct KhmerImeDisplayAttributeProvider;

impl KhmerImeDisplayAttributeProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ITfDisplayAttributeProvider_Impl for KhmerImeDisplayAttributeProvider_Impl {
    fn EnumDisplayAttributeInfo(&self) -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
        enum_display_attribute_info()
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> windows::core::Result<ITfDisplayAttributeInfo> {
        get_display_attribute_info(guid)
    }
}

#[implement(ITfDisplayAttributeInfo)]
struct KhmerImeDisplayAttributeInfo {
    attribute: PreeditDisplayAttribute,
}

impl KhmerImeDisplayAttributeInfo {
    fn new(attribute: PreeditDisplayAttribute) -> Self {
        Self { attribute }
    }
}

impl ITfDisplayAttributeInfo_Impl for KhmerImeDisplayAttributeInfo_Impl {
    fn GetGUID(&self) -> windows::core::Result<GUID> {
        Ok(self.attribute.guid())
    }

    fn GetDescription(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from(match self.attribute {
            PreeditDisplayAttribute::SegmentFocus => "KhmerIME focused segment",
            PreeditDisplayAttribute::SegmentEdit => "KhmerIME segment edit",
        }))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> windows::core::Result<()> {
        if pda.is_null() {
            return Err(Error::from(E_INVALIDARG));
        }
        unsafe {
            *pda = display_attribute_info(self.attribute);
        }
        Ok(())
    }

    fn SetAttributeInfo(&self, _pda: *const TF_DISPLAYATTRIBUTE) -> windows::core::Result<()> {
        Ok(())
    }

    fn Reset(&self) -> windows::core::Result<()> {
        Ok(())
    }
}

#[implement(IEnumTfDisplayAttributeInfo)]
struct KhmerImeDisplayAttributeInfoEnum {
    index: Mutex<usize>,
}

impl KhmerImeDisplayAttributeInfoEnum {
    fn new() -> Self {
        Self { index: Mutex::new(0) }
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for KhmerImeDisplayAttributeInfoEnum_Impl {
    fn Clone(&self) -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
        let current = self.index.lock().map(|index| *index).unwrap_or(0);
        Ok(KhmerImeDisplayAttributeInfoEnum {
            index: Mutex::new(current),
        }
        .into())
    }

    fn Next(
        &self,
        ulcount: u32,
        rginfo: *mut Option<ITfDisplayAttributeInfo>,
        pcfetched: *mut u32,
    ) -> windows::core::Result<()> {
        let attributes = [
            PreeditDisplayAttribute::SegmentFocus,
            PreeditDisplayAttribute::SegmentEdit,
        ];
        let mut index = self.index.lock().map_err(|_| Error::from(E_INVALIDARG))?;
        let mut fetched = 0usize;
        while fetched < ulcount as usize && *index < attributes.len() {
            unsafe {
                rginfo
                    .add(fetched)
                    .write(Some(KhmerImeDisplayAttributeInfo::new(attributes[*index]).into()));
            }
            *index += 1;
            fetched += 1;
        }
        if !pcfetched.is_null() {
            unsafe {
                *pcfetched = fetched as u32;
            }
        }
        if fetched == ulcount as usize {
            Ok(())
        } else {
            Err(Error::new(HRESULT(S_FALSE.0), "no more display attributes"))
        }
    }

    fn Reset(&self) -> windows::core::Result<()> {
        if let Ok(mut index) = self.index.lock() {
            *index = 0;
        }
        Ok(())
    }

    fn Skip(&self, ulcount: u32) -> windows::core::Result<()> {
        if let Ok(mut index) = self.index.lock() {
            *index = (*index + ulcount as usize).min(2);
        }
        Ok(())
    }
}

fn display_attribute_info(attribute: PreeditDisplayAttribute) -> TF_DISPLAYATTRIBUTE {
    let mut info = TF_DISPLAYATTRIBUTE {
        crText: none_color(),
        crBk: none_color(),
        lsStyle: TF_LS_SOLID,
        fBoldLine: FALSE,
        crLine: sys_color(8),
        bAttr: TF_ATTR_TARGET_NOTCONVERTED,
    };

    if attribute == PreeditDisplayAttribute::SegmentEdit {
        info.crText = colorref(SEGMENT_EDIT_HIGHLIGHT_FG);
        info.crBk = colorref(SEGMENT_EDIT_HIGHLIGHT_BG);
    }
    info
}

fn attribute_from_guid(guid: &GUID) -> Option<PreeditDisplayAttribute> {
    match *guid {
        GUID_KHMERIME_SEGMENT_FOCUS_ATTR => Some(PreeditDisplayAttribute::SegmentFocus),
        GUID_KHMERIME_SEGMENT_EDIT_ATTR => Some(PreeditDisplayAttribute::SegmentEdit),
        _ => None,
    }
}

fn none_color() -> TF_DA_COLOR {
    da_color(TF_CT_NONE, 0)
}

fn sys_color(index: i32) -> TF_DA_COLOR {
    da_color(TF_CT_SYSCOLOR, index)
}

fn colorref(color: COLORREF) -> TF_DA_COLOR {
    TF_DA_COLOR {
        r#type: TF_CT_COLORREF,
        Anonymous: TF_DA_COLOR_0 { cr: color },
    }
}

fn da_color(kind: TF_DA_COLORTYPE, value: i32) -> TF_DA_COLOR {
    TF_DA_COLOR {
        r#type: kind,
        Anonymous: TF_DA_COLOR_0 { nIndex: value },
    }
}

fn utf16_len_prefix(text: &str, byte_end: usize) -> usize {
    text.get(..byte_end).unwrap_or("").encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(input: &str, focused: bool) -> SegmentPreviewEntry {
        SegmentPreviewEntry {
            output: input.to_owned(),
            input: input.to_owned(),
            focused,
        }
    }

    #[test]
    fn focused_raw_input_display_span_matches_ibus_span_rules() {
        let span = focused_raw_input_display_span(
            "nihjeasnadai",
            &[segment("nih", false), segment("jea", true), segment("snadai", false)],
            Some(1),
            false,
        );

        assert_eq!(
            span,
            Some(PreeditDisplaySpan {
                start: 3,
                end: 6,
                attribute: PreeditDisplayAttribute::SegmentFocus,
            })
        );
    }

    #[test]
    fn focused_raw_input_display_span_uses_edit_attribute_in_segment_edit_mode() {
        let span =
            focused_raw_input_display_span("nihjea", &[segment("nih", false), segment("jea", true)], Some(1), true)
                .expect("focused span");

        assert_eq!(span.attribute, PreeditDisplayAttribute::SegmentEdit);
    }

    #[test]
    fn focused_raw_input_display_span_requires_exact_raw_concatenation() {
        let span = focused_raw_input_display_span(
            "nihxjea",
            &[segment("nih", false), segment("jea", true)],
            Some(1),
            false,
        );

        assert_eq!(span, None);
    }
}
