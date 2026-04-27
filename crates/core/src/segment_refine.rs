//! Segment refinement for long roman phrases.
//!
//! The decoder can identify multiple roman chunks inside a long token. This
//! module turns those observations into a focused editing session so UI/native
//! adapters can move between segments and cycle candidates without reimplementing
//! phrase segmentation.

use std::collections::{HashMap, HashSet};

use crate::ShadowObservation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentedChoice {
    /// Roman input for this segment.
    pub input: String,
    /// Character start offset inside the normalized raw input.
    pub start: usize,
    /// Character end offset inside the normalized raw input.
    pub end: usize,
    /// Visible candidates for the segment, already normalized for display.
    pub candidates: Vec<String>,
    /// Selected candidate index within `candidates`.
    pub selected: usize,
}

impl SegmentedChoice {
    pub fn selected_text(&self) -> String {
        self.candidates
            .get(self.selected)
            .cloned()
            .or_else(|| self.candidates.first().cloned())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentedSession {
    /// Original roman phrase being refined.
    pub raw_input: String,
    /// Editable segment choices, in phrase order.
    pub segments: Vec<SegmentedChoice>,
    /// Segment index that currently receives navigation/candidate commands.
    pub focused: usize,
}

impl SegmentedSession {
    pub fn focused_candidates(&self) -> Vec<String> {
        self.segments
            .get(self.focused)
            .map(|segment| segment.candidates.clone())
            .unwrap_or_default()
    }

    pub fn current_candidate_len(&self) -> usize {
        self.segments
            .get(self.focused)
            .map(|segment| segment.candidates.len())
            .unwrap_or(0)
    }

    pub fn focused_selected(&self) -> usize {
        self.segments
            .get(self.focused)
            .map(|segment| segment.selected)
            .unwrap_or(0)
    }

    pub fn composed_text(&self) -> String {
        self.segments
            .iter()
            .map(SegmentedChoice::selected_text)
            .collect::<String>()
    }
}

pub fn move_session_focus(session: &mut SegmentedSession, delta: isize) -> bool {
    if session.segments.len() <= 1 {
        return false;
    }
    let len = session.segments.len() as isize;
    let next = (session.focused as isize + delta).clamp(0, len - 1) as usize;
    if next == session.focused {
        return false;
    }
    session.focused = next;
    true
}

pub fn normalized_suggestion_key(item: &str) -> String {
    item.chars().filter(|ch| !ch.is_whitespace()).collect()
}

pub fn connect_khmer_display(item: &str) -> String {
    let parts = item.split_whitespace().collect::<Vec<_>>();
    if parts.len() <= 1 {
        return item.to_owned();
    }
    if parts.iter().all(|part| part.chars().any(is_khmer_char)) {
        parts.concat()
    } else {
        item.to_owned()
    }
}

pub fn normalize_visible_suggestions(items: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut seen = HashSet::<String>::new();

    for item in items {
        let display = connect_khmer_display(&item);
        let key = normalized_suggestion_key(&display);
        if seen.insert(key) {
            normalized.push(display);
        }
    }

    normalized
}

pub fn build_segmented_session<S>(
    observation: &ShadowObservation,
    raw_input: &str,
    history: &HashMap<String, usize>,
    suggest: &S,
) -> Option<SegmentedSession>
where
    S: Fn(&str, &HashMap<String, usize>) -> Vec<String>,
{
    build_segmented_session_from_pairs(raw_input, observation_segment_pairs(observation), history, 0, suggest)
}

pub fn reflow_segmented_session_from_selection<S, B, O>(
    session: &SegmentedSession,
    history: &HashMap<String, usize>,
    suggest: &S,
    best_prefix_consumption: &B,
    shadow_observation: &O,
) -> Option<SegmentedSession>
where
    S: Fn(&str, &HashMap<String, usize>) -> Vec<String>,
    B: Fn(&str, &str) -> Option<String>,
    O: Fn(&str, &HashMap<String, usize>) -> ShadowObservation,
{
    let focused = session.focused;
    let segment = session.segments.get(focused)?;
    let chosen = segment.selected_text();
    let tail = slice_chars(&session.raw_input, segment.start..char_len(&session.raw_input));
    let consumed = best_prefix_consumption(&tail, &chosen)?;
    let consumed_len = char_len(&consumed);
    if consumed_len == 0 || consumed_len == segment.end.saturating_sub(segment.start) {
        return None;
    }

    let mut segments = session.segments[..focused].to_vec();
    segments.push(build_segment_choice(
        consumed.clone(),
        Some(chosen),
        segment.start,
        history,
        suggest,
    ));

    let tail_start = segment.start + consumed_len;
    let total_len = char_len(&session.raw_input);
    if tail_start < total_len {
        let remaining_tail = slice_chars(&session.raw_input, tail_start..total_len);
        let observation = shadow_observation(&remaining_tail, history);
        if let Some(mut tail_session) = build_segmented_session_from_pairs(
            &session.raw_input,
            observation_segment_pairs(&observation),
            history,
            tail_start,
            suggest,
        ) {
            segments.append(&mut tail_session.segments);
        } else {
            segments.push(build_segment_choice(remaining_tail, None, tail_start, history, suggest));
        }
    }

    let focused = focused.min(segments.len().saturating_sub(1));
    Some(SegmentedSession {
        raw_input: session.raw_input.clone(),
        segments,
        focused,
    })
}

fn build_segmented_session_from_pairs<S>(
    raw_input: &str,
    pairs: Vec<(String, String)>,
    history: &HashMap<String, usize>,
    base_offset: usize,
    suggest: &S,
) -> Option<SegmentedSession>
where
    S: Fn(&str, &HashMap<String, usize>) -> Vec<String>,
{
    if pairs.len() < 2 {
        return None;
    }

    let mut cursor = base_offset;
    let segments = pairs
        .into_iter()
        .map(|(input, output)| {
            let start = cursor;
            cursor += char_len(&input);
            build_segment_choice(input, Some(output), start, history, suggest)
        })
        .collect::<Vec<_>>();

    Some(SegmentedSession {
        raw_input: raw_input.to_owned(),
        segments,
        focused: 0,
    })
}

fn build_segment_choice<S>(
    input: String,
    output: Option<String>,
    start: usize,
    history: &HashMap<String, usize>,
    suggest: &S,
) -> SegmentedChoice
where
    S: Fn(&str, &HashMap<String, usize>) -> Vec<String>,
{
    let mut candidates = normalize_visible_suggestions(suggest(&input, history));
    if let Some(output) = output.map(|item| connect_khmer_display(&item)) {
        if let Some(position) = candidates.iter().position(|candidate| candidate == &output) {
            if position != 0 {
                let preferred = candidates.remove(position);
                candidates.insert(0, preferred);
            }
        } else {
            candidates.insert(0, output);
        }
    } else if candidates.is_empty() {
        candidates.push(input.clone());
    }
    candidates.truncate(10);

    SegmentedChoice {
        end: start + char_len(&input),
        input,
        start,
        candidates,
        selected: 0,
    }
}

fn observation_segment_pairs(observation: &ShadowObservation) -> Vec<(String, String)> {
    if !observation.wfst_top_segment_details.is_empty() {
        observation
            .wfst_top_segment_details
            .iter()
            .map(|segment| (segment.input.clone(), segment.output.clone()))
            .collect::<Vec<_>>()
    } else {
        observation
            .wfst_top_segments
            .iter()
            .filter_map(|segment| segment.split_once("=>"))
            .map(|(input, output)| (input.to_owned(), output.to_owned()))
            .collect::<Vec<_>>()
    }
}

fn char_len(input: &str) -> usize {
    input.chars().count()
}

fn slice_chars(input: &str, range: std::ops::Range<usize>) -> String {
    input
        .chars()
        .skip(range.start)
        .take(range.end.saturating_sub(range.start))
        .collect()
}

fn is_khmer_char(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch) || ('\u{19e0}'..='\u{19ff}').contains(&ch)
}

#[cfg(test)]
mod tests {
    use super::{
        build_segmented_session, normalize_visible_suggestions, normalized_suggestion_key,
        reflow_segmented_session_from_selection, SegmentedChoice, SegmentedSession,
    };
    use crate::{DecodeSegment, DecoderMode, ShadowMismatch, ShadowObservation};
    use std::collections::HashMap;

