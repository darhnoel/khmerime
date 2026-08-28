use crate::ui::editor::{
    click_candidate, commit_active_selection, composition_style, enter_segment_edit, exit_segment_edit, is_space_key,
    move_segment_focus, popup_style, select_segment_candidate, shortcut_index, shortcut_label, should_exit_number_pick,
    update_candidates, visible_page_start, CandidateLevel, CandidateMode, EditorSignals,
};
use crate::ui::platform::{copy_to_clipboard, move_editor_caret, schedule_spell_popover_placement};
use crate::ui::spellcheck::{
    ContextDetectorStatus, SpellIssue, SpellIssueKind, SpellReview, SpellSegment,
};
use crate::ui::storage::{save_editor_text, save_enabled};
use crate::{EDITOR_ID, VISIBLE_SUGGESTIONS};
use dioxus::html::Modifiers;
use dioxus::prelude::*;

#[cfg(test)]
mod tests {
    use super::space_cycle_enabled;
    use crate::ui::editor::CandidateMode;

    #[test]
    fn space_cycle_is_disabled_for_next_word_mode() {
        assert!(!space_cycle_enabled(true, CandidateMode::NextWord));
    }

    #[test]
    fn space_cycle_is_enabled_for_transliteration_mode() {
        assert!(space_cycle_enabled(true, CandidateMode::Transliteration));
    }
}

