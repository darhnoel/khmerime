use dioxus::prelude::*;
use roman_lookup::ShadowObservation;

#[component]
pub(crate) fn ShadowPanel(debug: Option<ShadowObservation>) -> Element {
    rsx! {
        div { class: "guide-card debug-card shadow-side-panel",
            "data-testid": "shadow-panel",
            div { class: "card-head",
                div {
                    h2 { "Shadow Compare" }
                    p { "Legacy suggestions remain visible. This panel shows the shadow WFST comparison for the active token." }
                }
            }
            if let Some(debug) = debug {
                div { class: "debug-grid",
                    div { class: "debug-row",
                        span { class: "debug-label", "Input" }
                        code { class: "debug-value", {debug.input.clone()} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "Mismatch" }
                        code { class: "debug-value", {debug.mismatch.as_str()} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "Legacy Top" }
                        span { class: "debug-value", {debug.legacy_top.clone().unwrap_or_else(|| "-".to_owned())} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "WFST Top" }
                        span { class: "debug-value", {debug.wfst_top.clone().unwrap_or_else(|| "-".to_owned())} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "WFST Failure" }
                        code { class: "debug-value", {debug.wfst_failure.clone().unwrap_or_else(|| "-".to_owned())} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "Legacy Top-5" }
                        span { class: "debug-value", {debug.legacy_top5.join(" | ")} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "WFST Top-5" }
                        span { class: "debug-value", {debug.wfst_top5.join(" | ")} }
                    }
                    div { class: "debug-row",
                        span { class: "debug-label", "Latency" }
                        code { class: "debug-value", {format!(
                            "legacy {}us / wfst {}us",
                            debug.legacy_latency_us,
                            debug.wfst_latency_us
                                .map(|value| value.to_string())
                                .unwrap_or_else(|| "-".to_owned())
                        )} }
                    }
                }
            } else {
                div { class: "empty-state",
                    p { "No active token" }
                    span { "Type a roman token to inspect the shadow comparison." }
                }
            }
        }
    }
}