    fn sample_observation() -> ShadowObservation {
        ShadowObservation {
            mode: DecoderMode::Shadow,
            input: "khnhomtov".to_owned(),
            mismatch: ShadowMismatch::OutputMismatch,
            composer_chunks: vec!["khnhom".to_owned(), "t".to_owned(), "ov".to_owned()],
            composer_hint_chunks: vec!["tov".to_owned()],
            composer_pending_tail: String::new(),
            composer_fully_segmented: true,
            wfst_used_hint_chunks: true,
            wfst_top_segment_details: vec![
                DecodeSegment {
                    input: "khnhom".to_owned(),
                    output: "ខ្ញុំ".to_owned(),
                    weight_bps: 9500,
                },
                DecodeSegment {
                    input: "tov".to_owned(),
                    output: "ទៅ".to_owned(),
                    weight_bps: 9100,
                },
            ],
            wfst_top_segments: vec!["khnhom=>ខ្ញុំ".to_owned(), "tov=>ទៅ".to_owned()],
            legacy_latency_us: 10,
            wfst_latency_us: Some(8),
            legacy_failure: None,
            wfst_failure: None,
            legacy_top: Some("ខ្ញុំ ទៅ".to_owned()),
            wfst_top: Some("ខ្ញុំទៅ".to_owned()),
            legacy_top5: vec!["ខ្ញុំ ទៅ".to_owned()],
            wfst_top5: vec!["ខ្ញុំទៅ".to_owned()],
            legacy_top_in_wfst: false,
            wfst_top_in_legacy: false,
        }
    }