fn roman_hint_label(variants: &[String]) -> String {
    // Show all variants if there are 3 or fewer, otherwise show the first 3 followed by ellipsis.
    format!("{}", variants.join(" / "))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SpellPiece {
    Plain(String),
    Issue { index: usize, text: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SegmentPiece {
    Plain(String),
    Segment { index: usize, text: String },
}

fn spell_pieces(text: &str, issues: &[SpellIssue]) -> Vec<SpellPiece> {
    let mut pieces = Vec::new();
    let mut cursor = 0;
    for (index, issue) in issues.iter().enumerate() {
        if issue.start > cursor {
            pieces.push(SpellPiece::Plain(
                text.chars().skip(cursor).take(issue.start - cursor).collect(),
            ));
        }
        pieces.push(SpellPiece::Issue {
            index,
            text: text
                .chars()
                .skip(issue.start)
                .take(issue.end.saturating_sub(issue.start))
                .collect(),
        });
        cursor = issue.end;
    }
    if cursor < text.chars().count() {
        pieces.push(SpellPiece::Plain(text.chars().skip(cursor).collect()));
    }
    pieces
}

fn segment_pieces(text: &str, segments: &[SpellSegment]) -> Vec<SegmentPiece> {
    let mut pieces = Vec::new();
    let mut cursor = 0;
    for (index, segment) in segments.iter().enumerate() {
        if segment.start > cursor {
            pieces.push(SegmentPiece::Plain(
                text.chars().skip(cursor).take(segment.start - cursor).collect(),
            ));
        }
        pieces.push(SegmentPiece::Segment {
            index,
            text: text
                .chars()
                .skip(segment.start)
                .take(segment.end.saturating_sub(segment.start))
                .collect(),
        });
        cursor = segment.end;
    }
    if cursor < text.chars().count() {
        pieces.push(SegmentPiece::Plain(text.chars().skip(cursor).collect()));
    }
    pieces
}

fn install_spell_popover_placement() {
    let _ = dioxus::document::eval(
        r#"
        (() => {
            if (window.__khmerImeSpellPopoverPlacementVersion === 2) return;
            window.__khmerImeSpellPopoverPlacementVersion = 2;
            window.__khmerImeSpellPopoverPlacementInstalled = true;

            const position = (popover) => {
                requestAnimationFrame(() => {
                    if (!popover.isConnected) return;
                    const target = popover.closest('[data-testid="spell-issue"]');
                    const card = target?.closest('.editor-card');
                    if (!target || !card) return;

                    popover.classList.remove('above');
                    popover.style.setProperty('--spell-shift-x', '0px');
                    const cardRect = card.getBoundingClientRect();
                    const popoverRect = popover.getBoundingClientRect();
                    const safeLeft = Math.max(cardRect.left + 12, 12);
                    const safeRight = Math.min(cardRect.right - 12, window.innerWidth - 12);
                    let shiftX = 0;
                    if (popoverRect.right > safeRight) shiftX -= popoverRect.right - safeRight;
                    if (popoverRect.left + shiftX < safeLeft) {
                        shiftX += safeLeft - (popoverRect.left + shiftX);
                    }
                    popover.style.setProperty('--spell-shift-x', shiftX + 'px');

                    const targetRect = target.getBoundingClientRect();
                    const shiftedRect = popover.getBoundingClientRect();
                    const safeTop = Math.max(cardRect.top + 12, 12);
                    const safeBottom = Math.min(cardRect.bottom - 12, window.innerHeight - 12);
                    if (
                        shiftedRect.bottom > safeBottom
                        && targetRect.top - shiftedRect.height - 14 >= safeTop
                    ) {
                        popover.classList.add('above');
                    }
                });
            };
            window.__khmerImePositionSpellPopover = () => {
                document.querySelectorAll('[data-testid="spell-popover"]').forEach(position);
            };

            const positionWithin = (node) => {
                if (!(node instanceof Element)) return;
                if (node.matches('[data-testid="spell-popover"]')) position(node);
                node.querySelectorAll?.('[data-testid="spell-popover"]').forEach(position);
            };

            new MutationObserver((records) => {
                records.forEach((record) => record.addedNodes.forEach(positionWithin));
            }).observe(document.body, { childList: true, subtree: true });

            window.addEventListener('resize', () => {
                document.querySelectorAll('[data-testid="spell-popover"]').forEach(position);
            }, { passive: true });
            document.querySelectorAll('[data-testid="spell-popover"]').forEach(position);
        })();
        "#,
    );
}

#[component]
fn SpellIssueSpan(
    index: usize,
    text: String,
    suggestions: Vec<String>,
    kind: SpellIssueKind,
    confidence_millis: Option<u16>,
    active: bool,
    open: bool,
    choice_index: usize,
    state: EditorSignals,
) -> Element {
    let chosen = suggestions.get(choice_index).cloned().unwrap_or_default();
    let class = match (kind, active) {
        (SpellIssueKind::Warning, true) => "spell-match warning active",
        (SpellIssueKind::Warning, false) => "spell-match warning",
        (SpellIssueKind::Error, true) => "spell-match active",
        (SpellIssueKind::Error, false) => "spell-match",
    };
    rsx! {
        span {
            id: "spell-issue-{index}",
            "data-testid": "spell-issue",
            class,
            "data-spell-kind": if kind == SpellIssueKind::Warning { "warning" } else { "error" },
            onclick: move |event| {
                event.stop_propagation();
                let mut review = state.spell_review();
                review.select(index, true);
                state.spell_review.set(review);
                schedule_spell_popover_placement();
            },
            "{text}"
            if open {
                span {
                    class: "spell-popover",
                    "data-testid": "spell-popover",
                    onclick: move |event| event.stop_propagation(),
                    if kind == SpellIssueKind::Warning {
                        span { class: "spell-popover-label", "ត្រូវការពិនិត្យ" }
                        span { class: "spell-warning-detail",
                            // A subscript ្ត or ្ដ in an otherwise-unknown word is very
                            // often the jeung-tor/jeung-dor confusion — hint at that.
                            if text.contains("្ត") || text.contains("្ដ") {
                                "ប្រហែលច្រឡំជើងតជាមួយជើងដ"
                            } else {
                                "ពាក្យនេះមិនមានក្នុងវចនានុក្រម (ប្រហែលជាឈ្មោះ ឬពាក្យថ្មី)"
                            }
                        }
                        if let Some(confidence) = confidence_millis {
                            span { class: "spell-confidence", "ទំនុកចិត្ត {confidence as f32 / 10.0:.1}%" }
                        }
                    } else {
                        span { class: "spell-popover-label", "ប្រហែលជា" }
                        span { class: "spell-primary",
                            span {
                                class: "spell-primary-word",
                                "data-testid": "spell-option",
                                "{chosen}"
                            }
                            button {
                                class: "spell-accept",
                                "data-testid": "spell-accept",
                                disabled: chosen.is_empty(),
                                onclick: move |event| {
                                    event.stop_propagation();
                                    let replacement = chosen.clone();
                                    spawn(async move {
                                        let current_text = state.text();
                                        let mut review = state.spell_review();
                                        if let Some((next_text, replacement_end)) =
                                            review.accept(index, &replacement, &current_text)
                                        {
                                            save_editor_text(&next_text);
                                            state.text.set(next_text);
                                            if review.issues.is_empty() {
                                                state.clear_spell_review();
                                            } else {
                                                state.spell_review.set(review);
                                            }
                                            // Land the caret right after the corrected word, WITHOUT
                                            // focusing/scrolling — the user is in the review popover
                                            // flow, so stealing focus makes the view jump. Via a
                                            // pending signal so it runs AFTER the value re-renders
                                            // (a synchronous set would be reset by the re-render).
                                            state.pending_caret_no_focus.set(Some(replacement_end));
                                            state.clear_candidate_state_and_picker();
                                        }
                                    });
                                },
                                "ប្ដូរ"
                            }
                        }
                    }
                    if kind == SpellIssueKind::Error && suggestions.len() > 1 {
                        span { class: "spell-alternatives-label", "ផ្សេងទៀត" }
                        span { class: "spell-alternatives",
                            for (suggestion_index, suggestion) in suggestions.iter().enumerate() {
                                if suggestion_index != choice_index {
                                    button {
                                        key: "spell-option-{suggestion_index}-{suggestion}",
                                        class: "spell-alternative",
                                        "data-testid": "spell-option",
                                        onclick: move |event| {
                                            event.stop_propagation();
                                            let mut review = state.spell_review();
                                            review.choose_suggestion(suggestion_index);
                                            state.spell_review.set(review);
                                        },
                                        "{suggestion}"
                                    }
                                }
                            }
                        }
                    }
                    button {
                        class: "spell-ignore",
                        "data-testid": "spell-ignore",
                        onclick: move |event| {
                            event.stop_propagation();
                            // Ignore by the issue's SOURCE word (what the checker
                            // flagged), not the rendered slice — the two can differ
                            // (e.g. a ZWSP in the document), and suppression matches
                            // on source. Dismiss every instance of it at once.
                            let review_now = state.spell_review();
                            let Some(word) = review_now.issues.get(index).map(|i| i.source.clone())
                            else { return; };
                            let mut ignore = state.spell_ignore();
                            ignore.insert(word.clone());
                            state.spell_ignore.set(ignore);
                            let mut review = state.spell_review();
                            review.ignore_word(&word);
                            if review.issues.is_empty() {
                                state.clear_spell_review();
                            } else {
                                state.spell_review.set(review);
                            }
                        },
                        "មិនអើពើ"
                    }
                }
            }
        }
    }
}

#[component]
fn SpellOverlay(text: String, review: SpellReview, font_size: usize, state: EditorSignals) -> Element {
    let issue_pieces = spell_pieces(&text, &review.issues);
    let segmentation_pieces = segment_pieces(&text, &review.segments);
    rsx! {
        div {
            class: "spell-overlay spell-segmentation-overlay editor-spell-reviewed",
            "data-testid": "spell-segmentation-overlay",
            aria_hidden: "true",
            style: "font-size: {font_size}px;",
            for piece in segmentation_pieces {
                match piece {
                    SegmentPiece::Plain(text) => rsx! { span { "{text}" } },
                    SegmentPiece::Segment { index, text } => rsx! {
                        span {
                            class: "spell-segment",
                            "data-testid": "spell-segment",
                            "data-segment-index": "{index}",
                            "{text}"
                        }
                    },
                }
            }
        }
        div {
            class: "spell-overlay spell-issue-overlay editor-spell-reviewed",
            "data-testid": "spell-overlay",
            aria_hidden: "true",
            style: "font-size: {font_size}px;",
            for piece in issue_pieces {
                match piece {
                    SpellPiece::Plain(text) => rsx! { span { class: "spell-issue-plain", "{text}" } },
                    SpellPiece::Issue { index, text } => {
                        let issue = &review.issues[index];
                        rsx! {
                            SpellIssueSpan {
                                index,
                                text,
                                suggestions: issue.suggestions.clone(),
                                kind: issue.kind,
                                confidence_millis: issue.confidence_millis,
                                active: review.active_index == index,
                                open: review.open_index == Some(index),
                                choice_index: review.choice_index,
                                state,
                            }
                        }
                    }
                }
            }
        }
    }
}

fn cycle_live_candidate(delta: isize, mut state: EditorSignals) -> bool {
    let len = if state.segmented_refine_mode() && state.segmented_session().is_some() {
        state
            .segmented_session()
            .map(|session| session.current_candidate_len())
            .unwrap_or(0)
    } else {
        state.suggestions().len()
    };
    if len == 0 {
        return false;
    }

    let next = if !state.selection_started() {
        if delta < 0 {
            len.saturating_sub(1)
        } else {
            0
        }
    } else if delta < 0 {
        (state.selected() + len - 1) % len
    } else {
        (state.selected() + 1) % len
    };

    let changed = if state.segmented_refine_mode() && state.segmented_session().is_some() {
        select_segment_candidate(next, state)
    } else {
        state.selected.set(next);
        state.selection_started.set(true);
        true
    };
    if changed {
        state.number_pick_mode.set(false);
    }
    changed
}

fn handle_horizontal_arrow(delta: isize, has_live_suggestions: bool, mut state: EditorSignals) -> bool {
    if state.candidate_level() == CandidateLevel::Phrase {
        // Phrase mode reserves horizontal arrows so they cannot move the
        // document caret into the active Roman composition (macOS ADR-0004).
        return true;
    }
    if state.candidate_level() == CandidateLevel::Segment {
        let _ = move_segment_focus(delta, state);
        state.number_pick_mode.set(false);
        // Consume even at the first/last segment; never turn the same arrow
        // into candidate cycling or leak it into the textarea.
        return true;
    }

    if has_live_suggestions && (state.selection_started() || state.number_pick_mode()) {
        return cycle_live_candidate(delta, state);
    }

    false
}

fn space_cycle_enabled(has_live_suggestions: bool, candidate_mode: CandidateMode) -> bool {
    has_live_suggestions && candidate_mode == CandidateMode::Transliteration
}

fn apply_shortcut_selection(
    key: &str,
    modifiers: Modifiers,
    live_suggestion_len: usize,
    mut state: EditorSignals,
) -> bool {
    if modifiers.contains(Modifiers::CONTROL)
        || modifiers.contains(Modifiers::ALT)
        || modifiers.contains(Modifiers::META)
    {
        return false;
    }

    let Some(offset) = shortcut_index(key) else {
        return false;
    };

    let page_start = visible_page_start(state.selected(), live_suggestion_len);
    let index = page_start + offset;
    if index >= live_suggestion_len {
        return false;
    }

    if state.segmented_refine_mode() && state.segmented_session().is_some() {
        select_segment_candidate(index, state);
    } else {
        state.selected.set(index);
        state.selection_started.set(true);
    }
    state.number_pick_mode.set(true);
    true
}

#[component]
pub(crate) fn EditorCard(state: EditorSignals, font_size: Signal<usize>) -> Element {
    use_effect(install_spell_popover_placement);
    let text_value = state.text();
    let suggestions = state.suggestions();
    let suggestion_total = suggestions.len();
    let page_start = visible_page_start(state.selected(), suggestion_total);
    let page_count = suggestion_total.saturating_add(VISIBLE_SUGGESTIONS - 1) / VISIBLE_SUGGESTIONS;
    let page_number = if page_count == 0 {
        0
    } else {
        page_start / VISIBLE_SUGGESTIONS + 1
    };
    let candidate_level = state.candidate_level();
    let phrase_candidates = state.phrase_candidates();
    let segment_session = if candidate_level == CandidateLevel::Segment {
        state.segmented_session()
    } else {
        None
    };
    let recommended_indices = state.recommended_indices();
    let roman_variant_hints = state.roman_variant_hints();
    let has_suggestions = !suggestions.is_empty();
    // Next-word predictions get their own docked bar below the editor (ADR:
    // docked prediction bar). They are pointer-only and never float at the caret.
    let is_next_word = state.candidate_mode() == CandidateMode::NextWord;
    let show_candidate_list = has_suggestions && !is_next_word;
    let next_word_predictions = if is_next_word { suggestions.clone() } else { Vec::new() };
    let spell_review = state.spell_review();
    let spell_results_visible = spell_review.result_bar_visible();
    let spell_has_issues = !spell_review.issues.is_empty();
    // Floating copy-text button: shown only when the document has text; flips to a
    // "copied" confirmation for a moment after a click.
    let mut copied = use_signal(|| false);
    // Clear button: two-tap confirm — first tap arms it ("បាទ?"), second clears the
    // document. Reverts after a moment if not confirmed, so a mis-tap loses nothing.
    let mut clear_armed = use_signal(|| false);
    let has_text = !text_value.trim().is_empty();
    let editor_class = [
        "editor",
        state
            .composition()
            .is_some()
            .then_some("editor-composing")
            .unwrap_or(""),
        spell_results_visible.then_some("editor-spell-reviewed").unwrap_or(""),
        spell_has_issues.then_some("editor-spell-active").unwrap_or(""),
    ]
    .join(" ");
    rsx! {
        div { class: "editor-card",
            div { class: "editor-wrap",
                textarea {
                    id: EDITOR_ID,
                    "data-testid": "editor-input",
                    class: "{editor_class}",
                    style: "font-size: {font_size()}px;",
                    value: "{text_value}",
                    placeholder: "ចាប់ផ្ដើមសរសេរនៅទីនេះ…",
                    spellcheck: "false",
                    autocomplete: "off",
                    autocorrect: "off",
                    onpointerdown: move |_| {
                        let mut review = state.spell_review();
                        if review.open_index.is_some() {
                            review.dismiss_interaction();
                            state.spell_review.set(review);
                        }
                    },
                    oninput: move |event| {
                        let value = event.value();
                        save_editor_text(&value);
                        state.text.set(value.clone());
                        state.clear_spell_review();
                        // Start fresh after text changes so the next ArrowDown selects the first
                        // candidate for the current token instead of continuing stale selection.
                        state.number_pick_mode.set(false);
                        state.selection_started.set(false);
                        state.selected.set(0);
                        spawn(update_candidates(value, state));
                    },
                    onkeydown: move |event| {
                        let key = event.key().to_string();
                        let modifiers = event.modifiers();

                        if modifiers.contains(Modifiers::ALT)
                            && modifiers.contains(Modifiers::CONTROL)
                            && key.eq_ignore_ascii_case("k")
                        {
                            event.prevent_default();
                            let next = !state.roman_enabled();
                            state.roman_enabled.set(next);
                            save_enabled(next);
                            if !next {
                                state.clear_candidate_state_and_picker();
                            } else {
                                spawn(update_candidates(state.text(), state));
                            }
                            return;
                        }

                        // Spell-review navigation: while a review with issues is
                        // active, ←/→ jump between flagged words (the popover
                        // follows). Editing/clearing the text ends the review, so
                        // arrows return to their normal caret/candidate role.
                        if spell_has_issues && (key == "ArrowLeft" || key == "ArrowRight") {
                            event.prevent_default();
                            let mut review = state.spell_review();
                            review.move_selection(if key == "ArrowLeft" { -1 } else { 1 });
                            state.spell_review.set(review);
                            schedule_spell_popover_placement();
                            return;
                        }

                        if !state.roman_enabled() {
                            return;
                        }

                        let live_suggestions = state.suggestions();
                        // Next-word predictions are pointer-only (tap the docked bar):
                        // they must not respond to Tab/Enter/Space, so Enter stays free
                        // for newlines. Only transliteration candidates are keyboard-driven.
                        let has_live_suggestions = !live_suggestions.is_empty()
                            && state.candidate_mode() != CandidateMode::NextWord;
                        let can_cycle_with_space = space_cycle_enabled(has_live_suggestions, state.candidate_mode());
                        let live_suggestion_len = live_suggestions.len();
                        let selection_lock_active = has_live_suggestions && state.number_pick_mode();

                        match key.as_str() {
                            "ArrowLeft" => {
                                if handle_horizontal_arrow(-1, has_live_suggestions, state) {
                                    event.prevent_default();
                                }
                            }
                            "ArrowRight" => {
                                if handle_horizontal_arrow(1, has_live_suggestions, state) {
                                    event.prevent_default();
                                }
                            }
                            "Tab" if candidate_level == CandidateLevel::Phrase => {
                                event.prevent_default();
                                let _ = enter_segment_edit(state.selected(), state);
                            }
                            "Tab" if candidate_level == CandidateLevel::Segment => {
                                event.prevent_default();
                                let _ = exit_segment_edit(state);
                            }
                            "Tab" if has_live_suggestions => {
                                event.prevent_default();
                                let len = live_suggestion_len;
                                let next = if !state.selection_started() {
                                    0
                                } else {
                                    (state.selected() + 1) % len
                                };
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            "ArrowDown" if has_live_suggestions => {
                                event.prevent_default();
                                if event.is_auto_repeating() {
                                    return;
                                }
                                if state.segmented_refine_mode() {
                                    let Some(session) = state.segmented_session() else {
                                        return;
                                    };

                                    let len = session.current_candidate_len();
                                    if len == 0 {
                                        return;
                                    }

                                    let next = if !state.selection_started() {
                                        0
                                    } else {
                                        (state.selected() + 1) % len
                                    };

                                    select_segment_candidate(next, state);
                                    state.selection_started.set(true);
                                } else {
                                    let len = live_suggestion_len;
                                    if len == 0 {
                                        return;
                                    }

                                    let next = if !state.selection_started() {
                                        0
                                    } else {
                                        (state.selected() + 1) % len
                                    };

                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }

                                state.number_pick_mode.set(false);
                                // if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                //     let next = if !state.selection_started() { 0 } else { (state.selected() + 1) % len };
                                //     select_segment_candidate(next, state);
                                // } else {
                                //     if !state.selection_started() {
                                //         state.selected.set(0);
                                //     } else {
                                //         state.selected.set((state.selected() + 1) % len);
                                //     }
                                //     state.selection_started.set(true);
                                // }
                                // state.number_pick_mode.set(false);
                            }
                            "ArrowUp" if has_live_suggestions => {
                                if event.is_auto_repeating() {
                                    return;
                                }
                                event.prevent_default();
                                let len = live_suggestion_len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    let next = if !state.selection_started() {
                                        len.saturating_sub(1)
                                    } else {
                                        (state.selected() + len - 1) % len
                                    };
                                    select_segment_candidate(next, state);
                                } else {
                                    if !state.selection_started() {
                                        state.selected.set(len.saturating_sub(1));
                                    } else {
                                        state.selected.set((state.selected() + len - 1) % len);
                                    }
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(false);
                            }
                            key
                                if is_space_key(key)
                                    && modifiers.contains(Modifiers::SHIFT)
                                    && can_cycle_with_space =>
                            {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            key if is_space_key(key) && can_cycle_with_space && !state.selection_started() => {
                                event.prevent_default();
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(0, state);
                                } else {
                                    state.selected.set(0);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            key if is_space_key(key) && can_cycle_with_space => {
                                event.prevent_default();
                                let len = live_suggestion_len;
                                let next = (state.selected() + 1) % len;
                                if state.segmented_refine_mode() && state.segmented_session().is_some() {
                                    select_segment_candidate(next, state);
                                } else {
                                    state.selected.set(next);
                                    state.selection_started.set(true);
                                }
                                state.number_pick_mode.set(true);
                            }
                            "Enter" if has_live_suggestions => {
                                event.prevent_default();
                                spawn(commit_active_selection(false, state));
                            }
                            key if has_live_suggestions
                                && apply_shortcut_selection(key, modifiers, live_suggestion_len, state) =>
                            {
                                event.prevent_default();
                                if candidate_level == CandidateLevel::Phrase {
                                    spawn(click_candidate(state.selected(), state));
                                }
                            }
                            key if selection_lock_active && has_live_suggestions => {
                                if should_exit_number_pick(key) {
                                    state.number_pick_mode.set(false);
                                    state.selection_started.set(false);
                                } else if key.chars().count() == 1
                                    && !modifiers.contains(Modifiers::CONTROL)
                                    && !modifiers.contains(Modifiers::ALT)
                                    && !modifiers.contains(Modifiers::META)
                                {
                                    // Keep selection lock: printable keys should not edit text while cycling.
                                    event.prevent_default();
                                }
                            }
                            _ => {}
                        }
                    },
                }
                if has_text {
                    div { class: "editor-actions",
                    button {
                        class: if clear_armed() { "editor-clear-button is-armed" } else { "editor-clear-button" },
                        "data-testid": "clear-text",
                        r#type: "button",
                        title: "លុបអត្ថបទទាំងអស់",
                        aria_label: "លុបអត្ថបទទាំងអស់",
                        onclick: move |event| {
                            event.stop_propagation();
                            if clear_armed() {
                                // second tap: clear the document
                                save_editor_text("");
                                state.text.set(String::new());
                                state.clear_spell_review();
                                state.clear_candidate_state_and_picker();
                                clear_armed.set(false);
                            } else {
                                // first tap: arm, auto-disarm if not confirmed
                                clear_armed.set(true);
                                spawn(async move {
                                    clear_confirm_delay().await;
                                    clear_armed.set(false);
                                });
                            }
                        },
                        if clear_armed() {
                            span { class: "editor-clear-icon", aria_hidden: "true", "✕" }
                            span { class: "editor-clear-label", "យល់ព្រម?" }
                        } else {
                            span { class: "editor-clear-icon", aria_hidden: "true", "✕" }
                            span { class: "editor-clear-label", "លុប" }
                        }
                    }
                    button {
                        class: if copied() { "editor-copy-button is-copied" } else { "editor-copy-button" },
                        "data-testid": "copy-text",
                        r#type: "button",
                        title: "ចម្លងអត្ថបទ",
                        aria_label: "ចម្លងអត្ថបទ",
                        onclick: move |event| {
                            event.stop_propagation();
                            let text = state.text();
                            spawn(async move {
                                if copy_to_clipboard(&text).await {
                                    copied.set(true);
                                    copied_reset_delay().await;
                                    copied.set(false);
                                }
                            });
                        },
                        if copied() {
                            span { class: "editor-copy-icon", aria_hidden: "true", "✓" }
                            span { class: "editor-copy-label", "បានចម្លង" }
                        } else {
                            span { class: "editor-copy-icon", aria_hidden: "true", "⧉" }
                            span { class: "editor-copy-label", "ចម្លង" }
                        }
                    }
                    }
                }
                if spell_has_issues {
                    SpellOverlay {
                        text: text_value.clone(),
                        review: spell_review.clone(),
                        font_size: font_size(),
                        state,
                    }
                }
                // Clean-check celebration: review completed with no issues. Shows a
                // brief centered 🎉 + message, then the flow auto-clears the review.
                if spell_results_visible && !spell_has_issues {
                    div { class: "spell-clean-celebration", aria_hidden: "true",
                        span { class: "spell-clean-emoji", "🎉" }
                        span { class: "spell-clean-text", "ម៉ាស៊ីនមិនអាចរកឃើញកំហុស" }
                    }
                }
                // Caret popup: transliteration candidates only. Next-word
                // predictions render in the docked bar below (not here).
                if show_candidate_list {
                    div {
                        class: if candidate_level == CandidateLevel::Flat {
                            "suggestion-popup suggestion-popup-compact"
                        } else if candidate_level == CandidateLevel::Segment {
                            "suggestion-popup suggestion-popup-segment"
                        } else {
                            "suggestion-popup"
                        },
                        "data-testid": "suggestion-popup",
                        "data-candidate-level": match candidate_level {
                            CandidateLevel::Flat => "flat",
                            CandidateLevel::Phrase => "phrase",
                            CandidateLevel::Segment => "segment",
                        },
                        style: popup_style(state.popup()),
                        if let Some(session) = &segment_session {
                            div { class: "candidate-context", "data-testid": "segment-edit-header",
                                button {
                                    class: "candidate-back",
                                    aria_label: "Return to phrase candidates",
                                    title: "Phrase candidates",
                                    onclick: move |_| { let _ = exit_segment_edit(state); },
                                    "‹"
                                }
                                div { class: "candidate-segments",
                                    for (segment_index, segment) in session.segments.iter().enumerate() {
                                        button {
                                            key: "segment-{segment_index}-{segment.input}",
                                            class: if segment_index == session.focused {
                                                "candidate-segment active"
                                            } else {
                                                "candidate-segment"
                                            },
                                            onclick: move |_| {
                                                let focused = state.segmented_session().map(|live| live.focused).unwrap_or(0);
                                                let delta = segment_index as isize - focused as isize;
                                                if delta != 0 { let _ = move_segment_focus(delta, state); }
                                            },
                                            span { class: "candidate-segment-output", "{segment.selected_text()}" }
                                            span { class: "candidate-segment-input", "{segment.input}" }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "candidate-track candidate-track-popup",
                            ul { class: "suggestion-list candidate-list",
                                for (index, item) in suggestions.iter()
                                    .enumerate()
                                    .skip(page_start)
                                    .take(VISIBLE_SUGGESTIONS) {
                                    li {
                                        key: "popup-{index}-{item}",
                                        class: if index == state.selected()
                                            && (state.selection_started() || candidate_level != CandidateLevel::Flat) {
                                            "suggestion active"
                                        } else {
                                            "suggestion"
                                        },
                                        button {
                                            class: "candidate-choice",
                                            onclick: move |_| {
                                                spawn(click_candidate(index, state));
                                            },
                                            span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                            span { class: "suggestion-main",
                                                span { class: "suggestion-word", "{item}" }
                                                if candidate_level == CandidateLevel::Flat {
                                                    if let Some(variants) = roman_variant_hints.get(&index) {
                                                        span { class: "suggestion-roman-hint", "{roman_hint_label(variants)}" }
                                                    } else {
                                                        span { class: "suggestion-roman-hint", "(derived)"}
                                                    }
                                                }
                                            }
                                            if candidate_level == CandidateLevel::Flat && recommended_indices.contains(&index) {
                                                span { class: "suggestion-recommended", "គួរប្រើ" }
                                            }
                                        }
                                        if candidate_level == CandidateLevel::Phrase
                                            && phrase_candidates.get(index).map(|phrase| phrase.segments.len() >= 2).unwrap_or(false) {
                                            button {
                                                class: "candidate-edit",
                                                aria_label: "Edit phrase segments",
                                                title: "Edit phrase segments",
                                                onclick: move |_| { let _ = enter_segment_edit(index, state); },
                                                "✎"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if page_count > 1 {
                            div { class: "candidate-page", "data-testid": "candidate-page", "{page_number}/{page_count}" }
                        }
                    }
                }
                // Docked next-word prediction bar: labeled strip under the editor,
                // always in the same place, tap-only (Enter stays free for newlines).
                if is_next_word && !next_word_predictions.is_empty() {
                    div { class: "next-word-dock", "data-testid": "next-word-dock",
                        span { class: "next-word-dock-label",
                            svg { class: "next-word-dock-arrow", view_box: "0 0 24 24", fill: "none",
                                stroke: "currentColor", stroke_width: "2", stroke_linecap: "round",
                                path { d: "M5 12h14" } path { d: "m12 5 7 7-7 7" }
                            }
                            "បន្ទាប់"
                        }
                        div { class: "next-word-dock-chips",
                            for (index, item) in next_word_predictions.iter().enumerate().take(VISIBLE_SUGGESTIONS) {
                                button {
                                    key: "nextword-{index}-{item}",
                                    class: if index == 0 { "next-word-chip next-word-chip-top" } else { "next-word-chip" },
                                    onclick: move |_| { spawn(click_candidate(index, state)); },
                                    span { class: "kh", "{item}" }
                                }
                            }
                        }
                        span { class: "next-word-dock-hint", "ប៉ះដើម្បីបញ្ចូល" }
                    }
                }
                div {
                    class: if show_candidate_list { "candidate-bar" } else { "candidate-bar candidate-bar-empty" },
                    div { class: "candidate-track candidate-track-mobile",
                        if show_candidate_list {
                            ul { class: "suggestion-list candidate-list",
                                for (index, item) in suggestions.iter()
                                    .enumerate()
                                    .skip(page_start)
                                    .take(VISIBLE_SUGGESTIONS) {
                                    li {
                                        key: "mobile-{index}-{item}",
                                        class: if state.selection_started() && index == state.selected() { "suggestion active" } else { "suggestion" },
                                        button {
                                            onclick: move |_| {
                                                spawn(click_candidate(index, state));
                                            },
                                            span { class: "suggestion-rank", "{shortcut_label(index)}" }
                                            span { class: "suggestion-main",
                                                span { class: "suggestion-word", "{item}" }
                                                if let Some(variants) = roman_variant_hints.get(&index) {
                                                    span { class: "suggestion-roman-hint", "{roman_hint_label(variants)}" }
                                                } else {
                                                    span { class: "suggestion-roman-hint", "(derived)"}
                                                }
                                            }
                                            if recommended_indices.contains(&index) {
                                                span { class: "suggestion-recommended", "គួរប្រើ" }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "candidate-empty", aria_hidden: "true",
                                span { class: "segment-placeholder-chip segment-placeholder-chip-1" }
                                span { class: "segment-placeholder-chip segment-placeholder-chip-2" }
                                span { class: "segment-placeholder-chip segment-placeholder-chip-3" }
                            }
                        }
                    }
                    div { class: "candidate-footer",
                        div { class: "candidate-hints desktop-candidate-hints", "data-testid": "candidate-hints",
                            if is_next_word {
                                span { class: "candidate-hint",
                                    span { class: "editor-tip-text", "ប៉ះពាក្យដើម្បីបញ្ចូល" }
                                    span { class: "editor-tip-sep", "·" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "ចុះបន្ទាត់" }
                                }
                            } else if candidate_level == CandidateLevel::Phrase {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Space / ↑↓" }
                                    span { class: "editor-tip-text", "ប្ដូរឃ្លា" }
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "កែពាក្យ" }
                                    span { class: "keycap", "1–5 / Enter" }
                                    span { class: "editor-tip-text", "បញ្ចូល" }
                                }
                            } else if candidate_level == CandidateLevel::Segment {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "← →" }
                                    span { class: "editor-tip-text", "ប្ដូរពាក្យ" }
                                    span { class: "keycap", "Space / ↑↓" }
                                    span { class: "editor-tip-text", "ប្ដូរជម្រើស" }
                                    span { class: "keycap", "Tab" }
                                    span { class: "editor-tip-text", "ត្រឡប់" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "បញ្ចូលឃ្លា" }
                                }
                            } else if show_candidate_list {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Space" }
                                    span { class: "editor-tip-text", "ប្ដូរជម្រើស" }
                                    span { class: "keycap", "1–5" }
                                    span { class: "editor-tip-text", "ជ្រើស" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "បញ្ចូល" }
                                }
                            } else {
                                span { class: "candidate-hint",
                                    span { class: "keycap", "Ctrl+Alt+K" }
                                    span { class: "editor-tip-text", "បើក/បិទការបម្លែង" }
                                    span { class: "editor-tip-sep", "·" }
                                    span { class: "keycap", "Enter" }
                                    span { class: "editor-tip-text", "ចុះបន្ទាត់" }
                                }
                            }
                        }
                        div { class: "mobile-candidate-footer",
                            div { class: "mobile-caret-controls",
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-caret-left",
                                    aria_label: "Arrow-left behavior",
                                    onclick: move |_| {
                                        if !handle_horizontal_arrow(-1, show_candidate_list, state) {
                                            spawn(async move {
                                                let _ = move_editor_caret(-1).await;
                                            });
                                        }
                                    },
                                    "←"
                                }
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-caret-right",
                                    aria_label: "Arrow-right behavior",
                                    onclick: move |_| {
                                        if !handle_horizontal_arrow(1, show_candidate_list, state) {
                                            spawn(async move {
                                                let _ = move_editor_caret(1).await;
                                            });
                                        }
                                    },
                                    "→"
                                }
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-select-up",
                                    aria_label: "Select previous suggestion",
                                    disabled: !show_candidate_list,
                                    onclick: move |_| {
                                        let _ = cycle_live_candidate(-1, state);
                                    },
                                    "↑"
                                }
                                button {
                                    class: "mobile-caret-btn",
                                    "data-testid": "mobile-select-down",
                                    aria_label: "Select next suggestion",
                                    disabled: !show_candidate_list,
                                    onclick: move |_| {
                                        let _ = cycle_live_candidate(1, state);
                                    },
                                    "↓"
                                }
                            }
                            div { class: "mobile-candidate-hints", "data-testid": "mobile-candidate-hints",
                                if state.suggestion_loading() {
                                    span { class: "candidate-hint-loading", "កំពុងរៀបចំ…" }
                                }
                                if is_next_word {
                                    span { class: "editor-tip-text", "ប៉ះដើម្បីបញ្ចូល · Enter ចុះបន្ទាត់" }
                                } else if candidate_level == CandidateLevel::Phrase {
                                    span { class: "editor-tip-text", "ប៉ះឃ្លាដើម្បីបញ្ចូល · ប៉ះ ✎ ដើម្បីកែពាក្យ" }
                                } else if candidate_level == CandidateLevel::Segment {
                                    span { class: "editor-tip-text", "ប៉ះពាក្យដើម្បីប្ដូរ · ប៉ះ ‹ ដើម្បីត្រឡប់" }
                                } else if show_candidate_list {
                                    span { class: "keycap", "↑↓" }
                                    span { class: "editor-tip-text", "ជ្រើស" }
                                    span { class: "keycap", "Space" }
                                    span { class: "editor-tip-text", "ប្ដូរ" }
                                    span { class: "keycap", "1–5" }
                                    span { class: "editor-tip-text", "បញ្ចូល" }
                                } else {
                                    span { class: "editor-tip-text", "ប៉ះផ្ទៃសរសេរ ហើយចាប់ផ្ដើមវាយ" }
                                }
                            }
                        }
                    }
                }
                if let Some(mark) = state.composition() {
                    div {
                        class: "composition-mark",
                        style: composition_style(&mark, false),
                    }
                }
            }
        }
    }
}

/// How long the copy button shows its "បានចម្លង" confirmation before reverting.
#[cfg(target_arch = "wasm32")]
async fn copied_reset_delay() {
    gloo_timers::future::TimeoutFuture::new(1_500).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn copied_reset_delay() {
    tokio::time::sleep(std::time::Duration::from_millis(1_500)).await;
}

/// How long the clear button stays armed ("បាទ?") awaiting a confirming second tap.
#[cfg(target_arch = "wasm32")]
async fn clear_confirm_delay() {
    gloo_timers::future::TimeoutFuture::new(2_500).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn clear_confirm_delay() {
    tokio::time::sleep(std::time::Duration::from_millis(2_500)).await;
}
