use dioxus::prelude::*;
use roman_lookup::{DecoderMode, ShadowObservation};

use crate::ui::shadow::ShadowPanel;

#[component]
pub(crate) fn WorkspaceBody(
    roman_enabled: bool,
    decoder_mode: DecoderMode,
    shadow_debug: Option<ShadowObservation>,
    editor_card: Element,
) -> Element {
    rsx! {
        div {
            class: if roman_enabled && decoder_mode == DecoderMode::Shadow {
                "workspace-body workspace-body-shadow"
            } else {
                "workspace-body"
            },
            if roman_enabled && decoder_mode == DecoderMode::Shadow {
                ShadowPanel { debug: shadow_debug }
            }
            {editor_card}
        }
    }
}
