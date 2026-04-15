#[cfg(not(target_arch = "wasm32"))]
use dioxus::document;

#[cfg(target_arch = "wasm32")]
use web_sys::wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{window, CssStyleDeclaration, Document, Element, HtmlElement, HtmlTextAreaElement};

use crate::{CompositionMark, SuggestionPopup, EDITOR_ID};

#[cfg(any(not(target_arch = "wasm32"), test))]
const CHAR_WIDTH_DIVISOR: f64 = 0.62;
#[cfg(any(not(target_arch = "wasm32"), test))]
const CHAR_WIDTH_MULTIPLIER: f64 = 0.58;
const POPUP_HORIZONTAL_OFFSET: f64 = 18.0;
const POPUP_VERTICAL_OFFSET: f64 = 10.0;
const POPUP_SAFE_MARGIN: f64 = 8.0;
const POPUP_WIDTH: f64 = 280.0;
const POPUP_HEIGHT: f64 = 220.0;
#[cfg(any(not(target_arch = "wasm32"), test))]
const COMPOSITION_MIN_WIDTH: f64 = 12.0;

#[cfg(target_arch = "wasm32")]
const MIRROR_STYLE_PROPS: &[&str] = &[
    "box-sizing",
    "width",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "line-height",
    "letter-spacing",
    "text-transform",
    "text-indent",
    "white-space",
    "word-spacing",
];

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn preferred_popup_top(caret_top: f64, line_height: f64, client_height: f64) -> f64 {
    let below = caret_top + line_height + POPUP_VERTICAL_OFFSET;
    let below_fits = below + POPUP_HEIGHT <= client_height - POPUP_SAFE_MARGIN;
    if below_fits {
        return below;
    }
    caret_top - POPUP_VERTICAL_OFFSET - POPUP_HEIGHT
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn estimated_chars_per_line(client_width: f64, font_size: f64) -> usize {
    let slot_width = (font_size * CHAR_WIDTH_DIVISOR).max(1.0);
    ((client_width / slot_width).floor() as usize).max(1)
}

fn clamped_popup_position(left: f64, top: f64, client_width: f64, client_height: f64) -> SuggestionPopup {
    let max_left = (client_width - POPUP_WIDTH - POPUP_SAFE_MARGIN).max(POPUP_SAFE_MARGIN);
    let max_top = (client_height - POPUP_HEIGHT - POPUP_SAFE_MARGIN).max(POPUP_SAFE_MARGIN);
    SuggestionPopup {
        left: clamp(left, POPUP_SAFE_MARGIN, max_left),
        top: clamp(top, POPUP_SAFE_MARGIN, max_top),
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn estimated_popup_position(
    caret: usize,
    client_width: f64,
    client_height: f64,
    font_size: f64,
    line_height: f64,
) -> SuggestionPopup {
    let chars_per_line = estimated_chars_per_line(client_width, font_size);
    let left = POPUP_HORIZONTAL_OFFSET + ((caret % chars_per_line) as f64 * (font_size * CHAR_WIDTH_MULTIPLIER));
    let caret_top = POPUP_HORIZONTAL_OFFSET + ((caret / chars_per_line) as f64 * line_height);
    let top = preferred_popup_top(caret_top, line_height, client_height);
    clamped_popup_position(left, top, client_width, client_height)
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn estimated_composition_mark(
    start: usize,
    token_len: usize,
    client_width: f64,
    font_size: f64,
    line_height: f64,
) -> CompositionMark {
    let chars_per_line = estimated_chars_per_line(client_width, font_size);
    CompositionMark {
        left: (start % chars_per_line) as f64 * (font_size * CHAR_WIDTH_MULTIPLIER),
        top: (start / chars_per_line) as f64 * line_height,
        width: (token_len as f64 * font_size * CHAR_WIDTH_MULTIPLIER).max(COMPOSITION_MIN_WIDTH),
        height: line_height,
    }
}

#[cfg(any(not(target_arch = "wasm32"), test))]
struct PopupEstimateMetrics {
    client_width: f64,
    client_height: f64,
    font_size: f64,
    line_height: f64,
}

#[cfg(any(not(target_arch = "wasm32"), test))]
fn parse_popup_estimate_metrics(raw: &str) -> Option<PopupEstimateMetrics> {
    let mut parts = raw.split(',');
    Some(PopupEstimateMetrics {
        client_width: parts.next()?.trim().parse().ok()?,
        client_height: parts.next()?.trim().parse().ok()?,
        font_size: parts.next()?.trim().parse().ok()?,
        line_height: parts.next()?.trim().parse().ok()?,
    })
}

#[cfg(any(target_arch = "wasm32", test))]
struct PopupMarkerMetrics {
    marker_left: f64,
    marker_top: f64,
    mirror_left: f64,
    mirror_top: f64,
    scroll_left: f64,
    scroll_top: f64,
    line_height: f64,
    client_width: f64,
    client_height: f64,
}

#[cfg(test)]
fn parse_popup_marker_metrics(raw: &str) -> Option<PopupMarkerMetrics> {
    let mut parts = raw.split(',');
    Some(PopupMarkerMetrics {
        marker_left: parts.next()?.trim().parse().ok()?,
        marker_top: parts.next()?.trim().parse().ok()?,
        mirror_left: parts.next()?.trim().parse().ok()?,
        mirror_top: parts.next()?.trim().parse().ok()?,
        scroll_left: parts.next()?.trim().parse().ok()?,
        scroll_top: parts.next()?.trim().parse().ok()?,
        line_height: parts.next()?.trim().parse().ok()?,
        client_width: parts.next()?.trim().parse().ok()?,
        client_height: parts.next()?.trim().parse().ok()?,
    })
}

#[cfg(any(target_arch = "wasm32", test))]
fn popup_position_from_marker_metrics(metrics: PopupMarkerMetrics) -> SuggestionPopup {
    let left = metrics.marker_left - metrics.mirror_left - metrics.scroll_left + POPUP_HORIZONTAL_OFFSET;
    let caret_top = metrics.marker_top - metrics.mirror_top - metrics.scroll_top;
    let top = preferred_popup_top(caret_top, metrics.line_height, metrics.client_height);
    clamped_popup_position(left, top, metrics.client_width, metrics.client_height)
}

#[cfg(any(target_arch = "wasm32", test))]
struct CompositionMarkerMetrics {
    marker_left: f64,
    marker_top: f64,
    marker_width: f64,
    marker_height: f64,
    mirror_left: f64,
    mirror_top: f64,
    scroll_left: f64,
    scroll_top: f64,
}

#[cfg(test)]
fn parse_composition_marker_metrics(raw: &str) -> Option<CompositionMarkerMetrics> {
    let mut parts = raw.split(',');
    Some(CompositionMarkerMetrics {
        marker_left: parts.next()?.trim().parse().ok()?,
        marker_top: parts.next()?.trim().parse().ok()?,
        marker_width: parts.next()?.trim().parse().ok()?,
        marker_height: parts.next()?.trim().parse().ok()?,
        mirror_left: parts.next()?.trim().parse().ok()?,
        mirror_top: parts.next()?.trim().parse().ok()?,
        scroll_left: parts.next()?.trim().parse().ok()?,
        scroll_top: parts.next()?.trim().parse().ok()?,
    })
}

#[cfg(any(target_arch = "wasm32", test))]
fn composition_mark_from_marker_metrics(metrics: CompositionMarkerMetrics) -> CompositionMark {
    CompositionMark {
        left: metrics.marker_left - metrics.mirror_left - metrics.scroll_left,
        top: metrics.marker_top - metrics.mirror_top - metrics.scroll_top,
        width: metrics.marker_width,
        height: metrics.marker_height,
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_document() -> Option<Document> {
    window()?.document()
}

#[cfg(target_arch = "wasm32")]
fn editor_textarea() -> Option<HtmlTextAreaElement> {
    browser_document()?
        .get_element_by_id(EDITOR_ID)?
        .dyn_into::<HtmlTextAreaElement>()
        .ok()
}

#[cfg(target_arch = "wasm32")]
fn computed_style_for(element: &Element) -> Option<CssStyleDeclaration> {
    window()?.get_computed_style(element).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
fn parse_css_f64(style: &CssStyleDeclaration, property: &str) -> Option<f64> {
    style
        .get_property_value(property)
        .ok()?
        .trim_end_matches("px")
        .trim()
        .parse()
        .ok()
}

#[cfg(target_arch = "wasm32")]
fn line_height_for(style: &CssStyleDeclaration) -> f64 {
    parse_css_f64(style, "line-height")
        .or_else(|| parse_css_f64(style, "font-size").map(|font_size| font_size * 1.5))
        .unwrap_or(32.0)
}

#[cfg(target_arch = "wasm32")]
fn utf16_index_to_char_index(value: &str, utf16_index: u32) -> usize {
    let mut units = 0u32;
    for (char_index, ch) in value.chars().enumerate() {
        if units >= utf16_index {
            return char_index;
        }
        units += ch.len_utf16() as u32;
        if units > utf16_index {
            return char_index + 1;
        }
    }
    value.chars().count()
}

#[cfg(target_arch = "wasm32")]
fn char_index_to_utf16_index(value: &str, char_index: usize) -> u32 {
    value.chars().take(char_index).map(|ch| ch.len_utf16() as u32).sum()
}

#[cfg(target_arch = "wasm32")]
fn prefix_chars(value: &str, char_count: usize) -> String {
    value.chars().take(char_count).collect()
}

#[cfg(target_arch = "wasm32")]
fn char_at_or_dot(value: &str, char_index: usize) -> String {
    value
        .chars()
        .nth(char_index)
        .map(|ch| ch.to_string())
        .unwrap_or_else(|| ".".to_owned())
}

#[cfg(target_arch = "wasm32")]
fn build_measurement_mirror(
    document: &Document,
    editor: &HtmlTextAreaElement,
    style: &CssStyleDeclaration,
) -> Option<HtmlElement> {
    let mirror = document.create_element("div").ok()?.dyn_into::<HtmlElement>().ok()?;
    let mirror_style = mirror.style();
    mirror_style.set_property("position", "absolute").ok()?;
    mirror_style.set_property("visibility", "hidden").ok()?;
    mirror_style.set_property("white-space", "pre-wrap").ok()?;
    mirror_style.set_property("word-wrap", "break-word").ok()?;
    mirror_style.set_property("left", "-9999px").ok()?;
    mirror_style.set_property("top", "0").ok()?;
    for prop in MIRROR_STYLE_PROPS {
        let value = style.get_property_value(prop).ok()?;
        mirror_style.set_property(prop, &value).ok()?;
    }
    mirror_style
        .set_property("width", &format!("{}px", editor.client_width()))
        .ok()?;
    Some(mirror)
}

#[cfg(target_arch = "wasm32")]
fn append_measurement_nodes(mirror: &HtmlElement, marker: &Element) -> Option<web_sys::HtmlElement> {
    mirror.append_child(marker).ok()?;
    let body = browser_document()?.body()?;
    body.append_child(mirror).ok()?;
    Some(body)
}

#[cfg(target_arch = "wasm32")]
fn remove_measurement_nodes(body: &HtmlElement, mirror: &HtmlElement) {
    let _ = body.remove_child(mirror);
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn current_editor_caret() -> Option<usize> {
    let editor = editor_textarea()?;
    let utf16_index = editor.selection_start().ok().flatten()?;
    Some(utf16_index_to_char_index(&editor.value(), utf16_index))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn current_editor_caret() -> Option<usize> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return 0;
            return typeof el.selectionStart === "number" ? el.selectionStart : (el.value ? el.value.length : 0);
        "#,
        editor_id = EDITOR_ID,
    );
    document::eval(&script).join::<usize>().await.ok()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_editor_caret(caret: usize) {
    let Some(editor) = editor_textarea() else {
        return;
    };
    let cursor = char_index_to_utf16_index(&editor.value(), caret.min(editor.value().chars().count()));
    let _ = editor.focus();
    let _ = editor.set_selection_range(cursor, cursor);
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn set_editor_caret(caret: usize) {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (el) {{
                el.focus();
                if (typeof el.setSelectionRange === "function") {{
                    el.setSelectionRange({caret}, {caret});
                }}
            }}
        "#,
        editor_id = EDITOR_ID,
        caret = caret,
    );
    document::eval(&script);
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn editor_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    let document = browser_document()?;
    let editor = editor_textarea()?;
    let editor_element: Element = editor.clone().unchecked_into();
    let style = computed_style_for(&editor_element)?;
    let mirror = build_measurement_mirror(&document, &editor, &style)?;
    mirror.set_text_content(Some(&prefix_chars(&editor.value(), start)));

    let marker = document.create_element("span").ok()?;
    marker.set_text_content(Some(token));
    let body = append_measurement_nodes(&mirror, &marker)?;

    let mirror_rect = mirror.get_bounding_client_rect();
    let marker_rect = marker.get_bounding_client_rect();
    let metrics = CompositionMarkerMetrics {
        marker_left: marker_rect.left(),
        marker_top: marker_rect.top(),
        marker_width: marker_rect.width(),
        marker_height: marker_rect.height(),
        mirror_left: mirror_rect.left(),
        mirror_top: mirror_rect.top(),
        scroll_left: editor.scroll_left() as f64,
        scroll_top: editor.scroll_top() as f64,
    };
    remove_measurement_nodes(&body, &mirror);
    Some(composition_mark_from_marker_metrics(metrics))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn editor_composition_mark(start: usize, token: &str) -> Option<CompositionMark> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return "";
            const style = getComputedStyle(el);
            const fontSize = parseFloat(style.fontSize) || 24;
            const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.5;
            return `${{el.clientWidth}},${{fontSize}},${{lineHeight}}`;
        "#,
        editor_id = EDITOR_ID,
    );
    let raw = document::eval(&script).join::<String>().await.ok()?;
    let mut parts = raw.split(',');
    let client_width = parts.next()?.trim().parse().ok()?;
    let font_size = parts.next()?.trim().parse().ok()?;
    let line_height = parts.next()?.trim().parse().ok()?;
    Some(estimated_composition_mark(
        start,
        token.chars().count(),
        client_width,
        font_size,
        line_height,
    ))
}

#[cfg(target_arch = "wasm32")]
pub(crate) async fn editor_popup_position(caret: usize) -> Option<SuggestionPopup> {
    let document = browser_document()?;
    let editor = editor_textarea()?;
    let editor_element: Element = editor.clone().unchecked_into();
    let style = computed_style_for(&editor_element)?;
    let mirror = build_measurement_mirror(&document, &editor, &style)?;
    let raw = editor.value();
    mirror.set_text_content(Some(&prefix_chars(&raw, caret)));

    let marker = document.create_element("span").ok()?;
    marker.set_text_content(Some(&char_at_or_dot(&raw, caret)));
    let body = append_measurement_nodes(&mirror, &marker)?;

    let mirror_rect = mirror.get_bounding_client_rect();
    let marker_rect = marker.get_bounding_client_rect();
    let metrics = PopupMarkerMetrics {
        marker_left: marker_rect.left(),
        marker_top: marker_rect.top(),
        mirror_left: mirror_rect.left(),
        mirror_top: mirror_rect.top(),
        scroll_left: editor.scroll_left() as f64,
        scroll_top: editor.scroll_top() as f64,
        line_height: line_height_for(&style),
        client_width: editor.client_width() as f64,
        client_height: editor.client_height() as f64,
    };
    remove_measurement_nodes(&body, &mirror);
    Some(popup_position_from_marker_metrics(metrics))
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn editor_popup_position(caret: usize) -> Option<SuggestionPopup> {
    let script = format!(
        r#"
            const el = document.getElementById({editor_id:?});
            if (!el) return "";
            const style = getComputedStyle(el);
            const fontSize = parseFloat(style.fontSize) || 24;
            const lineHeight = parseFloat(style.lineHeight) || fontSize * 1.5;
            return `${{el.clientWidth}},${{el.clientHeight}},${{fontSize}},${{lineHeight}}`;
        "#,
        editor_id = EDITOR_ID,
    );
    let raw = document::eval(&script).join::<String>().await.ok()?;
    let metrics = parse_popup_estimate_metrics(&raw)?;
    Some(estimated_popup_position(
        caret,
        metrics.client_width,
        metrics.client_height,
        metrics.font_size,
        metrics.line_height,
    ))
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn mark_app_ready() {
    let Some(document) = browser_document() else {
        return;
    };
    if let Some(body) = document.body() {
        let _ = body.set_attribute("data-app-ready", "1");
    }
    if let Some(splash) = document.get_element_by_id("app-preboot-splash") {
        let _ = splash.set_attribute("data-hidden", "true");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn mark_app_ready() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimates_popup_position_in_rust() {
        let popup = estimated_popup_position(12, 640.0, 480.0, 24.0, 36.0);
        assert!(popup.left >= POPUP_SAFE_MARGIN);
        assert!(popup.top >= POPUP_SAFE_MARGIN);
        assert!(popup.left <= 640.0 - POPUP_WIDTH - POPUP_SAFE_MARGIN);
        assert!(popup.top <= 480.0 - POPUP_HEIGHT - POPUP_SAFE_MARGIN);
    }

    #[test]
    fn clamps_marker_based_popup_position_in_rust() {
        let popup = popup_position_from_marker_metrics(PopupMarkerMetrics {
            marker_left: 900.0,
            marker_top: 600.0,
            mirror_left: 100.0,
            mirror_top: 100.0,
            scroll_left: 0.0,
            scroll_top: 0.0,
            line_height: 36.0,
            client_width: 640.0,
            client_height: 480.0,
        });
        assert_eq!(popup.left, 352.0);
        assert_eq!(popup.top, 252.0);
    }

    #[test]
    fn prefers_popup_above_caret_when_bottom_space_is_tight() {
        let popup = popup_position_from_marker_metrics(PopupMarkerMetrics {
            marker_left: 160.0,
            marker_top: 350.0,
            mirror_left: 100.0,
            mirror_top: 100.0,
            scroll_left: 0.0,
            scroll_top: 0.0,
            line_height: 36.0,
            client_width: 380.0,
            client_height: 320.0,
        });
        // caret_top = 250, above = 20, below would overflow the viewport.
        assert_eq!(popup.top, 20.0);
    }

    #[test]
    fn estimates_composition_mark_in_rust() {
        let mark = estimated_composition_mark(18, 4, 600.0, 24.0, 36.0);
        assert!(mark.width >= COMPOSITION_MIN_WIDTH);
        assert_eq!(mark.height, 36.0);
    }

    #[test]
    fn builds_composition_mark_from_dom_measurements_in_rust() {
        let mark = composition_mark_from_marker_metrics(CompositionMarkerMetrics {
            marker_left: 180.0,
            marker_top: 240.0,
            marker_width: 54.0,
            marker_height: 32.0,
            mirror_left: 100.0,
            mirror_top: 200.0,
            scroll_left: 8.0,
            scroll_top: 4.0,
        });
        assert_eq!(mark.left, 72.0);
        assert_eq!(mark.top, 36.0);
        assert_eq!(mark.width, 54.0);
        assert_eq!(mark.height, 32.0);
    }

    #[test]
    fn parses_popup_marker_metrics() {
        let metrics = parse_popup_marker_metrics("10,20,1,2,3,4,5,6,7").expect("metrics");
        assert_eq!(metrics.marker_left, 10.0);
        assert_eq!(metrics.client_height, 7.0);
    }

    #[test]
    fn parses_composition_marker_metrics() {
        let metrics = parse_composition_marker_metrics("10,20,30,40,1,2,3,4").expect("metrics");
        assert_eq!(metrics.marker_width, 30.0);
        assert_eq!(metrics.scroll_top, 4.0);
    }
}
