use dioxus::html::InteractionLocation;
use dioxus::prelude::*;

use crate::ui::editor::{resolve, Direction, Key, Preedit, KEYMAP};

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveFlick {
    row: usize,
    column: usize,
    pointer_id: i32,
    start_x: f64,
    start_y: f64,
    direction: Direction,
}

fn direction_class(direction: Direction) -> &'static str {
    match direction {
        Direction::Center => "center",
        Direction::Up => "up",
        Direction::Left => "left",
        Direction::Right => "right",
        Direction::Down => "down",
    }
}

fn preview_member_class(direction: Direction, selected: Direction) -> String {
    if direction == selected {
        format!("flick-preview-member {} selected", direction_class(direction))
    } else {
        format!("flick-preview-member {}", direction_class(direction))
    }
}

fn capture_pointer(element_id: &str, pointer_id: i32) {
    let script = format!(
        r#"
        document.getElementById({element_id:?})?.setPointerCapture?.({pointer_id});
        "#,
    );
    let _ = dioxus::document::eval(&script);
}

fn insert_member(member: &'static str, mut preedit: Signal<Preedit>) {
    if member.is_empty() {
        return;
    }
    let mut next_preedit = preedit();
    next_preedit.push(member);
    preedit.set(next_preedit);
}

fn flick_backspace(mut preedit: Signal<Preedit>) {
    let mut next_preedit = preedit();
    if next_preedit.backspace() {
        preedit.set(next_preedit);
    }
}

fn key_family_label(key: &Key) -> String {
    [key.center, key.up, key.left, key.right, key.down]
        .into_iter()
        .filter(|member| !member.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[component]
pub(crate) fn FlickKeyboard(preedit: Signal<Preedit>) -> Element {
    let mut active = use_signal(|| None::<ActiveFlick>);

    rsx! {
        section { class: "flick-keyboard", "data-testid": "flick-keyboard", aria_label: "ក្តារចុចខ្មែរ",
            div { class: "flick-toolbar",
                div { class: "flick-toolbar-copy",
                    strong { "ក្តារចុចខ្មែរ" }
                    span { "ប៉ះ ឬអូសទៅទិសដើម្បីជ្រើសតួអក្សរ" }
                }
            }

            div { class: "flick-grid",
                for (row_index, row) in KEYMAP.iter().enumerate() {
                    div { class: "flick-row", key: "flick-row-{row_index}",
                        for (column_index, key_ref) in row.iter().enumerate() {
                            {
                                let key_def = *key_ref;
                                let key_id = format!("flick-key-{row_index}-{column_index}");
                                let pressed = active().filter(|gesture| {
                                    gesture.row == row_index && gesture.column == column_index
                                });
                                let selected_direction = pressed
                                    .map(|gesture| gesture.direction)
                                    .unwrap_or(Direction::Center);
                                rsx! {
                                    button {
                                        id: "{key_id}",
                                        key: "key-{row_index}-{column_index}",
                                        class: if pressed.is_some() { "flick-key pressed" } else { "flick-key" },
                                        "data-testid": "flick-key-{row_index}-{column_index}",
                                        aria_label: "{key_family_label(&key_def)}",
                                        aria_pressed: "{pressed.is_some()}",
                                        onpointerdown: {
                                            let key_id = key_id.clone();
                                            move |event| {
                                                event.prevent_default();
                                                let point = event.client_coordinates();
                                                let pointer_id = event.pointer_id();
                                                active.set(Some(ActiveFlick {
                                                    row: row_index,
                                                    column: column_index,
                                                    pointer_id,
                                                    start_x: point.x,
                                                    start_y: point.y,
                                                    direction: Direction::Center,
                                                }));
                                                capture_pointer(&key_id, pointer_id);
                                            }
                                        },
                                        onpointermove: move |event| {
                                            let Some(mut gesture) = active() else { return; };
                                            if gesture.pointer_id != event.pointer_id()
                                                || gesture.row != row_index
                                                || gesture.column != column_index
                                            {
                                                return;
                                            }
                                            let point = event.client_coordinates();
                                            let direction = resolve(
                                                &key_def,
                                                point.x - gesture.start_x,
                                                point.y - gesture.start_y,
                                            );
                                            if direction != gesture.direction {
                                                gesture.direction = direction;
                                                active.set(Some(gesture));
                                            }
                                        },
                                        onpointerup: move |event| {
                                            event.prevent_default();
                                            let Some(gesture) = active() else { return; };
                                            if gesture.pointer_id != event.pointer_id()
                                                || gesture.row != row_index
                                                || gesture.column != column_index
                                            {
                                                return;
                                            }
                                            let point = event.client_coordinates();
                                            let direction = resolve(
                                                &key_def,
                                                point.x - gesture.start_x,
                                                point.y - gesture.start_y,
                                            );
                                            active.set(None);
                                            let member = key_def.member(direction);
                                            insert_member(member, preedit);
                                        },
                                        onpointercancel: move |_| active.set(None),
                                        span { class: "flick-key-center", "{key_def.center}" }
                                        span { class: "flick-key-hint up", "{key_def.up}" }
                                        span { class: "flick-key-hint left", "{key_def.left}" }
                                        span { class: "flick-key-hint right", "{key_def.right}" }
                                        span { class: "flick-key-hint down", "{key_def.down}" }

                                        if pressed.is_some() {
                                            div { class: "flick-preview", aria_hidden: "true",
                                                if !key_def.center.is_empty() {
                                                    span { class: "{preview_member_class(Direction::Center, selected_direction)}", "{key_def.center}" }
                                                }
                                                if !key_def.up.is_empty() {
                                                    span { class: "{preview_member_class(Direction::Up, selected_direction)}", "{key_def.up}" }
                                                }
                                                if !key_def.left.is_empty() {
                                                    span { class: "{preview_member_class(Direction::Left, selected_direction)}", "{key_def.left}" }
                                                }
                                                if !key_def.right.is_empty() {
                                                    span { class: "{preview_member_class(Direction::Right, selected_direction)}", "{key_def.right}" }
                                                }
                                                if !key_def.down.is_empty() {
                                                    span { class: "{preview_member_class(Direction::Down, selected_direction)}", "{key_def.down}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "flick-actions",
                button { class: "flick-action secondary", disabled: true, "123" }
                button {
                    class: "flick-action delete",
                    "data-testid": "flick-backspace",
                    aria_label: "លុបថយក្រោយ",
                    onpointerdown: move |event| {
                        event.prevent_default();
                        flick_backspace(preedit);
                    },
                    "⌫"
                }
            }
        }
    }
}