    #[test]
    fn normalizes_visible_suggestions_by_khmer_display_key() {
        let normalized = normalize_visible_suggestions(vec![
            "ខ្ញុំ ទៅ".to_owned(),
            "ខ្ញុំទៅ".to_owned(),
            "foo bar".to_owned(),
            "foo bar".to_owned(),
        ]);
        assert_eq!(normalized, vec!["ខ្ញុំទៅ".to_owned(), "foo bar".to_owned()]);
        assert_eq!(normalized_suggestion_key("ខ្ញុំ ទៅ"), "ខ្ញុំទៅ");
    }

    #[test]
    fn builds_segmented_session_from_observation() {
        let suggest = |input: &str, _history: &HashMap<String, usize>| -> Vec<String> {
            match input {
                "khnhom" => vec!["ខ្ញុំ".to_owned()],
                "tov" => vec!["ទៅ".to_owned()],
                _ => Vec::new(),
            }
        };
        let session = build_segmented_session(&sample_observation(), "khnhomtov", &HashMap::new(), &suggest)
            .expect("segmented session");
        assert_eq!(session.segments.len(), 2);
        assert_eq!(session.segments[0].selected_text(), "ខ្ញុំ");
        assert_eq!(session.segments[1].selected_text(), "ទៅ");
    }

    #[test]
    fn reflows_suffix_when_prefix_consumption_changes() {
        let suggest = |input: &str, _history: &HashMap<String, usize>| -> Vec<String> {
            match input {
                "cheam" => vec!["ជា".to_owned()],
                "chea" => vec!["ជា".to_owned()],
                "ous" => vec!["អូស".to_owned()],
                "laor" => vec!["ល្អ".to_owned()],
                "mnous" => vec!["មនុស្ស".to_owned()],
                _ => vec![input.to_owned()],
            }
        };
        let session = SegmentedSession {
            raw_input: "cheamnouslaor".to_owned(),
            segments: vec![
                SegmentedChoice {
                    input: "cheam".to_owned(),
                    start: 0,
                    end: 5,
                    candidates: vec!["ជា".to_owned()],
                    selected: 0,
                },
                SegmentedChoice {
                    input: "ous".to_owned(),
                    start: 5,
                    end: 8,
                    candidates: vec!["អូស".to_owned()],
                    selected: 0,
                },
                SegmentedChoice {
                    input: "laor".to_owned(),
                    start: 8,
                    end: 12,
                    candidates: vec!["ល្អ".to_owned()],
                    selected: 0,
                },
            ],
            focused: 0,
        };
        let best_prefix = |tail: &str, _target: &str| -> Option<String> {
            if tail.starts_with("cheamnouslaor") {
                Some("chea".to_owned())
            } else {
                None
            }
        };
        let obs = |tail: &str, _history: &HashMap<String, usize>| -> ShadowObservation {
            ShadowObservation {
                mode: DecoderMode::Shadow,
                input: tail.to_owned(),
                mismatch: ShadowMismatch::OutputMismatch,
                composer_chunks: vec!["mnous".to_owned(), "laor".to_owned()],
                composer_hint_chunks: Vec::new(),
                composer_pending_tail: String::new(),
                composer_fully_segmented: true,
                wfst_used_hint_chunks: false,
                wfst_top_segment_details: vec![
                    DecodeSegment {
                        input: "mnous".to_owned(),
                        output: "មនុស្ស".to_owned(),
                        weight_bps: 9000,
                    },
                    DecodeSegment {
                        input: "laor".to_owned(),
                        output: "ល្អ".to_owned(),
                        weight_bps: 8900,
                    },
                ],
                wfst_top_segments: Vec::new(),
                legacy_latency_us: 0,
                wfst_latency_us: Some(0),
                legacy_failure: None,
                wfst_failure: None,
                legacy_top: None,
                wfst_top: None,
                legacy_top5: Vec::new(),
                wfst_top5: Vec::new(),
                legacy_top_in_wfst: false,
                wfst_top_in_legacy: false,
            }
        };
        let reflowed = reflow_segmented_session_from_selection(&session, &HashMap::new(), &suggest, &best_prefix, &obs)
            .expect("reflowed");
        assert_eq!(reflowed.segments[0].input, "chea");
        assert_eq!(reflowed.segments[1].input, "mnous");
    }
}
