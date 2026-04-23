use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation};

#[component]
pub(crate) fn WorkspaceBody(
    roman_enabled: bool,
    decoder_mode: DecoderMode,
    shadow_debug: Option<ShadowObservation>,
    editor_card: Element,
) -> Element {
    let _ = (roman_enabled, decoder_mode, shadow_debug);
    rsx! {
        div {
            class: "workspace-body",
            {editor_card}
        }
    }
}
